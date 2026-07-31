use clap::parser::{ArgMatches, ValueSource};
use clap::{CommandFactory, FromArgMatches, Parser};
use serde::{Deserialize, Serialize};

use sprite_packer::{pack, scan_dir, PackOptions};

/// JSON config file. All fields are optional — a value given on the command line
/// overrides the same key here.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ConfigFile {
    input: Option<String>,
    output: Option<String>,
    #[serde(alias = "texture-name")]
    texture_name: Option<String>,
    suffix: Option<String>,
    #[serde(alias = "sheet-start-index")]
    sheet_start_index: Option<u32>,
    #[serde(alias = "multi-config")]
    multi_config: Option<bool>,
    #[serde(alias = "power-of-two")]
    power_of_two: Option<bool>,
    #[serde(alias = "fixed-size")]
    fixed_size: Option<bool>,
    width: Option<u32>,
    height: Option<u32>,
    padding: Option<u32>,
    extrude: Option<u32>,
    #[serde(alias = "allow-rotation")]
    allow_rotation: Option<bool>,
    #[serde(alias = "detect-identical")]
    detect_identical: Option<bool>,
    #[serde(alias = "allow-trim")]
    allow_trim: Option<bool>,
    #[serde(alias = "alpha-threshold")]
    alpha_threshold: Option<u8>,
    scale: Option<f32>,
    #[serde(alias = "scale-method")]
    scale_method: Option<String>,
    packer: Option<String>,
    #[serde(alias = "packer-method")]
    packer_method: Option<String>,
    exporter: Option<String>,
    filter: Option<String>,
    #[serde(alias = "texture-format")]
    texture_format: Option<String>,
    #[serde(alias = "remove-file-extension")]
    remove_file_extension: Option<bool>,
    template: Option<String>,
    #[serde(alias = "template-extension")]
    template_extension: Option<String>,
    /// Extra key-value variables exposed to the template context as `vars.<key>`.
    vars: Option<std::collections::HashMap<String, serde_json::Value>>,
}

#[derive(Parser)]
#[command(name = "sprite-packer", about = "合图工具 — Pack images into a sprite atlas")]
struct Cli {
    /// Input directory containing images (scanned recursively)
    #[arg(short = 'i', long = "input", value_name = "DIR")]
    input: Option<String>,

    /// Output directory for atlas files
    #[arg(short = 'o', long = "output", value_name = "DIR")]
    output: Option<String>,

    /// JSON config file with packing options. Command-line args take priority.
    #[arg(long = "config", value_name = "FILE")]
    config: Option<String>,

    /// Write a default config template to FILE and exit (for easy editing).
    #[arg(long = "gen-config", value_name = "FILE")]
    gen_config: Option<String>,

    /// Suppress all informational output (errors still go to stderr)
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,

    /// Base name for output files
    #[arg(long = "texture-name", default_value = "atlas")]
    texture_name: String,

    /// Starting index for multi-sheet file names (default 0 → atlas-0, atlas-1)
    #[arg(long = "sheet-start-index", default_value = "0")]
    sheet_start_index: u32,

    /// One metadata file per sheet (--multi-config or --multi-config false).
    /// Set to false to merge all sheets into a single metadata file.
    #[arg(long = "multi-config", num_args = 0..=1, default_missing_value = "true", default_value_t = true)]
    multi_config: bool,

    /// Max atlas width per sheet
    #[arg(long = "width", default_value = "2048")]
    width: u32,

    /// Max atlas height per sheet
    #[arg(long = "height", default_value = "2048")]
    height: u32,

    /// Force power-of-two dimensions
    #[arg(long = "power-of-two")]
    power_of_two: bool,

    /// Padding between sprites in pixels
    #[arg(long = "padding", default_value = "0")]
    padding: u32,

    /// Edge extrusion in pixels
    #[arg(long = "extrude", default_value = "0")]
    extrude: u32,

    /// Allow sprite rotation (--allow-rotation or --allow-rotation false)
    #[arg(long = "allow-rotation", num_args = 0..=1, default_missing_value = "true", default_value_t = true)]
    allow_rotation: bool,

