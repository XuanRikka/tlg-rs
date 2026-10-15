# tlg-rs
一个纯Rust实现的高性能tlg格式编解码库。

## 什么是tlg
tlg是视觉小说引擎Kirikiri2(krkr2)内置的一种图片格式，常见有tlg5和tlg6两个版本。官方实现可参考[tlg-wic-codec](https://github.com/krkrz/tlg-wic-codec)。

## 这个项目做了什么
- 将原 C/C++ 实现完全用 Rust 重写，保证内存安全的前提下提供更高的性能；
- 重新设计 API，使用更符合 Rust 习惯的接口；
- 可选集成 [image-rs](https://github.com/image-rs/image) 生态，方便图片处理。

## 快速开始

### 添加依赖

```bash
# 默认启用 image 特性，方便与 image-rs 生态集成
cargo add tlg-rs

# 仅使用基础编解码功能，不引入 image-rs
cargo add tlg-rs --no-default-features
```

### 示例代码段
#### 示例：使用TlgReader/TlgWriter
```rust
use std::fs::File;
use std::io::BufReader;
use tlg::{TlgReader, TlgType, TlgWriter};

fn main() {
    // 读取 tlg 文件
    let tlg = File::open("aaa.tlg").unwrap();
    let reader = TlgReader::from_reader(BufReader::new(tlg));
    let (image, tags) = reader.read_to_image().unwrap();

    // 重新编码为 tlg6 并写入新文件
    let mut tlg2 = File::create("bbb.tlg").unwrap();
    let writer = TlgWriter::from_image(&image, tags, TlgType::Tlg6).unwrap();
    writer.write_to(&mut tlg2).unwrap();
}
```
TlgReader会自动识别tlg5 /tlg6格式，tags为tlg文件中可能存在的元信息。

#### 示例：直接使用编解码器
```rust
use std::fs;
use std::fs::File;
use std::io::BufReader;
use tlg::tlg6::{Tlg6Decoder, Tlg6Encoder};
use tlg::{TlgDecoderTrait, TlgEncoderTrait};

fn main() {
    // 解码 tlg6 文件
    let tlg = File::open("aaa.tlg").unwrap();
    let decoder = Tlg6Decoder::from_reader(BufReader::new(tlg)).unwrap();
    let image = decoder.decode_to_image().unwrap();

    // 编码为 tlg6
    let encoder = Tlg6Encoder::from_image(&image).unwrap();
    let encode_data = encoder.encode().unwrap();

    fs::write("bbb.tlg", encode_data).unwrap();
}
```
> 注意：Tlg6Decoder/Tlg6Encoder和tlg5的版本均不支持包含元信息（tags）的tlg格式，如需处理tags请使用TlgReader/TlgWriter。

把 `tlg::tlg6` 换成 `tlg::tlg5` 就是 tlg5 的用法。解码器实现了 `TlgDecoderTrait`（`from_path` / `from_reader` / `from_data` / `decode` / `decode_to_image`），编码器实现了 `TlgEncoderTrait`（`from_image` / `from_gray` / `from_rgb` / `from_rgba` / `from_raw` / `encode` / `encode_to`，以及 `width` / `height` / `pixel_layout`），所以 `Tlg6Decoder::from_path("aaa.tlg")`、`Tlg6Encoder::from_rgb(data, w, h)` 这类构造函数都可以直接用。

## 命令行工具
`tlg-rs-cli/` 提供 `tlg-rs` 命令，支持常见图片格式与 tlg 互转（依赖 image-rs 的全部格式特性）：

```bash
# 编码：PNG/JPEG/BMP… -> tlg，-v 指定版本（默认 6）
tlg-rs encode input.png output.tlg -v 5
tlg-rs encode input.png output.tlg -v 6

# 解码：tlg -> PNG/JPEG/BMP…，版本自动识别
tlg-rs decode output.tlg restored.png
```

## 鸣谢
 - KrKr2引擎开发者

## 许可证
本项目使用MIT开源
