use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rand::{RngExt, SeedableRng, rngs::StdRng};

use tlg::tlg6::bitstream::TLG6BitStream;

const SEED: u64 = 42;

fn write_single_bits(bits: &[u8]) -> usize {
    let mut bs = TLG6BitStream::new();
    for &bit in bits {
        bs.put_1bit(bit != 0);
    }
    bs.take_data().len()
}

fn write_values(values: &[u32], bit_lengths: &[u8]) -> usize {
    let mut bs = TLG6BitStream::new();
    for (&value, &bit_len) in values.iter().zip(bit_lengths.iter()) {
        bs.put_value(value, bit_len as u32);
    }
    bs.take_data().len()
}

fn write_gamma(values: &[u32]) -> usize {
    let mut bs = TLG6BitStream::new();
    for &value in values {
        bs.put_gamma(value.max(1));
    }
    bs.take_data().len()
}

fn gen_bits(len: usize) -> Vec<u8> {
    let mut rng = StdRng::seed_from_u64(SEED);
    (0..len).map(|_| (rng.random::<u8>() & 1) as u8).collect()
}

fn gen_values_and_lengths(len: usize) -> (Vec<u32>, Vec<u8>) {
    let mut rng = StdRng::seed_from_u64(SEED + 1);
    let bit_lengths: Vec<u8> = (0..len).map(|_| rng.random_range(1..=16)).collect();
    let values = bit_lengths
        .iter()
        .map(|&bits| rng.random_range(0..(1u32 << bits)))
        .collect();
    (values, bit_lengths)
}

fn gen_gamma_values(len: usize) -> Vec<u32> {
    let mut rng = StdRng::seed_from_u64(SEED + 2);
    (0..len).map(|_| rng.random_range(1..=4096)).collect()
}

fn bench_writer_bits(c: &mut Criterion) {
    let sizes = [4 * 1024usize, 64 * 1024, 1024 * 1024];
    let mut group = c.benchmark_group("put_1bit");

    for &size in &sizes {
        let bits = gen_bits(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &bits, |b, bits| {
            b.iter(|| std::hint::black_box(write_single_bits(bits)));
        });
    }

    group.finish();
}

fn bench_writer_values(c: &mut Criterion) {
    let sizes = [1024usize, 16 * 1024, 256 * 1024];
    let mut group = c.benchmark_group("put_value");

    for &size in &sizes {
        let (values, bit_lengths) = gen_values_and_lengths(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(values, bit_lengths),
            |b, (values, bit_lengths)| {
                b.iter(|| std::hint::black_box(write_values(values, bit_lengths)));
            },
        );
    }

    group.finish();
}

fn bench_writer_gamma(c: &mut Criterion) {
    let sizes = [1024usize, 16 * 1024, 256 * 1024];
    let mut group = c.benchmark_group("put_gamma");

    for &size in &sizes {
        let values = gen_gamma_values(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &values, |b, values| {
            b.iter(|| std::hint::black_box(write_gamma(values)));
        });
    }

    group.finish();
}

fn stream_benchmarks(c: &mut Criterion) {
    bench_writer_bits(c);
    bench_writer_values(c);
    bench_writer_gamma(c);
}

criterion_group!(benches, stream_benchmarks);
criterion_main!(benches);
