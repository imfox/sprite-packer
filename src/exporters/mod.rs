use serde::Serialize;

use crate::pack_processor::RectData;

/// Exporter descriptor, mirrors exporters/list.json.
pub struct ExporterDescriptor {
    pub type_name: &'static str,
    pub file_ext: &'static str,
    pub allow_trim: bool,
    pub allow_rotation: bool,
    /// MiniJinja template that renders the metadata output.
    pub template: &'static str,
}

/// JsonHash template. Key order is alphabetical (matches the old BTreeMap-built output).
const JSON_HASH_TEMPLATE: &str = r#"{
  "name": "{{ base_name }}.png",
  "sprites": [
{% for r in sprites %}    {
      "frame": {
        "h": {{ r.frame.h }},
        "w": {{ r.frame.w }},
        "x": {{ r.frame.x }},
        "y": {{ r.frame.y }}
      },
      "name": {{ r.name | to_json }},
      "rotated": {{ r.rotated }},
      "sourceSize": {
        "h": {{ r.source_size.h }},
        "w": {{ r.source_size.w }}
      },
      "spriteSourceSize": {
        "h": {{ r.sprite_source_size.h }},
        "w": {{ r.sprite_source_size.w }},
        "x": {{ r.sprite_source_size.x }},
        "y": {{ r.sprite_source_size.y }}
      },
      "trimmed": {{ r.trimmed }}
    }{% if not loop.last %},{% endif %}
{% endfor %}  ]
}"#;

/// JsonArray template. Field order mirrors the derived Serialize layout, with the
/// flattened frame inlined as x/y/w/h.
const JSON_ARRAY_TEMPLATE: &str = r#"[
{% for r in sprites %}  {
    "name": {{ r.name | to_json }},
    "x": {{ r.frame.x }},
    "y": {{ r.frame.y }},
    "w": {{ r.frame.w }},
    "h": {{ r.frame.h }},
    "rotated": {{ r.rotated }},
    "trimmed": {{ r.trimmed }},
    "spriteSourceSize": {
      "x": {{ r.sprite_source_size.x }},
      "y": {{ r.sprite_source_size.y }},
      "w": {{ r.sprite_source_size.w }},
      "h": {{ r.sprite_source_size.h }}
    },
    "sourceSize": {
      "w": {{ r.source_size.w }},
      "h": {{ r.source_size.h }}
    }
  }{% if not loop.last %},{% endif %}
{% endfor %}]"#;

/// Built-in exporters list.
pub fn list_exporters() -> Vec<ExporterDescriptor> {
    vec![
        ExporterDescriptor {
            type_name: "JsonHash",
            file_ext: "json",
            allow_trim: true,
            allow_rotation: true,
            template: JSON_HASH_TEMPLATE,
        },
        ExporterDescriptor {
            type_name: "JsonArray",
            file_ext: "json",
            allow_trim: true,
            allow_rotation: true,
            template: JSON_ARRAY_TEMPLATE,
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

/// Start export — render the exporter template to a metadata string.
///
/// `template` optionally points to a custom MiniJinja template file that overrides
/// the exporter's built-in template.
pub fn start_exporter(
    type_name: &str,
    rects: &[RectData],
    base_name: &str,
    remove_file_extension: bool,
    template: Option<&str>,
) -> Result<String, String> {
    let exporter = get_exporter_by_type(type_name)
        .unwrap_or_else(|| get_exporter_by_type("JsonHash").unwrap());

    let template = match template {
        Some(path) => std::fs::read_to_string(path)
            .map_err(|e| format!("Error reading template '{}': {}", path, e))?,
        None => exporter.template.to_string(),
    };

    let export_rects: Vec<ExportRect> = prepare_data(rects, remove_file_extension);
    render(&template, &export_rects, base_name)
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

/// Render a MiniJinja template with the exporter context.
fn render(template: &str, rects: &[ExportRect], base_name: &str) -> Result<String, String> {
    let sprites: Vec<serde_json::Value> = rects.iter().map(rect_to_json).collect();
    let ctx = serde_json::json!({ "base_name": base_name, "sprites": sprites });

    let mut env = minijinja::Environment::new();
    env.add_filter("to_json", to_json);
    env.render_str(template, ctx)
        .map_err(|e| format!("Template error: {}", e))
}

/// JSON-encode a scalar (used for names so quotes/escapes stay valid).
fn to_json(value: &minijinja::Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

fn rect_to_json(r: &ExportRect) -> serde_json::Value {
    serde_json::json!({
        "frame": { "x": r.frame.x, "y": r.frame.y, "w": r.frame.w, "h": r.frame.h },
        "name": r.name,
        "rotated": r.rotated,
        "source_size": { "w": r.source_size.w, "h": r.source_size.h },
        "sprite_source_size": {
            "x": r.sprite_source_size.x,
            "y": r.sprite_source_size.y,
            "w": r.sprite_source_size.w,
            "h": r.sprite_source_size.h
        },
        "trimmed": r.trimmed,
    })
}
