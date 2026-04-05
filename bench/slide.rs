use std::hint::black_box;
use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use tlg::tlg5::slide::{SlideCompressor, SLIDE_N, SLIDE_M};

fn prepare_input(size: usize) -> Vec<u8> {
    let seed = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-";
    let mut data = Vec::with_capacity(size);
    while data.len() < size {
        data.extend_from_slice(seed);
    }
    data.truncate(size);
    data
}

fn bench_slide_compress(c: &mut Criterion) {
    let mut group = c.benchmark_group("slide_compress");
    let sizes = [256 * 1024usize, 1024 * 1024usize, 4 * 1024 * 1024usize];

    for &size in &sizes {
        let data = prepare_input(size);

        let mut compressor = SlideCompressor::new();

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, input| {
            b.iter(|| {
                let out = compressor.encode(black_box(input));
                black_box(out);
            });
        });
    }

    group.finish();
}

fn bench_slide_decompress(c: &mut Criterion) {
    let mut group = c.benchmark_group("slide_decompress");
    let sizes = [256 * 1024usize, 1024 * 1024usize, 4 * 1024 * 1024usize];

    for &size in &sizes {
        let data = prepare_input(size);

        let mut compressor = SlideCompressor::new();
        let compressed = compressor.encode(&data);

        let mut decompressor = SlideCompressor::new();

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &compressed, |b, input| {
            b.iter(|| {
                let out = decompressor.decode(black_box(input));
                black_box(out);
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_slide_compress, bench_slide_decompress);
criterion_main!(benches);