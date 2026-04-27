// use std::io::Cursor;
// use std::process::exit;
// use criterion::{
//     criterion_group, criterion_main, BenchmarkId, Criterion, Throughput,
// };
// use image::{DynamicImage, GrayImage, RgbImage, RgbaImage};
// use rand::{rngs::StdRng, Rng, RngExt, SeedableRng};
//
// // 替换为你的 crate 名称
// use tlg::tlg5::{Tlg5Encoder};
// use tlg::tlg_type::TlgEncoderTrait;
//
// const SEED: u64 = 42;
//
// fn gen_gray_image(width: u32, height: u32) -> DynamicImage {
//     let mut rng = StdRng::seed_from_u64(SEED);
//     let pixel_count = (width * height) as usize;
//
//     // 预分配向量
//     let mut data = vec![0u8; pixel_count];
//
//     // 高性能填充：使用 fill 而不是 map/collect
//     // rand 0.8: rng.fill(&mut data[..]);
//     // rand 0.9: rng.fill(&mut data[..]);
//     // 注意：fill 需要 slice 实现 Distribution<u8>，Standard 默认支持 u8
//     rng.fill(&mut data[..]);
//
//     DynamicImage::ImageLuma8(
//         GrayImage::from_raw(width, height, data).expect("Failed to create gray image")
//     )
// }
//
// /// 生成随机 RGB 图
// fn gen_rgb_image(width: u32, height: u32) -> DynamicImage {
//     let mut rng = StdRng::seed_from_u64(SEED);
//     let pixel_count = (width * height * 3) as usize;
//     let mut data = vec![0u8; pixel_count];
//
//     rng.fill(&mut data[..]);
//
//     DynamicImage::ImageRgb8(
//         RgbImage::from_raw(width, height, data).expect("Failed to create rgb image")
//     )
// }
//
// /// 生成随机 RGBA 图
// fn gen_rgba_image(width: u32, height: u32) -> DynamicImage {
//     let mut rng = StdRng::seed_from_u64(SEED);
//     let pixel_count = (width * height * 4) as usize;
//     let mut data = vec![0u8; pixel_count];
//
//     rng.fill(&mut data[..]);
//
//     DynamicImage::ImageRgba8(
//         RgbaImage::from_raw(width, height, data).expect("Failed to create rgba image")
//     )
// }
//
// /// 对指定像素格式和分辨率进行编码基准测试
// fn bench_encode_format(
//     c: &mut Criterion,
//     format_name: &str,
//     gen_image: fn(u32, u32) -> DynamicImage,
//     resolutions: &[(u32, u32)],
//     sizes: &[usize]
// ) {
//     let mut group = c.benchmark_group(format!("encode_{}", format_name));
//
//     for (&(w, h), size)in resolutions.iter().zip(sizes.iter()) {
//         let total_pixels = (w * h) as u64;
//         group.throughput(Throughput::Elements(total_pixels));
//
//         let image = gen_image(w, h);
//         let encoder = Tlg5Encoder::from_image(&image).expect("failed to create encoder");
//
//         let size = encoder.encode().unwrap().len();
//
//         let mut buffer: Vec<u8> = Vec::with_capacity(size);
//
//         group.bench_with_input(
//             BenchmarkId::from_parameter(format!("{}x{}", w, h)),
//             encoder,
//             |b, enc| {
//                 b.iter(|| {
//                     buffer.clear();
//                     let mut cur = Cursor::new(&mut buffer);
//                     enc.encode_to(&mut cur).expect("encode failed");
//                 });
//             },
//         );
//     }
//     group.finish();
// }
//
// fn encode_benchmarks(c: &mut Criterion) {
//     let resolutions = [(256, 256), (512, 512), (1024, 1024)];
//     let gray_sizes: [usize; 3] = [66136, 263320, 1050904];
//     let rgb_sizes: [usize; 3] = [197848, 788888, 3150616];
//     let rgba_sizes: [usize; 3] = [263704, 1051672, 4200472];
//
//     bench_encode_format(c, "gray", gen_gray_image, &resolutions, &gray_sizes);
//     bench_encode_format(c, "rgb", gen_rgb_image, &resolutions, &rgb_sizes);
//     bench_encode_format(c, "rgba", gen_rgba_image, &resolutions, &rgba_sizes);
// }
//
// criterion_group!(benches, encode_benchmarks);
// criterion_main!(benches);