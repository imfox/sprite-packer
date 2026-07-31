use image::{DynamicImage, GenericImageView, RgbaImage};

/// How an individual sprite is placed in the atlas.
#[derive(Debug, Clone)]
pub struct RenderItem {
    /// The source image (may be rotated/cloned during rendering)
    pub image: DynamicImage,
    /// Destination x in atlas
    pub dx: i32,
    /// Destination y in atlas
    pub dy: i32,
    /// Source x in image
    pub sx: i32,
    /// Source y in image
    pub sy: i32,
    /// Source width to blit
    pub sw: i32,
    /// Source height to blit
    pub sh: i32,
    /// Original image width (before rotation)
    pub ow: i32,
    /// Original image height (before rotation)
    pub oh: i32,
    /// Whether this sprite should be skipped (identical clone)
    pub skip_render: bool,
    /// Whether the sprite is rotated
    pub rotated: bool,
    /// Extrude pixels
    pub extrude: i32,
}

/// Result of rendering a sheet.
pub struct RenderResult {
    pub image: RgbaImage,
    pub width: u32,
    pub height: u32,
}

/// Options for texture rendering.
pub struct RenderOptions {
    pub fixed_size: bool,
    pub width: u32,
    pub height: u32,
    pub power_of_two: bool,
    pub padding: u32,
    pub extrude: u32,
    pub scale: f32,
}

/// Texture renderer — creates the atlas image by blitting all sprites.
/// Mirrors utils/TextureRenderer.js.
pub struct TextureRenderer;

impl TextureRenderer {
    /// Compute actual atlas dimensions from max placed rect coordinates.
    pub fn get_size(data: &[RenderItem], options: &RenderOptions) -> (u32, u32) {
        if options.fixed_size {
            return (options.width, options.height);
        }

        let mut width = 0u32;
        let mut height = 0u32;

        for item in data {
            let w = if item.rotated {
                (item.dx + item.sh) as u32
            } else {
                (item.dx + item.sw) as u32
            };
            let h = if item.rotated {
                (item.dy + item.sw) as u32
            } else {
                (item.dy + item.sh) as u32
            };

            if w > width {
                width = w;
            }
            if h > height {
                height = h;
            }
        }

        width += options.padding + options.extrude;
        height += options.padding + options.extrude;

        if options.power_of_two {
            width = next_pow2(width);
            height = next_pow2(height);
        }

        (width.max(1), height.max(1))
    }

    /// Render all items into an atlas image.
    pub fn render(data: &[RenderItem], options: &RenderOptions) -> RenderResult {
        let (width, height) = Self::get_size(data, options);
        let mut atlas = RgbaImage::new(width, height);

        for item in data {
            if item.skip_render {
                continue;
            }

            let mut img = item.image.clone();
            let mut sx = item.sx;
            let mut sy = item.sy;
            let mut sw = item.sw;
            let mut sh = item.sh;
            let oh = item.oh;
            let _ow = item.ow;

            let dx = item.dx;
            let dy = item.dy;

            // Handle rotation
            if item.rotated {
                img = img.rotate90(); // rotate 90° CCW
                // Adjust source coords for rotated image
                let new_sx = oh - sh - sy;
                let new_sy = sx;
                sx = new_sx;
                sy = new_sy;
                let tmp = sw;
                sw = sh;
                sh = tmp;
            }

            // Handle extrude
            if item.extrude > 0 && !item.skip_render {
                Self::apply_extrude(&mut atlas, &img, dx, dy, sx, sy, sw, sh, item.ow, item.oh, item.extrude);
            }

            // Blit the sprite onto the atlas
            for y in 0..sh {
                for x in 0..sw {
                    let px = (dx + x) as u32;
                    let py = (dy + y) as u32;
                    if px < width && py < height {
                        let src_x = (sx + x) as u32;
                        let src_y = (sy + y) as u32;
                        if src_x < img.width() && src_y < img.height() {
                            atlas.put_pixel(px, py, img.get_pixel(src_x, src_y));
                        }
                    }
                }
            }
        }

        // Apply scale if needed
        if options.scale != 1.0 && options.scale > 0.0 {
            let new_w = (width as f32 * options.scale).round() as u32;
            let new_h = (height as f32 * options.scale).round() as u32;
            let scaled = image::imageops::resize(
                &atlas,
                new_w.max(1),
                new_h.max(1),
                image::imageops::FilterType::CatmullRom,
            );
            return RenderResult {
                image: scaled,
                width: new_w.max(1),
                height: new_h.max(1),
            };
        }

        RenderResult {
            image: atlas,
            width,
            height,
        }
    }

