use image::RgbaImage;

/// Alpha-based transparent border trimming.
/// Mirrors utils/Trimmer.js from free-tex-packer-core.
pub struct Trimmer;

#[derive(Debug, Clone)]
pub struct TrimResult {
    pub trimmed: bool,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Trimmer {
    /// Trim transparent borders from a single image.
    pub fn trim_image(img: &RgbaImage, alpha_threshold: u8) -> TrimResult {
        let w = img.width();
        let h = img.height();

        let left = Self::get_left_space(img, alpha_threshold);
        // If fully transparent
        if left == w {
            return TrimResult {
                trimmed: true,
                x: 0,
                y: 0,
                w: 1,
                h: 1,
            };
        }

        let right = Self::get_right_space(img, alpha_threshold);
        let top = Self::get_top_space(img, alpha_threshold);
        let bottom = Self::get_bottom_space(img, alpha_threshold);

        let trimmed = left > 0 || right > 0 || top > 0 || bottom > 0;

        TrimResult {
            trimmed,
            x: left,
            y: top,
            w: w - left - right,
            h: h - top - bottom,
        }
    }

    /// Scan left-to-right, find first column with alpha > threshold
    fn get_left_space(img: &RgbaImage, threshold: u8) -> u32 {
        for x in 0..img.width() {
            for y in 0..img.height() {
                let p = img.get_pixel(x, y);
                if p[3] > threshold {
                    return x;
                }
            }
        }
        img.width()
    }

    /// Scan right-to-left
    fn get_right_space(img: &RgbaImage, threshold: u8) -> u32 {
        for x in (0..img.width()).rev() {
            for y in 0..img.height() {
                let p = img.get_pixel(x, y);
                if p[3] > threshold {
                    return img.width() - x - 1;
                }
            }
        }
        0
    }

    /// Scan top-to-bottom
    fn get_top_space(img: &RgbaImage, threshold: u8) -> u32 {
        for y in 0..img.height() {
            for x in 0..img.width() {
                let p = img.get_pixel(x, y);
                if p[3] > threshold {
                    return y;
                }
            }
        }
        0
    }

    /// Scan bottom-to-top
    fn get_bottom_space(img: &RgbaImage, threshold: u8) -> u32 {
        for y in (0..img.height()).rev() {
            for x in 0..img.width() {
                let p = img.get_pixel(x, y);
                if p[3] > threshold {
                    return img.height() - y - 1;
                }
            }
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn make_img(data: &[u8], w: u32, h: u32) -> RgbaImage {
        let mut img = RgbaImage::new(w, h);
        for (i, &a) in data.iter().enumerate() {
            let x = i as u32 % w;
            let y = i as u32 / w;
            img.put_pixel(x, y, Rgba([255, 255, 255, a]));
        }
        img
    }

    #[test]
    fn test_no_trim_needed() {
        let img = make_img(&[255, 255, 255, 255], 2, 2);
        let r = Trimmer::trim_image(&img, 0);
        assert!(!r.trimmed);
    }

    #[test]
    fn test_trim_transparent_border() {
        // 4x4 image with only center 2x2 opaque
        let mut img = RgbaImage::new(4, 4);
        for y in 0..4 {
            for x in 0..4 {
                let a = if x >= 1 && x < 3 && y >= 1 && y < 3 { 255 } else { 0 };
                img.put_pixel(x, y, Rgba([255, 255, 255, a]));
            }
        }
        let r = Trimmer::trim_image(&img, 0);
        assert!(r.trimmed);
        assert_eq!(r.x, 1);
        assert_eq!(r.y, 1);
        assert_eq!(r.w, 2);
        assert_eq!(r.h, 2);
    }

    #[test]
    fn test_fully_transparent() {
        let img = make_img(&[0, 0, 0, 0], 2, 2);
        let r = Trimmer::trim_image(&img, 0);
        assert!(r.trimmed);
        assert_eq!(r.w, 1);
        assert_eq!(r.h, 1);
    }
}
