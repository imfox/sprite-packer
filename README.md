# sprite-packer

Rust 实现的合图（Sprite Atlas）打包工具。将目录下多张小图片打包成一张/多张大图，并输出 JSON 位置元数据。

算法与输出参考 [free-tex-packer-core](https://github.com/odrick/free-tex-packer-core) 与 npm [maxrects-packer](https://github.com/soimy/maxrects-packer)，目标是与参考实现保持结果一致。

## 功能特性

- **多打包器**：MaxRectsBin（6 种启发式方法）、MaxRectsPacker（npm 引擎 4 种模式）、OptimalPacker（自动尝试所有组合取最优）
- **自动多张合成**：超过单张 atlas 尺寸限制时自动生成多张 sheet
- **旋转**：允许精灵旋转 90° 以提升空间利用率
- **透明裁剪（Trim）**：自动去掉精灵四周的透明像素，可设 alpha 阈值
- **外扩（Extrude）**：扩展边缘像素，防止采样时的边缘渗色
- **重复检测（Detect Identical）**：内容完全相同的图片只渲染一份，其余复用
- **Power-of-Two**：将 atlas 尺寸强制对齐到 2 的幂
- **缩放（Scale）**：按比例缩放并选择缩放算法（BILINEAR 等）
- **滤镜（Filter）**：identity / grayscale / mask
- **配置文件**：JSON 配置文件，命令行参数优先
- **自定义模板**：MiniJinja 模板驱动导出，可输出任意位置数据格式（XML、Unity 等）
- **静默模式**：`-q` 不打印任何信息

## 支持格式

输入：`png`、`jpg`、`jpeg`、`bmp`（递归扫描目录）
输出：`png` + JSON（JsonHash / JsonArray）

## 构建

```bash
cargo build --release
# 生成的可执行文件：target/release/sprite-packer(.exe)
```

运行测试：

```bash
cargo test
```

## 命令行用法

```
sprite-packer -i <输入目录> -o <输出目录> [选项]
```

### 参数

| 参数 | 说明 | 默认值 |
|---|---|---|
| `-i, --input <DIR>` | 输入目录（递归扫描图片） | 必填 |
| `-o, --output <DIR>` | 输出目录 | 必填 |
| `--config <FILE>` | JSON 配置文件，命令行参数优先 | 无 |
| `--gen-config <FILE>` | 生成默认配置文件模板并退出 | 无 |
| `-q, --quiet` | 静默模式，不打印任何信息（错误仍输出到 stderr） | 关闭 |
| `--texture-name <NAME>` | 输出文件基础名 | `atlas` |
| `--sheet-start-index <N>` | 多 sheet 文件名的起始序号（如 `atlas-0`、`atlas-1`） | `0` |
| `--single-config [true/false]` | 多图集时合并为一个配置（每个精灵带 `image` 字段）；`false` 时每个图集各生成一份配置 | `false` |
| `--width <N>` | 单张 atlas 最大宽度 | `2048` |
| `--height <N>` | 单张 atlas 最大高度 | `2048` |
| `--power-of-two` | 强制 Power-of-Two 尺寸 | `false` |
| `--padding <N>` | 精灵间像素间距 | `0` |
| `--extrude <N>` | 边缘外扩像素 | `0` |
| `--allow-rotation [true/false]` | 允许旋转 | `true` |
| `--trim [true/false]` | 启用透明裁剪 | `true` |
| `--alpha-threshold <N>` | 裁剪 alpha 阈值 (0-255) | `0` |
| `--packer <NAME>` | 打包器 | `MaxRectsBin` |
| `--packer-method <NAME>` | 打包方法 | `BestShortSideFit` |
| `--exporter <NAME>` | 导出格式 | `JsonHash` |
| `--remove-file-extension [true/false]` | 精灵名去掉扩展名 | `false` |
| `--filter <NAME>` | 位图滤镜 | `none` |
| `--scale <F>` | 缩放因子 | `1.0` |
| `--template <FILE>` | 自定义 MiniJinja 模板文件，覆盖内置导出模板（见下文「自定义模板」） | 无 |
| `--template-extension <EXT>` | 自定义模板输出的元数据文件扩展名 | 取 exporter 默认（`json`） |
| `--vars <KEY=VALUE>...` | 传给模板的额外键值对，模板中以 `{{ vars.<key> }}` 访问；值尽量按 JSON 解析（`2`、`true`、`{"a":1}`），否则作为字符串。可重复：`--vars a=1 --vars b=2`，或一个参数带多对：`--vars a=1 b=2` | 无 |

### 打包器与方法

- `--packer MaxRectsBin`：经典 MaxRects 算法
  - 方法：`BestShortSideFit`（默认）、`BestLongSideFit`、`BestAreaFit`、`BottomLeftRule`、`ContactPointRule`、`FillWidth`
- `--packer MaxRectsPacker`：npm maxrects-packer 引擎
  - 方法：`Smart`、`SmartArea`、`Square`、`SquareArea`
- `--packer OptimalPacker`：遍历所有打包器 × 方法 × 旋转组合，选 sheet 数最少、空间利用率最高的结果

### 示例

```bash
# 基本用法
sprite-packer -i ./images -o ./out

# 指定尺寸、间距与外扩
sprite-packer -i ./images -o ./out --width 1024 --height 1024 --padding 2 --extrude 1

# 使用配置文件（命令行参数优先）
sprite-packer --config my.config.json

# 生成默认配置模板
sprite-packer --gen-config my.config.json

# 静默打包
sprite-packer -i ./images -o ./out -q

# 使用自定义模板（Starling XML 示例）
sprite-packer -i ./images -o ./out --template starling.tpl --template-extension xml
```

## 配置文件

配置为 JSON 格式，**键名使用 camelCase**（与 free-tex-packer-core 一致），同时兼容 kebab-case（如 `power-of-two`）。

**优先级**：命令行显式传入的参数 > 配置文件 > 默认值。

生成默认模板：

```bash
sprite-packer --gen-config default.json
```

示例配置：

```json
{
    "input": "./images",
    "output": "./out",
    "textureName": "atlas",
    "suffix": "-",
    "sheetStartIndex": 0,
    "singleConfig": false,
    "powerOfTwo": false,
    "fixedSize": false,
    "width": 2048,
    "height": 2048,
    "padding": 1,
    "extrude": 0,
    "allowRotation": false,
    "detectIdentical": true,
    "allowTrim": true,
    "alphaThreshold": 0,
    "scale": 1.0,
    "scaleMethod": "BILINEAR",
    "packer": "OptimalPacker",
    "packerMethod": "SmartArea",
    "exporter": "JsonHash",
    "filter": "none",
    "textureFormat": "png",
    "removeFileExtension": false,
    "template": "",
    "templateExtension": "",
    "vars": {}
}
```

所有键均可选。

## 输出格式

### JsonHash（默认）

TexturePacker 风格的哈希索引：

```json
{
    "file.png": {
        "frame": { "x": 0, "y": 0, "w": 100, "h": 100 },
        "rotated": false,
        "trimmed": false,
        "spriteSourceSize": { "x": 0, "y": 0, "w": 100, "h": 100 },
        "sourceSize": { "w": 100, "h": 100 }
    },
    "...": { "...": "..." }
}
```

### JsonArray

与 JsonHash 相同的数据，但输出为数组形式（TexturePacker `--texture-format JSONArray` 风格）。

### 多图集配置导出

碎图较多、超过单张图集尺寸上限时，会自动拆分为多张图集（`atlas-0.png`、`atlas-1.png`…）。

**默认（`--single-config` 关闭，常见情况）**：每张图集各生成一份配置，配置只包含该图集自己的精灵，`name` 指向对应的图集文件。N 张图集产生 N 份配置（`atlas-0.json`、`atlas-1.json`…）。单张图集的配置格式如下：

```json
{
    "name": "atlas-0.png",
    "sprites": [
        { "frame": { "x": 0, "y": 0, "w": 100, "h": 100 }, "name": "35.png", "rotated": false, "trimmed": true, "spriteSourceSize": { "x": 299, "y": 266, "w": 31, "h": 28 }, "sourceSize": { "w": 500, "h": 500 } },
        { "frame": { "x": 0, "y": 100, "w": 80, "h": 80 }, "name": "204.png", "rotated": false, "trimmed": true, "spriteSourceSize": { "x": 200, "y": 209, "w": 30, "h": 28 }, "sourceSize": { "w": 500, "h": 500 } }
    ]
}
```

**合并为一份（可选）**：设置 `--single-config` 时，所有图集的精灵合并进**一个**配置文件（`atlas.json`），每个精灵额外带 `image`（所属图集文件名）与 `index`（图集序号，配合 `--sheet-start-index`）字段标明所属图集：

```json
{
    "name": "atlas.png",
    "sprites": [
        { "frame": { "x": 0, "y": 0, "w": 100, "h": 100 }, "image": "atlas-0.png", "index": 0, "name": "35.png", "rotated": false, "trimmed": true, "spriteSourceSize": { "x": 299, "y": 266, "w": 31, "h": 28 }, "sourceSize": { "w": 500, "h": 500 } },
        { "frame": { "x": 0, "y": 100, "w": 80, "h": 80 }, "image": "atlas-1.png", "index": 1, "name": "204.png", "rotated": false, "trimmed": true, "spriteSourceSize": { "x": 200, "y": 209, "w": 30, "h": 28 }, "sourceSize": { "w": 500, "h": 500 } }
    ]
}
```

单图集（只有一张）时该参数无影响（本来就只有一个配置）。

## 自定义模板（MiniJinja）

内置的 JsonHash / JsonArray 使用 [MiniJinja](https://github.com/mitsuhiko/minijinja)（Jinja2 语法）模板生成。你可以传入自己的模板文件，把元数据输出成任意格式（XML、Unity、自定义文本等）。

```bash
sprite-packer -i ./images -o ./out --template starling.tpl --template-extension xml
```

- `--template` 指向一个 MiniJinja 模板文件，渲染结果替代内置导出器模板的输出
- `--template-extension` 指定元数据输出文件的后缀（默认沿用 exporter 的 `json`）
- `--vars key=value ...` 传入额外键值对，模板中用 `{{ vars.key }}` 读取（值按 JSON 解析，数字/布尔/对象均可；字符串建议配合 `| to_json` 写 JSON 值）
- 模板文件读取失败或模板语法错误时，程序报错并以非零码退出

```bash
# 向模板传入自定义字段
sprite-packer -i ./images -o ./out --template my.tpl \
  --vars author=me version=2 "publish=true"
```

### 模板上下文变量

渲染时提供以下变量：

| 变量 | 类型 | 说明 |
|---|---|---|
| `single_config` | 布尔 | 本次导出是否为合并模式：`true` 时所有图集合并为一份配置（每个精灵带 `image`、`index` 字段），`false` 时每个图集各生成一份配置 |
| `sprites` | 数组 | 每个精灵一个对象，字段见下表 |
| `images` | 数组 | 每张图集一个对象，按 sheet 顺序排列：`name`（图集文件名，如 `atlas-0.png`）、`index`（图集序号）、`width`、`height`（渲染后图集的宽高） |
| `input_dir` | 字符串 | 输入目录的基础名（如输入 `images/back.img` 则为 `back.img`），常用来做 `frames` 之类的分组键 |
| `options` | 对象 | 本次打包的全部生成参数（`PackOptions`，snake_case 键），如 `options.single_config`、`options.width`、`options.padding`、`options.allow_rotation`。注意其中的 `options.single_config` 是用户传入的合并选项；顶层 `single_config` 表示本次导出是否为合并模式 |
| `vars` | 对象 | `--vars` 或配置 `vars` 传入的额外键值对，`{{ vars.<key> }}` 访问（值按 JSON 解析时可为数字/布尔/对象/数组） |

`sprites` 中每个元素 `r` 的字段：

| 字段 | 类型 | 说明 |
|---|---|---|
| `r.name` | 字符串 | 精灵文件名（`removeFileExtension` 为 true 时去掉扩展名） |
| `r.image` | 字符串 | 所属图集文件名（如 `atlas-0.png`） |
| `r.index` | 整数 | 所属图集序号（如 `0` 对应 `atlas-0.png`，随 `--sheet-start-index` 偏移） |
| `r.frame` | `{x, y, w, h}` | 精灵在 atlas 中的位置与尺寸 |
| `r.rotated` | 布尔 | 是否旋转 90° |
| `r.trimmed` | 布尔 | 是否裁剪过透明边 |
| `r.sprite_source_size` | `{x, y, w, h}` | 裁剪后的内容在原图中的区域 |
| `r.source_size` | `{w, h}` | 原图完整尺寸 |

模板语法遵循 [MiniJinja](https://jinja.palletsprojects.com/en/stable/templates/) 官方文档。本工具额外注册了两个过滤器：

- `to_json`：把值编码为 JSON 字面量（字符串自动转义引号，写 JSON 的字符串值时务必使用，如 `{{ r.name | to_json }}`）
- `strip_ext`：去掉文件名的最后一个扩展名（`x.png` → `x`），可用作 `frames` 的键名：`{{ r.name | strip_ext | to_json }}`

### 示例（Starling XML）

完整示例见 `testunit/starling.tpl`：

```xml
<?xml version="1.0" encoding="UTF-8"?>
<TextureAtlas imagePath="{{ options.texture_name }}.png">
{% for r in sprites %}	<SubTexture name="{{ r.name }}" x="{{ r.frame.x }}" y="{{ r.frame.y }}" width="{{ r.frame.w }}" height="{{ r.frame.h }}"{% if r.rotated %} rotated="true"{% endif %}{% if r.trimmed %} frameX="{{ r.sprite_source_size.x }}" frameY="{{ r.sprite_source_size.y }}" frameWidth="{{ r.source_size.w }}" frameHeight="{{ r.source_size.h }}"{% endif %}/>
{% endfor %}</TextureAtlas>
```

### 默认模板参考

内置 JsonHash 模板（`{% if single_config %}` 段仅在合并模式下输出 `image` 与 `index` 字段，每个图集单独配置时保持纯净）：

```jinja
{
  "name": "{{ options.texture_name }}.png",
  "sprites": [
{% for r in sprites %}    {
      "frame": {
        "h": {{ r.frame.h }},
        "w": {{ r.frame.w }},
        "x": {{ r.frame.x }},
        "y": {{ r.frame.y }}
      },
{% if single_config %}      "image": {{ r.image | to_json }},
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
}
```

内置 JsonArray 模板：

```jinja
[
{% for r in sprites %}  {
    "name": {{ r.name | to_json }},
{% if single_config %}    "image": {{ r.image | to_json }},
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
{% endfor %}]
```

## 项目结构

```
src/
├── main.rs              # CLI 入口（clap 参数解析、配置文件合并）
├── lib.rs               # 公共 API：pack() / scan_dir()
├── pack_processor.rs    # PackProcessor 编排（打包流程、OptimalPacker 组合枚举）
├── files_processor.rs   # FilesProcessor 输出文件
├── math/
│   └── rect.rs          # Rect 结构
├── packers/
│   ├── maxrects_bin.rs  # MaxRectsBin（5+1 种方法 + 旋转）
│   ├── maxrects_packer.rs # MaxRectsPacker
│   ├── npm_bin.rs       # npm maxrects-packer 引擎移植
│   └── optimal_packer.rs # OptimalPacker
├── exporters/
│   └── mod.rs           # 导出器（JsonHash / JsonArray）
├── filters/
│   ├── identity.rs      # 无操作滤镜
│   ├── grayscale.rs     # 灰度
│   └── mask.rs          # 透明度掩码
└── utils/
    ├── trimmer.rs       # 透明裁剪
    └── texture_renderer.rs # atlas 渲染（贴图、外扩、缩放）
```

## 作为库使用

```rust
use sprite_packer::{pack, scan_dir, PackOptions};

let sources = scan_dir("./images")?;
let options = PackOptions {
    texture_name: "atlas".into(),
    width: 2048,
    height: 2048,
    ..Default::default()
};
let results = pack(&sources, &options)?; // Vec<PackResult { name, buffer }>
```
