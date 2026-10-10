//! TLG5 编解码基准测试（criterion）。
//!
//! 输入固定为 benches/test.jpg，计时区间内只包含编解码本身；图片读取、像素克隆、
//! 码流复制等准备工作都放在 iter_batched 的 setup 里，不计入测量。
//!
//! 吞吐量口径是像素：criterion 的 thrpt 行单位为 Elements/s，也就是像素/秒，
//! 其中 Melem/s 即百万像素/秒（MPixel/s）。
//!
//! 运行方式：
//!
//!     cargo bench --bench tlg5              # 完整基准
//!     cargo bench --bench tlg5 -- --quick   # 快速冒烟：采样更少、时间更短
//!     cargo bench --bench tlg5 -- encode    # 只跑名字里含 encode 的用例

use std::hint::black_box;
use std::path::PathBuf;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use tlg::tlg5::{Tlg5Decoder, Tlg5Encoder};
use tlg::{PixelLayout, TlgDecoderTrait, TlgEncoderTrait};

/// 测试图片路径，基于 crate 根目录定位，保证在任意工作目录下运行都成立。
fn test_image_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("benches")
        .join("test.jpg")
}

/// 在计时区之外准备好的基准输入。
struct Input {
    /// 用例标识，形如 534x510。
    id: String,
    width: u32,
    height: u32,
    /// 未压缩的 RGB8 像素，长度为 width * height * 3。
    rgb: Vec<u8>,
    /// 与 rgb 对应的 TLG5 码流，供解码与往返基准复用。
    encoded: Vec<u8>,
}

impl Input {
    /// 读取测试图片，并预先编码一份 TLG5 码流。
    ///
    /// 同时做一次编解码闭环自检：如果库本身有问题，基准应当直接报错，
    /// 而不是安静地测出一堆没有意义的数字。
    fn load() -> Self {
        let path = test_image_path();
        let image = image::open(&path)
            .unwrap_or_else(|e| panic!("无法打开测试图片 {}: {e}", path.display()))
            .to_rgb8();
        let (width, height) = image.dimensions();
        let rgb = image.into_raw();

        let encoded = Tlg5Encoder::from_rgb(rgb.clone(), width, height)
            .encode()
            .expect("TLG5 预编码失败");

        let (decoded, info) = Tlg5Decoder::from_data(encoded.clone())
            .expect("构造 TLG5 解码器失败")
            .decode()
            .expect("TLG5 预解码失败");
        assert_eq!((info.width, info.height), (width, height), "解码尺寸与输入不一致");
        assert_eq!(info.pixel_layout, PixelLayout::Rgb, "解码像素格式与输入不一致");
        assert_eq!(decoded, rgb, "TLG5 编解码不是无损的");

        println!(
            "TLG5 基准输入: {} ({}x{}, {} 字节 RGB, 码流 {} 字节)",
            path.display(),
            width,
            height,
            rgb.len(),
            encoded.len()
        );

        Input {
            id: format!("{width}x{height}"),
            width,
            height,
            rgb,
            encoded,
        }
    }

    /// 像素总数，作为吞吐量的计量单位。
    fn pixels(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }
}

/// 编码：RGB8 原始像素 -> TLG5 码流。
fn bench_encode(c: &mut Criterion) {
    let input = Input::load();
    let mut group = c.benchmark_group("tlg5/encode");
    group.throughput(Throughput::Elements(input.pixels()));

    group.bench_with_input(BenchmarkId::from_parameter(&input.id), &input, |b, input| {
        b.iter_batched(
            // 编码器按值取得像素数据，因此每次迭代都要准备一份新输入；
            // iter_batched 的 setup 不计入测量时间。
            || Tlg5Encoder::from_rgb(input.rgb.clone(), input.width, input.height),
            |encoder| black_box(encoder.encode().expect("TLG5 编码失败")),
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// 解码：TLG5 码流 -> RGB8 原始像素。
fn bench_decode(c: &mut Criterion) {
    let input = Input::load();
    let mut group = c.benchmark_group("tlg5/decode");
    group.throughput(Throughput::Elements(input.pixels()));

    group.bench_with_input(BenchmarkId::from_parameter(&input.id), &input, |b, input| {
        b.iter_batched(
            || Tlg5Decoder::from_data(input.encoded.clone()).expect("构造 TLG5 解码器失败"),
            |decoder| black_box(decoder.decode().expect("TLG5 解码失败")),
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// 往返：编码后立刻解码，衡量端到端的像素吞吐量。
fn bench_roundtrip(c: &mut Criterion) {
    let input = Input::load();
    let mut group = c.benchmark_group("tlg5/roundtrip");
    group.throughput(Throughput::Elements(input.pixels()));

    group.bench_with_input(BenchmarkId::from_parameter(&input.id), &input, |b, input| {
        b.iter_batched(
            || input.rgb.clone(),
            |rgb| {
                let encoded = Tlg5Encoder::from_rgb(rgb, input.width, input.height)
                    .encode()
                    .expect("TLG5 编码失败");
                let (decoded, _) = Tlg5Decoder::from_data(encoded)
                    .expect("构造 TLG5 解码器失败")
                    .decode()
                    .expect("TLG5 解码失败");
                black_box(decoded)
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(benches, bench_encode, bench_decode, bench_roundtrip);
criterion_main!(benches);
