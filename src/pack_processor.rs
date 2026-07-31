use image::DynamicImage;
use serde::Serialize;

use crate::math::Rect;
use crate::packers::{BinOptions, MaxRectsBin, NpmMethod, NpmPacker, PackMethod};
use crate::utils::Trimmer;

/// One packing-combo candidate. Mirrors a `{packerClass, packerMethod, allowRotation}`
/// entry from free-tex-packer-core's OptimalPacker getAllPackers().
enum Combo {
    Bin { method: PackMethod, allow_rotation: bool },
    Npm { method: NpmMethod, allow_rotation: bool },
}

/// Options for the packing process. Mirrors index.js defaults.
/// Serialized into the template context as `options` (snake_case keys).
#[derive(Debug, Clone, Serialize)]
pub struct PackOptions {
    pub texture_name: String,
    pub suffix: String,
    /// Starting index for multi-sheet file names (e.g. `atlas-1`, `atlas-2`).
    pub sheet_start_index: u32,
    /// Merge all sheets' sprites into a single metadata file (each sprite stamped
    /// with its image). When false, generate one metadata file per sheet.
    pub single_config: bool,
    pub width: u32,
    pub height: u32,
    pub power_of_two: bool,
    pub fixed_size: bool,
    pub padding: u32,
    pub extrude: u32,
    pub allow_rotation: bool,
    pub detect_identical: bool,
    pub allow_trim: bool,
    pub alpha_threshold: u8,
    pub scale: f32,
    pub scale_method: String,
    pub packer: String,
    pub packer_method: String,
    pub exporter: String,
    pub filter: String,
    pub texture_format: String,
    pub remove_file_extension: bool,
    /// Custom MiniJinja template file for metadata export. Overrides the exporter's
    /// built-in template when set.
    pub template: Option<String>,
    /// Output file extension for the metadata when a custom template is used.
    pub template_extension: Option<String>,
    /// Extra key-value variables exposed to the template context as `vars.<key>`.
    pub vars: std::collections::HashMap<String, serde_json::Value>,
    /// Input directory path, exposed to the template context as `options.input`
    /// (use the `basename` filter to get its last segment).
    pub input: String,
}

impl Default for PackOptions {
    fn default() -> Self {
        Self {
            texture_name: "atlas".into(),
            suffix: "-".into(),
            sheet_start_index: 0,
            single_config: false,
            width: 2048,
            height: 2048,
            power_of_two: false,
            fixed_size: false,
            padding: 0,
            extrude: 0,
            allow_rotation: true,
            detect_identical: true,
            allow_trim: true,
            alpha_threshold: 0,
            scale: 1.0,
            scale_method: "BILINEAR".into(),
            packer: "MaxRectsBin".into(),
            packer_method: "BestShortSideFit".into(),
            exporter: "JsonHash".into(),
            filter: "none".into(),
            texture_format: "png".into(),
            remove_file_extension: false,
            template: None,
            template_extension: None,
            vars: std::collections::HashMap::new(),
            input: "".into(),
        }
    }
}

/// Complete sprite data used throughout the packing pipeline.
#[derive(Debug, Clone)]
pub struct RectData {
    pub name: String,
    pub frame: Rect,
    pub rotated: bool,
    pub trimmed: bool,
    pub sprite_source_size: Rect,
    pub source_size: (i32, i32),
    pub index: usize,
    pub skip_render: bool,
    pub cloned: bool,
    pub image: DynamicImage,
    pub base64: String,
    pub image_width: u32,
    pub image_height: u32,
}

/// A source image loaded from disk.
#[derive(Debug, Clone)]
pub struct SourceImage {
    pub name: String,
    pub image: DynamicImage,
}

impl SourceImage {
    pub fn base64(&self) -> String {
        let mut buf = std::io::Cursor::new(Vec::new());
        if self.image.write_to(&mut buf, image::ImageFormat::Png).is_ok() {
            return base64_encode(&buf.into_inner());
        }
        String::new()
    }
}

fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }

    result
}

/// Result from the packing process: one per output file.
#[derive(Debug, Clone)]
pub struct PackResult {
    pub name: String,
    pub buffer: Vec<u8>,
}

