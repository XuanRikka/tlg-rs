use std::io::Cursor;

use criterion::{
    criterion_group, criterion_main, BenchmarkId, Criterion, Throughput,
};
use image::{DynamicImage, GrayImage, RgbImage, RgbaImage};
use rand::{rngs::StdRng, RngExt, SeedableRng};

use tlg::tlg6::Tlg6Encoder;
use tlg::tlg_trait::TlgEncoderTrait;

const SEED: u64 = 42;

fn gen_gray_image(width: u32, height: u32) -> DynamicImage {
    let mut rng = StdRng::seed_from_u64(SEED);
    let pixel_count = (width * height) as usize;
    let mut data = vec![0u8; pixel_count];
    rng.fill(&mut data[..]);

    DynamicImage::ImageLuma8(
        GrayImage::from_raw(width, height, data).expect("Failed to create gray image"),
    )
}

fn gen_rgb_image(width: u32, height: u32) -> DynamicImage {
    let mut rng = StdRng::seed_from_u64(SEED);
    let pixel_count = (width * height * 3) as usize;
    let mut data = vec![0u8; pixel_count];
    rng.fill(&mut data[..]);

    DynamicImage::ImageRgb8(
        RgbImage::from_raw(width, height, data).expect("Failed to create rgb image"),
    )
}

fn gen_rgba_image(width: u32, height: u32) -> DynamicImage {
    let mut rng = StdRng::seed_from_u64(SEED);
    let pixel_count = (width * height * 4) as usize;
    let mut data = vec![0u8; pixel_count];
    rng.fill(&mut data[..]);

    DynamicImage::ImageRgba8(
        RgbaImage::from_raw(width, height, data).expect("Failed to create rgba image"),
    )
}

fn bench_encode_format(
    c: &mut Criterion,
    format_name: &str,
    gen_image: fn(u32, u32) -> DynamicImage,
    resolutions: &[(u32, u32)],
) {
    let mut group = c.benchmark_group(format!("tlg6_encode_{}", format_name));

    for &(w, h) in resolutions {
        let total_pixels = (w * h) as u64;
        group.throughput(Throughput::Elements(total_pixels));

        let image = gen_image(w, h);
        let encoder = Tlg6Encoder::from_image(&image).expect("failed to create encoder");

        let size = encoder.encode().unwrap().len();
        let mut buffer: Vec<u8> = Vec::with_capacity(size);

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x{}", w, h)),
            &encoder,
            |b, enc| {
                b.iter(|| {
                    buffer.clear();
                    let mut cur = Cursor::new(&mut buffer);
                    enc.encode_to(&mut cur).expect("encode failed");
                    std::hint::black_box(&buffer);
                });
            },
        );
    }
    group.finish();
}

fn encode_benchmarks(c: &mut Criterion) {
    let resolutions = [(256, 256), (512, 512), (1024, 1024)];

    bench_encode_format(c, "gray", gen_gray_image, &resolutions);
    bench_encode_format(c, "rgb", gen_rgb_image, &resolutions);
    bench_encode_format(c, "rgba", gen_rgba_image, &resolutions);
}

criterion_group!(benches, encode_benchmarks);
criterion_main!(benches);
