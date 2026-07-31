use serde::Serialize;

use crate::pack_processor::RectData;

/// Exporter descriptor, mirrors exporters/list.json.
pub struct ExporterDescriptor {
    pub type_name: &'static str,
    pub file_ext: &'static str,
    pub allow_trim: bool,
    pub allow_rotation: bool,
}

/// Built-in exporters list.
pub fn list_exporters() -> Vec<ExporterDescriptor> {
    vec![
        ExporterDescriptor {
            type_name: "JsonHash",
            file_ext: "json",
            allow_trim: true,
            allow_rotation: true,
        },
        ExporterDescriptor {
            type_name: "JsonArray",
            file_ext: "json",
            allow_trim: true,
            allow_rotation: true,
        },
    ]
}

/// Get an exporter by type name.
pub fn get_exporter_by_type(type_name: &str) -> Option<ExporterDescriptor> {
    let lower = type_name.to_lowercase();
    list_exporters()
        .into_iter()
        .find(|e| e.type_name.to_lowercase() == lower)
}

/// Data for a single rect in export output.
#[derive(Debug, Clone, Serialize)]
pub struct ExportRect {
    pub name: String,
    #[serde(flatten)]
    pub frame: ExportFrame,
    pub rotated: bool,
    pub trimmed: bool,
    #[serde(rename = "spriteSourceSize")]
    pub sprite_source_size: ExportFrame,
    #[serde(rename = "sourceSize")]
    pub source_size: ExportSize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportFrame {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportSize {
    pub w: i32,
    pub h: i32,
}

/// Start export — generate metadata string.
pub fn start_exporter(
    type_name: &str,
    rects: &[RectData],
    base_name: &str,
    remove_file_extension: bool,
) -> String {
    let exporter = get_exporter_by_type(type_name)
        .unwrap_or_else(|| get_exporter_by_type("JsonHash").unwrap());

    let export_rects: Vec<ExportRect> = prepare_data(rects, remove_file_extension);

    match exporter.type_name {
        "JsonArray" => export_json_array(&export_rects, base_name),
        _ => export_json_hash(&export_rects, base_name),
    }
}

/// Strip the last `.ext` segment, mirroring the JS `split(".").pop()`.
fn strip_extension(name: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, _)) => stem.to_string(),
        None => name.to_string(),
    }
}

fn prepare_data(rects: &[RectData], remove_file_extension: bool) -> Vec<ExportRect> {
    rects
        .iter()
        .map(|r| ExportRect {
            name: if remove_file_extension {
                strip_extension(&r.name)
            } else {
                r.name.clone()
            },
            frame: ExportFrame {
                x: r.frame.x,
                y: r.frame.y,
                w: r.frame.width,
                h: r.frame.height,
            },
            rotated: r.rotated,
            trimmed: r.trimmed,
            sprite_source_size: ExportFrame {
                x: r.sprite_source_size.x,
                y: r.sprite_source_size.y,
                w: r.sprite_source_size.width,
                h: r.sprite_source_size.height,
            },
            source_size: ExportSize {
                w: r.source_size.0,
                h: r.source_size.1,
            },
        })
        .collect()
}

/// JsonHash format: `{ "filename.png": { frame: {...}, ... }, ... }`
fn export_json_hash(rects: &[ExportRect], base_name: &str) -> String {
    let mut map = serde_json::Map::new();
    map.insert(
        "name".to_string(),
        serde_json::Value::String(format!("{}.png", base_name)),
    );

    let rects_val: Vec<serde_json::Value> = rects
        .iter()
        .map(|r| {
            let mut rect_map = serde_json::Map::new();
            rect_map.insert("name".into(), serde_json::Value::String(r.name.clone()));
            rect_map.insert(
                "frame".into(),
                serde_json::to_value(&r.frame).unwrap_or_default(),
            );
            rect_map.insert("rotated".into(), serde_json::Value::Bool(r.rotated));
            rect_map.insert("trimmed".into(), serde_json::Value::Bool(r.trimmed));
            rect_map.insert(
                "spriteSourceSize".into(),
                serde_json::to_value(&r.sprite_source_size).unwrap_or_default(),
            );
            rect_map.insert(
                "sourceSize".into(),
                serde_json::to_value(&r.source_size).unwrap_or_default(),
            );
            serde_json::Value::Object(rect_map)
        })
        .collect();

    map.insert("sprites".to_string(), serde_json::Value::Array(rects_val));
    serde_json::to_string_pretty(&serde_json::Value::Object(map)).unwrap_or_default()
}

/// JsonArray format: `[ { name, frame, ... }, ... ]`
fn export_json_array(rects: &[ExportRect], _base_name: &str) -> String {
    serde_json::to_string_pretty(rects).unwrap_or_default()
}
