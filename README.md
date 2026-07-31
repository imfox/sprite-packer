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
    "removeFileExtension": false
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
