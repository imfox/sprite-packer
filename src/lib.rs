pub mod math;
pub mod packers;
pub mod utils;
pub mod filters;
pub mod exporters;
pub mod pack_processor;
pub mod files_processor;

pub use pack_processor::{PackOptions, PackProcessor, PackResult, RectData, SourceImage};
pub use files_processor::FilesProcessor;

use std::path::Path;
use walkdir::WalkDir;

/// Scan a directory recursively for supported image files.
pub fn scan_dir(dir: &str) -> Result<Vec<SourceImage>, String> {
    let mut sources = Vec::new();

    for entry in WalkDir::new(dir).follow_links(true) {
        let entry = entry.map_err(|e| format!("Walk error: {}", e))?;
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "bmp") {
            continue;
        }

        match image::open(path) {
            Ok(img) => {
                let name = normalize_name(path);
                sources.push(SourceImage { name, image: img });
            }
            Err(e) => {
                eprintln!("  Warning: skipping {} ({})", path.display(), e);
            }
        }
    }

    sources.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(sources)
}

/// Normalize a file path to a sprite name (file name, keeping the extension).
/// Mirrors free-tex-packer-core: names keep extensions unless `removeFileExtension`
/// is set (stripped later during export).
fn normalize_name(path: &Path) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Main packing entry point.
/// Takes source images and options, returns output files.
pub fn pack(sources: &[SourceImage], options: &PackOptions) -> Result<Vec<PackResult>, String> {
    let sheets = PackProcessor::pack(sources, options)?;
    let results = FilesProcessor::process(&sheets, options);
    Ok(results)
}