    /// Apply edge extrusion (duplicate edge pixels outward).
    /// Mirrors the extrude logic in TextureRenderer.renderItem().
    fn apply_extrude(
        atlas: &mut RgbaImage,
        img: &DynamicImage,
        dx: i32,
        dy: i32,
        sx: i32,
        sy: i32,
        sw: i32,
        sh: i32,
        _ow: i32,
        _oh: i32,
        extrude: i32,
    ) {
        let aw = atlas.width() as i32;
        let ah = atlas.height() as i32;

        let get_src = |x: u32, y: u32| {
            if x < img.width() && y < img.height() {
                img.get_pixel(x, y)
            } else {
                image::Rgba([0, 0, 0, 0])
            }
        };

        let put = |atlas: &mut RgbaImage, x: i32, y: i32, p: image::Rgba<u8>| {
            if x >= 0 && x < aw && y >= 0 && y < ah {
                atlas.put_pixel(x as u32, y as u32, p);
            }
        };

        if extrude <= 0 {
            return;
        }

        let e = extrude;

        // Corners
        let c = |x: u32, y: u32| get_src(x, y);
        for ey in 0..e {
            for ex in 0..e {
                put(atlas, dx - e + ex, dy - e + ey, c(sx as u32, sy as u32));
                put(atlas, dx + sw + ex, dy - e + ey, c((sx + sw - 1) as u32, sy as u32));
                put(atlas, dx - e + ex, dy + sh + ey, c(sx as u32, (sy + sh - 1) as u32));
                put(atlas, dx + sw + ex, dy + sh + ey, c((sx + sw - 1) as u32, (sy + sh - 1) as u32));
            }
        }

        // Top/bottom edges
        for ex in 0..sw {
            for ey in 0..e {
                put(atlas, dx + ex, dy - e + ey, get_src((sx + ex) as u32, sy as u32));
                put(atlas, dx + ex, dy + sh + ey, get_src((sx + ex) as u32, (sy + sh - 1) as u32));
            }
        }

        // Left/right edges
        for ey in 0..sh {
            for ex in 0..e {
                put(atlas, dx - e + ex, dy + ey, get_src(sx as u32, (sy + ey) as u32));
                put(atlas, dx + sw + ex, dy + ey, get_src((sx + sw - 1) as u32, (sy + ey) as u32));
            }
        }
    }
}

fn next_pow2(v: u32) -> u32 {
    let mut v = v - 1;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v |= v >> 8;
    v |= v >> 16;
    v + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_pow2() {
        assert_eq!(next_pow2(1), 1);
        assert_eq!(next_pow2(2), 2);
        assert_eq!(next_pow2(3), 4);
        assert_eq!(next_pow2(255), 256);
        assert_eq!(next_pow2(256), 256);
        assert_eq!(next_pow2(257), 512);
    }

    #[test]
    fn test_render_simple() {
        let img = DynamicImage::new_rgba8(10, 10);
        let items = vec![RenderItem {
            image: img,
            dx: 0,
            dy: 0,
            sx: 0,
            sy: 0,
            sw: 10,
            sh: 10,
            ow: 10,
            oh: 10,
            skip_render: false,
            rotated: false,
            extrude: 0,
        }];
        let opts = RenderOptions {
            fixed_size: false,
            width: 0,
            height: 0,
            power_of_two: false,
            padding: 0,
            extrude: 0,
            scale: 1.0,
        };
        let result = TextureRenderer::render(&items, &opts);
        assert_eq!(result.width, 10);
        assert_eq!(result.height, 10);
    }

    #[test]
    fn test_power_of_two() {
        let img = DynamicImage::new_rgba8(10, 10);
        let items = vec![RenderItem {
            image: img,
            dx: 0,
            dy: 0,
            sx: 0,
            sy: 0,
            sw: 10,
            sh: 10,
            ow: 10,
            oh: 10,
            skip_render: false,
            rotated: false,
            extrude: 0,
        }];
        let opts = RenderOptions {
            fixed_size: false,
            width: 0,
            height: 0,
            power_of_two: true,
            padding: 0,
            extrude: 0,
            scale: 1.0,
        };
        let result = TextureRenderer::render(&items, &opts);
        assert_eq!(result.width, 16);
        assert_eq!(result.height, 16);
    }
}