    /// Enable transparency trimming (--trim or --trim false)
    #[arg(long = "trim", num_args = 0..=1, default_missing_value = "true", default_value_t = true)]
    trim: bool,

    /// Alpha threshold for trimming (0-255)
    #[arg(long = "alpha-threshold", default_value = "0")]
    alpha_threshold: u8,

    /// Packer algorithm (MaxRectsBin, MaxRectsPacker, OptimalPacker)
    #[arg(long = "packer", default_value = "MaxRectsBin")]
    packer: String,

    /// Packer method: BestShortSideFit, BestLongSideFit, BestAreaFit, BottomLeftRule, ContactPointRule
    #[arg(long = "packer-method", default_value = "BestShortSideFit")]
    packer_method: String,

    /// Exporter format: JsonHash, JsonArray
    #[arg(long = "exporter", default_value = "JsonHash")]
    exporter: String,

    /// Remove file extension from sprite names (--remove-file-extension or --remove-file-extension false)
    #[arg(long = "remove-file-extension", num_args = 0..=1, default_missing_value = "true", default_value_t = false)]
    remove_file_extension: bool,

    /// Bitmap filter: none, grayscale, mask
    #[arg(long = "filter", default_value = "none")]
    filter: String,

    /// Scale factor
    #[arg(long = "scale", default_value = "1.0")]
    scale: f32,

    /// Custom MiniJinja template file for metadata export (overrides the exporter's
    /// built-in template). See README for the context variables available.
    #[arg(long = "template", value_name = "FILE")]
    template: Option<String>,

    /// Output file extension for the metadata when a custom template is used
    #[arg(long = "template-extension", value_name = "EXT")]
    template_extension: Option<String>,

    /// Extra key=value variables exposed to the template context as `vars.<key>`.
    /// Values are parsed as JSON when possible (2, true, {"a": 1}), else kept as
    /// plain strings. May be repeated: `--vars a=1 --vars b=2`, or one flag with
    /// multiple pairs: `--vars a=1 b=2`.
    #[arg(long = "vars", value_name = "KEY=VALUE", num_args = 1..)]
    vars: Vec<String>,
}

/// Default config template written by --gen-config.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DefaultConfig {
    input: &'static str,
    output: &'static str,
    texture_name: &'static str,
    suffix: &'static str,
    sheet_start_index: u32,
    multi_config: bool,
    power_of_two: bool,
    fixed_size: bool,
    width: u32,
    height: u32,
    padding: u32,
    extrude: u32,
    allow_rotation: bool,
    detect_identical: bool,
    allow_trim: bool,
    alpha_threshold: u8,
    scale: f32,
    scale_method: &'static str,
    packer: &'static str,
    packer_method: &'static str,
    exporter: &'static str,
    filter: &'static str,
    texture_format: &'static str,
    remove_file_extension: bool,
    template: &'static str,
    template_extension: &'static str,
    vars: std::collections::HashMap<String, serde_json::Value>,
}

/// Defaults mirror PackOptions::default() plus the input/output placeholders.
fn default_config() -> DefaultConfig {
    DefaultConfig {
        input: "",
        output: "",
        texture_name: "atlas",
        suffix: "-",
        sheet_start_index: 0,
        multi_config: true,
        power_of_two: false,
        fixed_size: false,
        width: 2048,
        height: 2048,
        padding: 0,
        extrude: 0,
        allow_rotation: true,
        detect_identical: true,
        allow_trim: true,
        alpha_threshold: 0,
        scale: 1.0,
        scale_method: "BILINEAR",
        packer: "MaxRectsBin",
        packer_method: "BestShortSideFit",
        exporter: "JsonHash",
        filter: "none",
        texture_format: "png",
        remove_file_extension: false,
        template: "",
        template_extension: "",
        vars: std::collections::HashMap::new(),
    }
}

