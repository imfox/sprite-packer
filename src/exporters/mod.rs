use serde::Serialize;

use crate::pack_processor::{PackOptions, RectData};

/// Extra key-value variables available to the template context as `vars.<key>`.
pub type Vars = std::collections::HashMap<String, serde_json::Value>;

/// Exporter descriptor, mirrors exporters/list.json.
pub struct ExporterDescriptor {
    pub type_name: &'static str,
    pub file_ext: &'static str,
    pub allow_trim: bool,
    pub allow_rotation: bool,
    /// MiniJinja template that renders the metadata output.
    pub template: &'static str,
}

/// Generation info exposed to the template context as `exports`.
#[derive(Debug, Clone)]
pub struct ExportInfo {
    /// Whether this export merges all sheets into one config (true) or produces one
    /// config per sheet (false).
    pub single_config: bool,
    /// Total number of atlas sheets produced.
    pub image_count: usize,
    /// Index of the sheet this config describes (merged exports report the first sheet).
    pub cur_image_index: u32,
    /// Image name of the sheet this config describes (usable with `image_dict`).
    pub cur_image: String,
}

/// Metadata for one rendered atlas image, exposed to the template context as
/// `images` (one entry per atlas, in sheet order).
#[derive(Debug, Clone)]
pub struct AtlasInfo {
    pub name: String,
    pub index: u32,
    pub width: u32,
    pub height: u32,
}

/// JsonHash template. Key order is alphabetical (matches the old BTreeMap-built output).
/// Per-sheet output (exports.single_config=false) matches the reference; when sheets are
/// merged into one config (exports.single_config=true), each sprite is stamped with the
/// atlas `image` and `index` it belongs to.
const JSON_HASH_TEMPLATE: &str = r#"{
  "name": "{{ options.texture_name }}.png",
  "sprites": [
{% for r in sprites %}    {
      "frame": {
        "h": {{ r.frame.h }},
        "w": {{ r.frame.w }},
        "x": {{ r.frame.x }},
        "y": {{ r.frame.y }}
      },
{% if exports.single_config %}      "image": {{ r.image | to_json }},
      "index": {{ r.index }},
{% endif %}      "name": {{ r.name | to_json }},
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
{% if exports.single_config %}    "image": {{ r.image | to_json }},
    "index": {{ r.index }},
{% endif %}    "x": {{ r.frame.x }},
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
    /// Atlas image this sprite belongs to (e.g. `atlas-0.png`).
    pub image: String,
    /// Index of the atlas sheet this sprite belongs to (e.g. 0 for `atlas-0.png`).
    pub index: u32,
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
/// the exporter's built-in template. `sheet_index` and `image` identify the atlas
/// sheet these rects belong to. Per-sheet export always uses exports.single_config=false
/// so the built-in template matches the reference output. `info.image_count` is the
/// total number of atlas sheets produced. `images` and `options` are exposed to the
/// template context. `vars` are extra key-value pairs exposed as `vars.<key>`.
pub fn start_exporter(
    type_name: &str,
    rects: &[RectData],
    sheet_index: u32,
    image: &str,
    remove_file_extension: bool,
    template: Option<&str>,
    vars: &Vars,
    images: &[AtlasInfo],
    image_count: usize,
    options: &PackOptions,
) -> Result<String, String> {
    let template = resolve_template(type_name, template)?;
    let prepared = prepare_data(rects, remove_file_extension, sheet_index, image, false, vars);
    let info = ExportInfo {
        single_config: prepared.single_config,
        image_count,
        cur_image_index: sheet_index,
        cur_image: image.to_string(),
    };
    render(&template, &prepared.rects, &info, &prepared.vars, images, options)
}

/// Export a single merged metadata file covering multiple sheets. Each sprite is
/// stamped with the `image` (atlas file name) and `index` (atlas sheet index) of the
/// sheet it belongs to. `groups` holds (sheet_index, image_name, sprites) per sheet.
/// `info.image_count` is the total number of atlas sheets produced. `images` and
/// `options` are exposed to the template context.
pub fn start_exporter_merged(
    type_name: &str,
    groups: &[(u32, String, &[RectData])],
    remove_file_extension: bool,
    template: Option<&str>,
    vars: &Vars,
    images: &[AtlasInfo],
    image_count: usize,
    options: &PackOptions,
) -> Result<String, String> {
    let template = resolve_template(type_name, template)?;

    let export_rects: Vec<ExportRect> = groups
        .iter()
        .flat_map(|(index, image, rects)| {
            prepare_data(rects, remove_file_extension, *index, image, true, vars).rects
        })
        .collect();

    let (cur_image_index, cur_image) = groups
        .first()
        .map(|g| (g.0, g.1.clone()))
        .unwrap_or((0, String::new()));
    let info = ExportInfo {
        single_config: true,
        image_count,
        cur_image_index,
        cur_image,
    };
    render(&template, &export_rects, &info, vars, images, options)
}

/// Resolve the template to render: a custom template file when given, otherwise the
/// built-in template for the exporter.
fn resolve_template(type_name: &str, custom: Option<&str>) -> Result<String, String> {
    if let Some(path) = custom {
        return std::fs::read_to_string(path)
            .map_err(|e| format!("Error reading template '{}': {}", path, e));
    }
    let exporter = get_exporter_by_type(type_name)
        .unwrap_or_else(|| get_exporter_by_type("JsonHash").unwrap());
    Ok(exporter.template.to_string())
}

/// Strip the last `.ext` segment, mirroring the JS `split(".").pop()`.
fn strip_extension(name: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, _)) => stem.to_string(),
        None => name.to_string(),
    }
}

