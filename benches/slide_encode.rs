mod slide_common;

use std::hint::black_box;

use criterion::{
    BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main,
};
use slide_common::{BENCH_SIZES, prepare_input};
use tlg::slide::SlideEncoder;

fn bench_slide_compress(c: &mut Criterion) {
    let mut group = c.benchmark_group("slide_compress");

    for &size in &BENCH_SIZES {
        let data = prepare_input(size);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, input| {
            b.iter_batched(
                || SlideEncoder::new(),
                |mut comp| {
                    let out = comp.encode(black_box(input));
                    black_box(out);
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_store_restore(c: &mut Criterion) {
    let mut group = c.benchmark_group("store_restore");

    for &size in &BENCH_SIZES {
        let data = prepare_input(size);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, input| {
            b.iter_batched(
                || {
                    let mut comp = SlideEncoder::new();
                    let half = input.len() / 2;
                    comp.encode(&input[..half]);
                    comp
                },
                |mut comp| {
                    comp.store();
                    comp.restore();
                    black_box(());
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

criterion_group!(benches, bench_slide_compress, bench_store_restore);
criterion_main!(benches);