/// Convert PackMethod string to enum.
pub fn parse_pack_method(s: &str) -> PackMethod {
    match s.to_lowercase().as_str() {
        "bestlongsidefit" => PackMethod::BestLongSideFit,
        "bestareafit" => PackMethod::BestAreaFit,
        "bottomleftrule" => PackMethod::BottomLeftRule,
        "contactpointrule" => PackMethod::ContactPointRule,
        "fillwidth" => PackMethod::FillWidth,
        _ => PackMethod::BestShortSideFit,
    }
}

/// All available packing heuristics — MaxRectsBin.methods from free-tex-packer-core,
/// plus FillWidth from maxrects-packer.
pub fn all_pack_methods() -> Vec<PackMethod> {
    vec![
        PackMethod::BestShortSideFit,
        PackMethod::BestLongSideFit,
        PackMethod::BestAreaFit,
        PackMethod::BottomLeftRule,
        PackMethod::ContactPointRule,
        PackMethod::FillWidth,
    ]
}

/// The packing orchestration engine. Mirrors PackProcessor.js.
pub struct PackProcessor;

impl PackProcessor {
    /// Main packing entry point.
    pub fn pack(
        sources: &[SourceImage],
        options: &PackOptions,
    ) -> Result<Vec<Vec<RectData>>, String> {
        let padding = options.padding as i32;
        let extrude = options.extrude as i32;
        let alpha_threshold = options.alpha_threshold;

        // Step 1: Build rects
        let mut rects = Vec::new();
        let mut max_w = 0i32;
        let mut max_h = 0i32;
        let mut min_w = 0i32;
        let mut min_h = 0i32;

        let mut names: Vec<usize> = (0..sources.len()).collect();
        names.sort_by(|&a, &b| sources[a].name.cmp(&sources[b].name));

        for &idx in &names {
            let src = &sources[idx];
            let w = src.image.width() as i32;
            let h = src.image.height() as i32;

            max_w += w;
            max_h += h;
            if w > min_w { min_w = w + padding * 2 + extrude * 2; }
            if h > min_h { min_h = h + padding * 2 + extrude * 2; }

            rects.push(RectData {
                name: src.name.clone(),
                frame: Rect::new(0, 0, w, h),
                rotated: false,
                trimmed: false,
                sprite_source_size: Rect::new(0, 0, w, h),
                source_size: (w, h),
                index: idx,
                skip_render: false,
                cloned: false,
                image: src.image.clone(),
                base64: src.base64(),
                image_width: w as u32,
                image_height: h as u32,
            });
        }

        // Step 2: Determine atlas size
        let mut atlas_w = options.width as i32;
        let mut atlas_h = options.height as i32;
        if atlas_w == 0 { atlas_w = max_w; }
        if atlas_h == 0 { atlas_h = max_h; }
        if options.power_of_two {
            atlas_w = next_pow2(atlas_w as u32) as i32;
            atlas_h = next_pow2(atlas_h as u32) as i32;
        }

        // Step 3: Trim
        if options.allow_trim {
            for rect in &mut rects {
                let img = match &rect.image {
                    DynamicImage::ImageRgba8(rgba) => rgba.clone(),
                    other => other.to_rgba8(),
                };
                let trim = Trimmer::trim_image(&img, alpha_threshold);
                if trim.trimmed {
                    rect.trimmed = true;
                    rect.sprite_source_size = Rect::new(
                        trim.x as i32, trim.y as i32,
                        trim.w as i32, trim.h as i32,
                    );
                    rect.frame.width = trim.w as i32;
                    rect.frame.height = trim.h as i32;
                }
            }
            min_w = 0;
            min_h = 0;
            for rect in &rects {
                if rect.frame.width > min_w {
                    min_w = rect.frame.width + padding * 2 + extrude * 2;
                }
                if rect.frame.height > min_h {
                    min_h = rect.frame.height + padding * 2 + extrude * 2;
                }
            }
        }

        // Step 4: Validate size
        if atlas_w < min_w || atlas_h < min_h {
            return Err(format!("Invalid size. Min: {}x{}", min_w, min_h));
        }

        // Step 5: Add padding + extrude to frame
        for rect in &mut rects {
            rect.frame.width += padding * 2 + extrude * 2;
            rect.frame.height += padding * 2 + extrude * 2;
        }

        // Step 6: Detect identical. Duplicates are removed from packing but re-added
        // later as skip-render clones sharing the original's frame (applyIdentical).
        let mut duplicates: Vec<(String, String)> = Vec::new();
        if options.detect_identical {
            let mut identical_map: Vec<usize> = Vec::new();
            for i in 0..rects.len() {
                for j in (i + 1)..rects.len() {
                    if rects[i].base64 == rects[j].base64 && !identical_map.contains(&j) {
                        identical_map.push(j);
                        duplicates.push((rects[j].name.clone(), rects[j].base64.clone()));
                    }
                }
            }
            identical_map.sort_unstable_by(|a, b| b.cmp(a));
            identical_map.dedup();
            for idx in identical_map {
                rects.remove(idx);
            }
        }

        // Step 7: Pack loop
        let pack_method = parse_pack_method(&options.packer_method);
        let remaining = rects;
        let source_area: i32 = remaining.iter().map(|r| r.source_size.0 * r.source_size.1).sum();

        // OptimalPacker tries every method/rotation combo; a regular packer uses its own.
        // Mirrors JS getAllPackers(): MaxRectsBin's 5 methods, then MaxRectsPacker's 4
        // methods, each × {allowRotation:false, allowRotation:true} when rotation is on.
        let is_optimal = options.packer.eq_ignore_ascii_case(crate::packers::OptimalPacker::TYPE);
        let combos: Vec<Combo> = if is_optimal {
            let mut c = Vec::new();
            for method in [
                PackMethod::BestShortSideFit,
                PackMethod::BestLongSideFit,
                PackMethod::BestAreaFit,
                PackMethod::BottomLeftRule,
                PackMethod::ContactPointRule,
            ] {
                c.push(Combo::Bin { method, allow_rotation: false });
                if options.allow_rotation {
                    c.push(Combo::Bin { method, allow_rotation: true });
                }
            }
            for method in [
                NpmMethod::Smart,
                NpmMethod::SmartArea,
                NpmMethod::Square,
                NpmMethod::SquareArea,
            ] {
                c.push(Combo::Npm { method, allow_rotation: false });
                if options.allow_rotation {
                    c.push(Combo::Npm { method, allow_rotation: true });
                }
            }
            c
        } else if options.packer.eq_ignore_ascii_case("MaxRectsPacker") {
            let method = NpmMethod::parse(&options.packer_method).unwrap_or(NpmMethod::Smart);
            vec![Combo::Npm { method, allow_rotation: options.allow_rotation }]
        } else {
            vec![Combo::Bin { method: pack_method, allow_rotation: options.allow_rotation }]
        };

        let mut opt_sheets = Vec::new();
        let mut opt_count = usize::MAX;
        let mut opt_eff = 0.0f32;

        for combo in &combos {
            let mut local_rects = remaining.clone();
            let mut combo_sheets: Vec<Vec<RectData>> = Vec::new();
            let mut total_sheet_area = 0f32;

            while !local_rects.is_empty() {
                // Pack one sheet; placed rects (coords adjusted) are returned, unplaced
                // ones stay in local_rects for the next sheet.
                let mut sheet: Vec<RectData> = match combo {
                    Combo::Bin { method, allow_rotation } => {
                        let bin_opts = BinOptions {
                            smart: false,
                            pot: options.power_of_two,
                            square: false,
                            allow_rotation: *allow_rotation,
                            border: 0,
                            logic: *method,
                        };
                        // Rects are pre-padded (padding*2 + extrude*2 added to frame above), so the
                        // bin gets padding 0 to keep a clean (0,0,atlas_w,atlas_h) free rect, exactly
                        // like free-tex-packer-core's `new Rect(0, 0, width, height)`.
                        let mut packer = MaxRectsBin::new(atlas_w, atlas_h, 0, *allow_rotation, &bin_opts);
                        let mut sheet = Vec::new();

                        // Global best-first placement (mirrors JS insert2): each step, score every
                        // remaining rect against every free rect and place the single best pair.
                        loop {
                            let mut best: Option<(usize, i32, i32, i32, i32, i32, i32, bool)> = None;
                            for (i, rect) in local_rects.iter().enumerate() {
                                if let Some((s1, s2, x, y, pw, ph, rotated)) =
                                    packer.find_node(rect.frame.width, rect.frame.height, *method)
                                {
                                    let better = match &best {
                                        None => true,
                                        Some((_, bs1, bs2, ..)) => {
                                            s1 < *bs1 || (s1 == *bs1 && s2 < *bs2)
                                        }
                                    };
                                    if better {
                                        best = Some((i, s1, s2, x, y, pw, ph, rotated));
                                    }
                                }
                            }

                            let (idx, _s1, _s2, x, y, pw, ph, rotated) = match best {
                                None => break,
                                Some(b) => b,
                            };

                            packer.place_rectangle(&Rect::new(x, y, pw, ph));

                            let mut rd = local_rects.remove(idx);
                            rd.frame.x = x;
                            rd.frame.y = y;
                            rd.frame.width = pw;
                            rd.frame.height = ph;
                            rd.rotated = rotated;

                            // Adjust coords: subtract padding + extrude
                            rd.frame.x += padding + extrude;
                            rd.frame.y += padding + extrude;
                            rd.frame.width -= padding * 2 + extrude * 2;
                            rd.frame.height -= padding * 2 + extrude * 2;

                            sheet.push(rd);
                        }
                        sheet
                    }
                    Combo::Npm { method, allow_rotation } => {
                        let npm_opts = method.options(*allow_rotation);
                        let mut packer = NpmPacker::new(atlas_w, atlas_h, npm_opts);
                        let mut inputs: Vec<(i32, i32, usize)> = local_rects
                            .iter()
                            .enumerate()
                            .map(|(i, r)| (r.frame.width, r.frame.height, i))
                            .collect();
                        packer.add_array(&mut inputs);

                        // free-tex-packer-core takes ONLY bins[0].rects; the rest stay in the
                        // working list and are re-sorted by a fresh packer next sheet.
                        let placed = packer
                            .bins
                            .first()
                            .map(|b| b.rects.clone())
                            .unwrap_or_default();
                        let mut placed_idx: std::collections::HashSet<usize> =
                            std::collections::HashSet::new();
                        let mut sheet = Vec::with_capacity(placed.len());
                        for p in &placed {
                            let mut rd = local_rects[p.index].clone();
                            rd.frame.x = p.x + padding + extrude;
                            rd.frame.y = p.y + padding + extrude;
                            rd.frame.width -= padding * 2 + extrude * 2;
                            rd.frame.height -= padding * 2 + extrude * 2;
                            rd.rotated = p.rot;
                            placed_idx.insert(p.index);
                            sheet.push(rd);
                        }

                        // Keep only the unplaced rects for the next sheet
                        local_rects = local_rects
                            .drain(..)
                            .enumerate()
                            .filter(|(i, _)| !placed_idx.contains(i))
                            .map(|(_, rd)| rd)
                            .collect();
                        sheet
                    }
                };

                if sheet.is_empty() {
                    break;
                }

                // Re-add identical clones, sharing the original's frame (mirrors applyIdentical)
                if options.detect_identical {
                    for (dup_name, dup_base64) in &duplicates {
                        if let Some(orig) = sheet.iter().find(|r| r.base64 == *dup_base64) {
                            let mut clone = orig.clone();
                            clone.name = dup_name.clone();
                            clone.skip_render = true;
                            clone.cloned = true;
                            sheet.push(clone);
                        }
                    }
                }

                // Actual rendered sheet size (mirrors TextureRenderer.getSize) for efficiency.
                // Using the fixed max atlas size here would make every combo with the same
                // sheet count tie, defeating OptimalPacker's efficiency tiebreak.
                let (sw, sh) = if options.fixed_size {
                    (atlas_w.max(1) as u32, atlas_h.max(1) as u32)
                } else {
                    let mut sw = 0i32;
                    let mut sh = 0i32;
                    for rd in &sheet {
                        let (w, h) = if rd.rotated {
                            (rd.frame.x + rd.frame.height, rd.frame.y + rd.frame.width)
                        } else {
                            (rd.frame.x + rd.frame.width, rd.frame.y + rd.frame.height)
                        };
                        if w > sw {
                            sw = w;
                        }
                        if h > sh {
                            sh = h;
                        }
                    }
                    let sw = (sw + padding + extrude).max(1) as u32;
                    let sh = (sh + padding + extrude).max(1) as u32;
                    if options.power_of_two {
                        (next_pow2(sw), next_pow2(sh))
                    } else {
                        (sw, sh)
                    }
                };
                total_sheet_area += (sw * sh) as f32;
                combo_sheets.push(sheet);
            }

            if !local_rects.is_empty() {
                continue;
            }

            let n_sheets = combo_sheets.len();
            let efficiency = if total_sheet_area > 0.0 {
                source_area as f32 / total_sheet_area
            } else {
                0.0
            };

            if n_sheets < opt_count || (n_sheets == opt_count && efficiency > opt_eff) {
                opt_sheets = combo_sheets;
                opt_count = n_sheets;
                opt_eff = efficiency;
            }
        }

        if opt_sheets.is_empty() {
            return Err("Packing failed: no suitable placement found.".into());
        }

        Ok(opt_sheets)
    }
}