/// Write a default config template to `path`. Refuses to overwrite an existing file.
fn gen_config(path: &str, quiet: bool) {
    let content = serde_json::to_string_pretty(&default_config()).unwrap();
    let content = format!("{}\n", content);
    if std::path::Path::new(path).exists() {
        eprintln!("Error: '{}' already exists — refusing to overwrite (delete it first to regenerate).", path);
        std::process::exit(1);
    }
    if let Err(e) = std::fs::write(path, content) {
        eprintln!("Error writing config template to '{}': {}", path, e);
        std::process::exit(1);
    }
    if !quiet {
        println!("Default config template written to: {}", path);
    }
}

fn main() {
    // 没有任何参数时等价于 --help（stdout 输出、exit 0）
    if std::env::args_os().nth(1).is_none() {
        let mut cmd = Cli::command();
        let _ = cmd.print_help();
        println!();
        return;
    }

    let matches = Cli::command().get_matches();
    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());

    // --gen-config writes a template and exits without packing
    if let Some(path) = &cli.gen_config {
        gen_config(path, cli.quiet);
        return;
    }

    let cfg = load_config(cli.config.as_deref());

    // Resolve input/output: command line wins over config file
    let input = match cli.input.clone().or_else(|| cfg.input.clone()) {
        Some(p) => p,
        None => {
            eprintln!("Error: missing input directory (use --input or a config \"input\" key)");
            std::process::exit(1);
        }
    };
    let output = match cli.output.clone().or_else(|| cfg.output.clone()) {
        Some(p) => p,
        None => {
            eprintln!("Error: missing output directory (use --output or a config \"output\" key)");
            std::process::exit(1);
        }
    };

    let options = match build_options(&matches, &cli, &cfg) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    // 1. Scan directory
    if !cli.quiet {
        println!("Scanning: {} ...", input);
    }
    let sources = match scan_dir(&input) {
        Ok(s) => {
            if s.is_empty() {
                eprintln!("No supported images found in '{}'", input);
                std::process::exit(1);
            }
            if !cli.quiet {
                println!("  Found {} images", s.len());
            }
            s
        }
        Err(e) => {
            eprintln!("Error scanning '{}': {}", input, e);
            std::process::exit(1);
        }
    };

    // 2. Pack
    if !cli.quiet {
        println!(
            "Packing (max {}x{}, pad={}, extrude={}) ...",
            options.width, options.height, options.padding, options.extrude
        );
    }
    let results = match pack(&sources, &options) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Packing failed: {}", e);
            std::process::exit(1);
        }
    };

    // 3. Ensure output directory exists
    let out_dir = std::path::Path::new(&output);
    if let Err(e) = std::fs::create_dir_all(out_dir) {
        eprintln!("Cannot create output directory '{}': {}", output, e);
        std::process::exit(1);
    }

    // 4. Write output files
    for res in &results {
        let file_path = out_dir.join(&res.name);
        match std::fs::write(&file_path, &res.buffer) {
            Ok(_) => {
                if !cli.quiet {
                    println!("  Saved: {}", file_path.display());
                }
            }
            Err(e) => eprintln!("  Failed to write {}: {}", file_path.display(), e),
        }
    }

    if !cli.quiet {
        println!("Done! {} files written.", results.len());
    }
}

/// Read and parse the --config file, or return an empty config when absent.
fn load_config(path: Option<&str>) -> ConfigFile {
    let Some(path) = path else {
        return ConfigFile::default();
    };
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error reading config file '{}': {}", path, e);
            std::process::exit(1);
        }
    };
    match serde_json::from_str(&text) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error parsing config file '{}': {}", path, e);
            std::process::exit(1);
        }
    }
}

