mod slide_common;

use std::hint::black_box;

use criterion::{
    BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main,
};
use slide_common::{BENCH_SIZES, prepare_input};
use tlg::slide::{SlideDecoder, SlideEncoder};

fn bench_slide_decompress(c: &mut Criterion) {
    let mut group = c.benchmark_group("slide_decompress");

    for &size in &BENCH_SIZES {
        let data = prepare_input(size);
        let mut compressor = SlideEncoder::new();
        let compressed = compressor.encode(&data);

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &compressed, |b, input| {
            b.iter_batched(
                || SlideDecoder::new(),
                |mut decomp| {
                    let out = decomp.decode(black_box(input));
                    black_box(out);
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

criterion_group!(benches, bench_slide_decompress);
criterion_main!(benches);