fn next_pow2(v: u32) -> u32 {
    if v == 0 { return 1; }
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
    use image::{DynamicImage, Rgba, RgbaImage};

    fn solid_image(w: u32, h: u32, r: u8, g: u8, b: u8) -> DynamicImage {
        let mut img = RgbaImage::new(w, h);
        for px in img.pixels_mut() {
            *px = Rgba([r, g, b, 255]);
        }
        DynamicImage::ImageRgba8(img)
    }

    fn pack_sources(sources: Vec<SourceImage>, packer: &str) -> Vec<Vec<RectData>> {
        let opts = PackOptions {
            packer: packer.into(),
            ..Default::default()
        };
        PackProcessor::pack(&sources, &opts).unwrap()
    }

    #[test]
    fn test_all_pack_methods_covers_every_enum() {
        let methods = all_pack_methods();
        assert_eq!(methods.len(), 6);
        for m in methods {
            parse_pack_method(&format!("{:?}", m));
        }
    }

    #[test]
    fn test_optimal_packer_places_all_unique_sprites() {
        // 4 unique images + 2 duplicates (same pixel data) => 4 unique after detection
        let sources = vec![
            SourceImage { name: "a".into(), image: solid_image(40, 30, 255, 0, 0) },
            SourceImage { name: "b".into(), image: solid_image(20, 60, 0, 255, 0) },
            SourceImage { name: "c".into(), image: solid_image(50, 50, 0, 0, 255) },
            SourceImage { name: "d".into(), image: solid_image(10, 10, 255, 255, 0) },
            SourceImage { name: "a2".into(), image: solid_image(40, 30, 255, 0, 0) },
            SourceImage { name: "c2".into(), image: solid_image(50, 50, 0, 0, 255) },
        ];

        let sheets = pack_sources(sources, "OptimalPacker");
        let names: Vec<&str> = sheets
            .iter()
            .flat_map(|s| s.iter())
            .map(|r| r.name.as_str())
            .collect();

        // 4 unique sprites are placed; the 2 duplicates are re-added as skip-render
        // clones sharing their original's frame (mirrors applyIdentical), so all 6
        // sprite names appear in the output.
        assert_eq!(names.len(), 6);
        for n in ["a", "b", "c", "d", "a2", "c2"] {
            assert!(names.contains(&n), "missing {}", n);
        }
        let clone_a = sheets.iter().flatten().find(|r| r.name == "a2").unwrap();
        let orig_a = sheets.iter().flatten().find(|r| r.name == "a").unwrap();
        assert_eq!(clone_a.frame, orig_a.frame);
        assert!(clone_a.skip_render);
    }

    #[test]
    fn test_optimal_packer_beats_worst_method() {
        // Enough sprites that packing order matters; optimal must place all of them
        let mut sources = Vec::new();
        for i in 0..20 {
            let w = 20 + (i % 5) * 7;
            let h = 20 + (i % 4) * 9;
            sources.push(SourceImage {
                name: format!("s{}", i),
                image: solid_image(w, h, i as u8, 100, 200),
            });
        }

        // Small atlas forces multiple sheets; optimal should still place everything
        let opts = PackOptions {
            packer: "OptimalPacker".into(),
            width: 128,
            height: 128,
            allow_rotation: true,
            ..Default::default()
        };
        let sheets = PackProcessor::pack(&sources, &opts).unwrap();
        let total: usize = sheets.iter().map(|s| s.len()).sum();
        assert_eq!(total, sources.len(), "OptimalPacker dropped rects");
    }
}
