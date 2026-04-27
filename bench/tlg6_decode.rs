// use criterion::{
//     BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main,
// };
// use image::{DynamicImage, GrayImage, RgbImage, RgbaImage};
// use rand::{RngExt, SeedableRng, rngs::StdRng};
//
// use tlg::tlg6::{Tlg6Decoder, Tlg6Encoder};
// use tlg::tlg_type::{TlgDecoderTrait, TlgEncoderTrait};
//
// const SEED: u64 = 42;
//
// fn gen_gray_image(width: u32, height: u32) -> DynamicImage {
//     let mut rng = StdRng::seed_from_u64(SEED);
//     let pixel_count = (width * height) as usize;
//     let mut data = vec![0u8; pixel_count];
//     rng.fill(&mut data[..]);
//
//     DynamicImage::ImageLuma8(
//         GrayImage::from_raw(width, height, data).expect("failed to create gray image"),
//     )
// }
//
// fn gen_rgb_image(width: u32, height: u32) -> DynamicImage {
//     let mut rng = StdRng::seed_from_u64(SEED);
//     let pixel_count = (width * height * 3) as usize;
//     let mut data = vec![0u8; pixel_count];
//     rng.fill(&mut data[..]);
//
//     DynamicImage::ImageRgb8(
//         RgbImage::from_raw(width, height, data).expect("failed to create rgb image"),
//     )
// }
//
// fn gen_rgba_image(width: u32, height: u32) -> DynamicImage {
//     let mut rng = StdRng::seed_from_u64(SEED);
//     let pixel_count = (width * height * 4) as usize;
//     let mut data = vec![0u8; pixel_count];
//     rng.fill(&mut data[..]);
//
//     DynamicImage::ImageRgba8(
//         RgbaImage::from_raw(width, height, data).expect("failed to create rgba image"),
//     )
// }
//
// fn bench_decode_format(
//     c: &mut Criterion,
//     format_name: &str,
//     gen_image: fn(u32, u32) -> DynamicImage,
//     resolutions: &[(u32, u32)],
// ) {
//     let mut group = c.benchmark_group(format!("decode_{}", format_name));
//
//     for &(w, h) in resolutions {
//         let total_pixels = (w * h) as u64;
//         group.throughput(Throughput::Elements(total_pixels));
//
//         let image = gen_image(w, h);
//         let encoded = Tlg6Encoder::from_image(&image)
//             .expect("failed to create encoder")
//             .encode()
//             .expect("encode failed");
//
//         group.bench_with_input(
//             BenchmarkId::from_parameter(format!("{}x{}", w, h)),
//             &encoded,
//             |b, data: &Vec<u8>| {
//                 b.iter(|| {
//                     let decoder = Tlg6Decoder::from_data(data.clone()).expect("decoder init failed");
//                     let image = decoder.decode().expect("decode failed");
//                     black_box(image);
//                 });
//             },
//         );
//     }
//
//     group.finish();
// }
//
// fn decode_benchmarks(c: &mut Criterion) {
//     let resolutions = [(256, 256), (512, 512), (1024, 1024)];
//
//     bench_decode_format(c, "gray", gen_gray_image, &resolutions);
//     bench_decode_format(c, "rgb", gen_rgb_image, &resolutions);
//     bench_decode_format(c, "rgba", gen_rgba_image, &resolutions);
// }
//
// criterion_group!(benches, decode_benchmarks);
// criterion_main!(benches);
