use image::DynamicImage;

use crate::exporters;
use crate::pack_processor::{PackOptions, PackResult, RectData};
use crate::utils::{TextureRenderer, RenderItem, RenderOptions};
use crate::filters;

/// Generates output files from packed sprite sheets.
/// Mirrors FilesProcessor.js from free-tex-packer-core.
pub struct FilesProcessor;

impl FilesProcessor {
    /// Process packed sheet data — render textures and export metadata.
    pub fn process(
        sheets: &[Vec<RectData>],
        options: &PackOptions,
    ) -> Result<Vec<PackResult>, String> {
        let mut results = Vec::new();
        let suffix = &options.suffix;
        let multi_sheet = sheets.len() > 1;
        // Multi-sheet configs can be merged into a single metadata file
        let merged = multi_sheet && !options.multi_config;

        let mut groups: Vec<(u32, String, &[RectData])> = Vec::new();

        for (sheet_idx, sheet) in sheets.iter().enumerate() {
            // Build render items
            let mut render_items = Vec::new();
            for rd in sheet {
                render_items.push(RenderItem {
                    image: rd.image.clone(),
                    dx: rd.frame.x,
                    dy: rd.frame.y,
                    sx: rd.sprite_source_size.x,
                    sy: rd.sprite_source_size.y,
                    sw: rd.sprite_source_size.width,
                    sh: rd.sprite_source_size.height,
                    ow: rd.source_size.0,
                    oh: rd.source_size.1,
                    skip_render: rd.skip_render,
                    rotated: rd.rotated,
                    extrude: options.extrude as i32,
                });
            }

            // Render atlas image
            let render_opts = RenderOptions {
                fixed_size: options.fixed_size,
                width: options.width,
                height: options.height,
                power_of_two: options.power_of_two,
                padding: options.padding,
                extrude: options.extrude,
                scale: options.scale,
            };

            let mut render_result = TextureRenderer::render(&render_items, &render_opts);

            // Apply filter
            if let Some(filter) = filters::get_filter_by_type(&options.filter) {
                filter.apply(&mut render_result.image);
            }

            // Encode PNG
            let mut png_buf = std::io::Cursor::new(Vec::new());
            let img = DynamicImage::ImageRgba8(render_result.image);
            let _ = img.write_to(&mut png_buf, image::ImageFormat::Png);
            let image_data = png_buf.into_inner();

            // Sheet index used in the file name and stamped on each sprite
            let index = options.sheet_start_index + sheet_idx as u32;

            // Generate file name
            let fname = if multi_sheet {
                format!("{}{}{}", options.texture_name, suffix, index)
            } else {
                options.texture_name.clone()
            };

            let image_name = format!("{}.png", fname);

            // Push atlas image
            results.push(PackResult {
                name: image_name.clone(),
                buffer: image_data,
            });

            if merged {
                groups.push((index, image_name, sheet));
            } else {
                // Push per-sheet metadata
                let metadata = exporters::start_exporter(
                    &options.exporter,
                    sheet,
                    &fname,
                    index,
                    &image_name,
                    options.remove_file_extension,
                    options.template.as_deref(),
                    &options.vars,
                )?;
                results.push(PackResult {
                    name: format!("{}.{}", fname, metadata_ext(options)),
                    buffer: metadata.into_bytes(),
                });
            }
        }

        // Merged multi-sheet metadata: one file covering all sheets
        if merged {
            let metadata = exporters::start_exporter_merged(
                &options.exporter,
                &groups,
                &options.texture_name,
                options.remove_file_extension,
                options.template.as_deref(),
                &options.vars,
            )?;
            results.push(PackResult {
                name: format!("{}.{}", options.texture_name, metadata_ext(options)),
                buffer: metadata.into_bytes(),
            });
        }

        Ok(results)
    }
}

/// Metadata file extension: custom template extension wins, else exporter's.
fn metadata_ext(options: &PackOptions) -> String {
    options
        .template_extension
        .clone()
        .unwrap_or_else(|| {
            exporters::get_exporter_by_type(&options.exporter)
                .map(|e| e.file_ext.to_string())
                .unwrap_or_else(|| "json".into())
        })
}
