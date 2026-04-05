use criterion::{black_box, BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use tlg::tlg5::slide::compressor::SlideCompressor;

fn bench_slide_compress(c: &mut Criterion) {
    let mut group = c.benchmark_group("slide_compress");
    let sizes = [256 * 1024usize, 1024 * 1024usize, 4 * 1024 * 1024usize];
    let seed = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-";

    for &size in &sizes {
        let mut data = Vec::with_capacity(size);
        while data.len() < size {
            data.extend_from_slice(seed);
        }
        data.truncate(size);

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, input| {
            b.iter_batched(
                SlideCompressor::new,
                |mut compressor| {
                    let out = compressor.encode(black_box(input));
                    black_box(out);
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

criterion_group!(benches, bench_slide_compress);
criterion_main!(benches);