/// Merge CLI args and config file into PackOptions. For every key, the command line
/// wins when the user explicitly typed it; otherwise the config file value is used,
/// falling back to the PackOptions default.
fn build_options(matches: &ArgMatches, cli: &Cli, cfg: &ConfigFile) -> Result<PackOptions, String> {
    let d = PackOptions::default();
    let cli_wins = |name: &str| matches.value_source(name) == Some(ValueSource::CommandLine);

    Ok(PackOptions {
        texture_name: if cli_wins("texture_name") {
            cli.texture_name.clone()
        } else {
            cfg.texture_name.clone().unwrap_or(d.texture_name)
        },
        suffix: cfg.suffix.clone().unwrap_or(d.suffix),
        sheet_start_index: if cli_wins("sheet_start_index") {
            cli.sheet_start_index
        } else {
            cfg.sheet_start_index.unwrap_or(d.sheet_start_index)
        },
        multi_config: if cli_wins("multi_config") {
            cli.multi_config
        } else {
            cfg.multi_config.unwrap_or(d.multi_config)
        },
        width: if cli_wins("width") {
            cli.width
        } else {
            cfg.width.unwrap_or(d.width)
        },
        height: if cli_wins("height") {
            cli.height
        } else {
            cfg.height.unwrap_or(d.height)
        },
        power_of_two: if cli_wins("power_of_two") {
            cli.power_of_two
        } else {
            cfg.power_of_two.unwrap_or(d.power_of_two)
        },
        fixed_size: cfg.fixed_size.unwrap_or(d.fixed_size),
        padding: if cli_wins("padding") {
            cli.padding
        } else {
            cfg.padding.unwrap_or(d.padding)
        },
        extrude: if cli_wins("extrude") {
            cli.extrude
        } else {
            cfg.extrude.unwrap_or(d.extrude)
        },
        allow_rotation: if cli_wins("allow_rotation") {
            cli.allow_rotation
        } else {
            cfg.allow_rotation.unwrap_or(d.allow_rotation)
        },
        detect_identical: cfg.detect_identical.unwrap_or(d.detect_identical),
        allow_trim: if cli_wins("trim") {
            cli.trim
        } else {
            cfg.allow_trim.unwrap_or(d.allow_trim)
        },
        alpha_threshold: if cli_wins("alpha_threshold") {
            cli.alpha_threshold
        } else {
            cfg.alpha_threshold.unwrap_or(d.alpha_threshold)
        },
        scale: if cli_wins("scale") {
            cli.scale
        } else {
            cfg.scale.unwrap_or(d.scale)
        },
        scale_method: cfg.scale_method.clone().unwrap_or(d.scale_method),
        packer: if cli_wins("packer") {
            cli.packer.clone()
        } else {
            cfg.packer.clone().unwrap_or(d.packer)
        },
        packer_method: if cli_wins("packer_method") {
            cli.packer_method.clone()
        } else {
            cfg.packer_method.clone().unwrap_or(d.packer_method)
        },
        exporter: if cli_wins("exporter") {
            cli.exporter.clone()
        } else {
            cfg.exporter.clone().unwrap_or(d.exporter)
        },
        filter: if cli_wins("filter") {
            cli.filter.clone()
        } else {
            cfg.filter.clone().unwrap_or(d.filter)
        },
        texture_format: cfg.texture_format.clone().unwrap_or(d.texture_format),
        remove_file_extension: if cli_wins("remove_file_extension") {
            cli.remove_file_extension
        } else {
            cfg.remove_file_extension.unwrap_or(d.remove_file_extension)
        },
        template: if cli_wins("template") {
            cli.template.clone()
        } else {
            cfg.template.clone().or(d.template)
        },
        template_extension: if cli_wins("template_extension") {
            cli.template_extension.clone()
        } else {
            cfg.template_extension.clone().or(d.template_extension)
        },
        vars: merge_vars(cfg, cli)?,
    })
}

/// Merge config-file `vars` with command-line `KEY=VALUE` pairs. CLI keys override
/// config keys one by one. CLI values are parsed as JSON when possible (2, true,
/// {"a": 1}), otherwise kept as plain strings.
fn merge_vars(cfg: &ConfigFile, cli: &Cli) -> Result<std::collections::HashMap<String, serde_json::Value>, String> {
    let mut vars = cfg.vars.clone().unwrap_or_default();
    for kv in &cli.vars {
        let (k, v) = kv
            .split_once('=')
            .ok_or_else(|| format!("Invalid --vars value '{}' — expected KEY=VALUE", kv))?;
        if k.is_empty() {
            return Err(format!("Invalid --vars value '{}' — empty key", kv));
        }
        let val = serde_json::from_str::<serde_json::Value>(v)
            .unwrap_or_else(|_| serde_json::Value::String(v.to_string()));
        vars.insert(k.to_string(), val);
    }
    Ok(vars)
}