/// Prepared export data, carrying the single-config flag so the template can tell
/// per-sheet output (one config per atlas) from merged output (one config for all),
/// plus the extra variables available to the template.
pub struct PreparedExport {
    pub rects: Vec<ExportRect>,
    pub single_config: bool,
    pub vars: Vars,
}

fn prepare_data(
    rects: &[RectData],
    remove_file_extension: bool,
    sheet_index: u32,
    image: &str,
    single_config: bool,
    vars: &Vars,
) -> PreparedExport {
    PreparedExport {
        rects: rects
            .iter()
            .map(|r| ExportRect {
                name: if remove_file_extension {
                    strip_extension(&r.name)
                } else {
                    r.name.clone()
                },
                image: image.to_string(),
                index: sheet_index,
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
            .collect(),
        single_config,
        vars: vars.clone(),
    }
}

/// Render a MiniJinja template with the exporter context.
fn render(
    template: &str,
    rects: &[ExportRect],
    info: &ExportInfo,
    vars: &Vars,
    images: &[AtlasInfo],
    options: &PackOptions,
) -> Result<String, String> {
    let atlas_info = |i: &AtlasInfo| {
        serde_json::json!({ "name": i.name, "index": i.index, "width": i.width, "height": i.height })
    };
    let sprites: Vec<serde_json::Value> = rects.iter().map(rect_to_json).collect();
    let images_json: Vec<serde_json::Value> = images.iter().map(atlas_info).collect();
    // image_dict maps atlas file name -> atlas info, so a sprite's atlas can be
    // looked up by name (`image_dict[r.image]`) instead of by `index`, which may
    // not start at 0 and cannot be used as an array offset.
    let image_dict: serde_json::Value = images
        .iter()
        .map(|i| (i.name.clone(), atlas_info(i)))
        .collect();
    let exports = serde_json::json!({
        "single_config": info.single_config,
        "image_count": info.image_count,
        "cur_image_index": info.cur_image_index,
        "cur_image": info.cur_image,
    });
    let ctx = serde_json::json!({
        "exports": exports,
        "sprites": sprites,
        "images": images_json,
        "image_dict": image_dict,
        "vars": vars,
        "options": options,
    });

    let mut env = minijinja::Environment::new();
    env.add_filter("to_json", to_json);
    env.add_filter("without_extname", without_extname);
    env.add_filter("basename", basename);
    env.render_str(template, ctx)
        .map_err(|e| format!("Template error: {}", e))
}

/// JSON-encode a scalar (used for names so quotes/escapes stay valid).
fn to_json(value: &minijinja::Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// Strip the last `.ext` segment of a sprite name (e.g. `x.png` → `x`).
fn without_extname(value: &minijinja::Value) -> String {
    match value.as_str() {
        Some(s) => strip_extension(s),
        None => String::new(),
    }
}

/// Extract the last path segment (e.g. `/a/b/c` → `c`). Handles both `/` and `\`.
fn basename(value: &minijinja::Value) -> String {
    match value.as_str() {
        Some(s) => s.rsplit(['/', '\\']).next().unwrap_or("").to_string(),
        None => String::new(),
    }
}

fn rect_to_json(r: &ExportRect) -> serde_json::Value {
    serde_json::json!({
        "frame": { "x": r.frame.x, "y": r.frame.y, "w": r.frame.w, "h": r.frame.h },
        "image": r.image,
        "index": r.index,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_info(single_config: bool, image_count: usize, cur_image_index: u32, cur_image: &str) -> ExportInfo {
        ExportInfo {
            single_config,
            image_count,
            cur_image_index,
            cur_image: cur_image.into(),
        }
    }

    fn sample_rects() -> Vec<ExportRect> {
        vec![
            ExportRect {
                name: "a.png".into(),
                image: "atlas-1.png".into(),
                index: 1,
                frame: ExportFrame { x: 0, y: 0, w: 10, h: 20 },
                rotated: false,
                trimmed: true,
                sprite_source_size: ExportFrame { x: 2, y: 3, w: 10, h: 20 },
                source_size: ExportSize { w: 50, h: 60 },
            },
            ExportRect {
                name: "b.png".into(),
                image: "atlas-2.png".into(),
                index: 2,
                frame: ExportFrame { x: 0, y: 20, w: 30, h: 40 },
                rotated: true,
                trimmed: false,
                sprite_source_size: ExportFrame { x: 0, y: 0, w: 30, h: 40 },
                source_size: ExportSize { w: 30, h: 40 },
            },
        ]
    }

    #[test]
    fn per_sheet_output_has_no_image_or_index() {
        let rects = sample_rects();
        let vars = Vars::new();
        let out = render(JSON_HASH_TEMPLATE, &rects, &sample_info(false, 1, 0, ""), &vars, &[], &PackOptions::default()).unwrap();
        // The config name comes from options.texture_name, and per-sheet output has no
        // image/index stamping.
        assert!(out.contains("\"name\": \"atlas.png\""));
        assert!(!out.contains("image"));
        assert!(!out.contains("index"));
    }

    #[test]
    fn merged_hash_output_has_image_and_index() {
        let rects = sample_rects();
        let vars = Vars::new();
        let out = render(JSON_HASH_TEMPLATE, &rects, &sample_info(true, 1, 0, ""), &vars, &[], &PackOptions::default()).unwrap();
        assert!(out.contains("\"image\": \"atlas-1.png\""));
        assert!(out.contains("\"index\": 1"));
        assert!(out.contains("\"image\": \"atlas-2.png\""));
        assert!(out.contains("\"index\": 2"));
        // JsonHash keys are alphabetical: image before name within a sprite
        assert!(out.find("\"image\"").unwrap() < out.find("\"name\": \"a.png\"").unwrap());
    }

    #[test]
    fn merged_array_output_has_image_and_index() {
        let rects = sample_rects();
        let vars = Vars::new();
        let out = render(JSON_ARRAY_TEMPLATE, &rects, &sample_info(true, 1, 0, ""), &vars, &[], &PackOptions::default()).unwrap();
        assert!(out.contains("\"image\": \"atlas-1.png\""));
        assert!(out.contains("\"index\": 1"));
        // JsonArray mirrors the struct order: name, image, index
        let name = out.find("\"name\": \"a.png\"").unwrap();
        let image = out.find("\"image\": \"atlas-1.png\"").unwrap();
        let index = out.find("\"index\": 1").unwrap();
        assert!(name < image && image < index);
    }

    #[test]
    fn vars_are_exposed_to_template_context() {
        let rects = sample_rects();
        let mut vars = Vars::new();
        vars.insert("author".into(), serde_json::json!("me"));
        vars.insert("version".into(), serde_json::json!(2));
        let tpl = "author={{ vars.author | to_json }} version={{ vars.version }}";
        let out = render(tpl, &rects, &sample_info(false, 1, 0, ""), &vars, &[], &PackOptions::default()).unwrap();
        assert_eq!(out, "author=\"me\" version=2");
    }

    #[test]
    fn images_options_input_and_without_extname_are_exposed() {
        let rects = sample_rects();
        let vars = Vars::new();
        let images = vec![
            AtlasInfo { name: "atlas-0.png".into(), index: 0, width: 64, height: 64 },
            AtlasInfo { name: "atlas-1.png".into(), index: 1, width: 128, height: 32 },
        ];
        let opts = PackOptions {
            input: "dir/ghosthand.img".into(),
            ..Default::default()
        };
        // Custom template exercising images (files array), options.input | basename
        // (frames key) and without_extname (sprite base name as key), with index used
        // as the `source` field.
        let tpl = r#"{
  "files": [{% for img in images %}"{{ img.name }}"{% if not loop.last %},{% endif %}{% endfor %}],
  "frames": { "{{ options.input | basename }}": {
{% for r in sprites %}    "{{ r.name | without_extname }}": { "source": {{ r.index }} }{% if not loop.last %},{% endif %}
{% endfor %}  } }
}"#;
        let out = render(tpl, &rects, &sample_info(true, 2, 0, ""), &vars, &images, &opts).unwrap();
        assert!(out.contains("\"files\": [\"atlas-0.png\",\"atlas-1.png\"]"));
        assert!(out.contains("\"ghosthand.img\""));
        assert!(out.contains("\"a\": { \"source\": 1 }"));
        assert!(out.contains("\"b\": { \"source\": 2 }"));
        // frame/image/index are available on each sprite alongside the new context vars
        let out2 = render(
            "{% for r in sprites %}{{ r.image }} {{ r.index }};{% endfor %}",
            &rects,
            &sample_info(false, 1, 0, ""),
            &vars,
            &[],
            &PackOptions::default(),
        )
        .unwrap();
        assert_eq!(out2, "atlas-1.png 1;atlas-2.png 2;");
    }

    #[test]
    fn image_dict_lookup_by_name() {
        let rects = sample_rects();
        let vars = Vars::new();
        // Index starts at 1 (not 0), so the `images` array cannot be indexed by
        // r.index — atlas info must be looked up by the sprite's atlas name.
        let images = vec![
            AtlasInfo { name: "atlas-1.png".into(), index: 1, width: 64, height: 64 },
            AtlasInfo { name: "atlas-2.png".into(), index: 2, width: 128, height: 32 },
        ];
        let tpl = "{% for r in sprites %}{{ r.image }}:{{ image_dict[r.image].width }}x{{ image_dict[r.image].height }};{% endfor %}";
        let out = render(tpl, &rects, &sample_info(true, 2, 0, ""), &vars, &images, &PackOptions::default()).unwrap();
        assert_eq!(out, "atlas-1.png:64x64;atlas-2.png:128x32;");
    }

    #[test]
    fn basename_filter_extracts_last_path_segment() {
        let rects = sample_rects();
        let mut vars = Vars::new();
        vars.insert("win_path".into(), serde_json::json!("dir\\file.png"));
        let tpl = "{{ '/a/b/c' | basename }}|{{ 'c' | basename }}|{{ vars.win_path | basename }}";
        let out = render(tpl, &rects, &sample_info(false, 1, 0, ""), &vars, &[], &PackOptions::default()).unwrap();
        assert_eq!(out, "c|c|file.png");
    }

    #[test]
    fn options_are_exposed_to_template_context() {
        let rects = sample_rects();
        let vars = Vars::new();
        let opts = PackOptions {
            width: 1024,
            height: 512,
            padding: 2,
            single_config: true,
            ..Default::default()
        };
        let tpl = "{{ options.width }}x{{ options.height }} pad={{ options.padding }} single={{ options.single_config }} name={{ options.texture_name }}";
        let out = render(tpl, &rects, &sample_info(false, 1, 0, ""), &vars, &[], &opts).unwrap();
        assert_eq!(out, "1024x512 pad=2 single=true name=atlas");
    }

    #[test]
    fn exports_object_has_single_config_image_count_and_cur_image() {
        let rects = sample_rects();
        let vars = Vars::new();
        let images = vec![
            AtlasInfo { name: "atlas-0.png".into(), index: 0, width: 64, height: 64 },
            AtlasInfo { name: "atlas-1.png".into(), index: 1, width: 128, height: 32 },
            AtlasInfo { name: "atlas-2.png".into(), index: 2, width: 256, height: 16 },
        ];
        // Merged export over 3 sheets: exports.single_config=true, image_count=3.
        let tpl = "sc={{ exports.single_config }} n={{ exports.image_count }} cur={{ exports.cur_image_index }} w={{ image_dict[exports.cur_image].width }}";
        let out = render(tpl, &rects, &sample_info(true, 3, 0, "atlas-0.png"), &vars, &images, &PackOptions::default()).unwrap();
        assert_eq!(out, "sc=true n=3 cur=0 w=64");
        // Per-sheet export over a single sheet.
        let out = render(tpl, &rects, &sample_info(false, 1, 2, "atlas-2.png"), &vars, &images, &PackOptions::default()).unwrap();
        assert_eq!(out, "sc=false n=1 cur=2 w=256");
    }
}
