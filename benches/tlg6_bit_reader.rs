use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rand::{RngExt, SeedableRng, rngs::StdRng};

use tlg::tlg6::bitstream::{TLG6BitReader, TLG6BitStream};

const SEED: u64 = 42;

fn encode_values(values: &[u32], bit_lengths: &[u8]) -> Vec<u8> {
    let mut bs = TLG6BitStream::new();
    for (&value, &bit_len) in values.iter().zip(bit_lengths.iter()) {
        bs.put_value(value, bit_len as u32);
    }
    bs.take_data()
}

fn encode_gamma(values: &[u32]) -> Vec<u8> {
    let mut bs = TLG6BitStream::new();
    for &value in values {
        bs.put_gamma(value.max(1));
    }
    bs.take_data()
}

fn read_single_bits(data: &[u8], bit_count: usize) -> u32 {
    let mut br = TLG6BitReader::new(data);
    let mut acc = 0u32;
    for _ in 0..bit_count {
        acc = acc.wrapping_add(br.get_1bit() as u32);
    }
    acc
}

fn read_values(data: &[u8], bit_lengths: &[u8]) -> u32 {
    let mut br = TLG6BitReader::new(data);
    let mut acc = 0u32;
    for &bit_len in bit_lengths {
        acc ^= br.get_value(bit_len as u32);
    }
    acc
}

fn peek_u32_only(data: &[u8], count: usize) -> u32 {
    let mut br = TLG6BitReader::new(data);
    let mut acc = 0u32;
    for _ in 0..count {
        acc ^= br.peek_u32_le();
        br.skip_bits(1);
    }
    acc
}

fn skip_bits_only(data: &[u8], skip_pattern: &[u8]) -> u32 {
    let mut br = TLG6BitReader::new(data);
    for &skip in skip_pattern {
        br.skip_bits(skip as u32);
    }
    br.get_byte_pos() as u32
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

fn gen_skip_pattern(len: usize) -> Vec<u8> {
    let mut rng = StdRng::seed_from_u64(SEED + 3);
    (0..len).map(|_| rng.random_range(1..=7)).collect()
}

fn bench_reader_bits(c: &mut Criterion) {
    let sizes = [4 * 1024usize, 64 * 1024, 1024 * 1024];
    let mut group = c.benchmark_group("get_1bit");

    for &size in &sizes {
        let bits = gen_bits(size);
        let encoded = encode_values(
            &bits.iter().map(|&bit| bit as u32).collect::<Vec<_>>(),
            &vec![1u8; size],
        );
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &encoded, |b, encoded| {
            b.iter(|| std::hint::black_box(read_single_bits(encoded, size)));
        });
    }

    group.finish();
}

fn bench_reader_values(c: &mut Criterion) {
    let sizes = [1024usize, 16 * 1024];
    let mut group = c.benchmark_group("get_value");

    for &size in &sizes {
        let (values, bit_lengths) = gen_values_and_lengths(size);
        let encoded = encode_values(&values, &bit_lengths);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(encoded, bit_lengths),
            |b, (encoded, bit_lengths)| {
                b.iter(|| std::hint::black_box(read_values(encoded, bit_lengths)));
            },
        );
    }

    group.finish();
}

fn bench_reader_peek_u32(c: &mut Criterion) {
    let sizes = [4 * 1024usize, 64 * 1024, 1024 * 1024];
    let mut group = c.benchmark_group("peek_u32_le");

    for &size in &sizes {
        let values = gen_gamma_values(size);
        let encoded = encode_gamma(&values);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &encoded, |b, encoded| {
            b.iter(|| std::hint::black_box(peek_u32_only(encoded, size)));
        });
    }

    group.finish();
}

fn bench_reader_skip_bits(c: &mut Criterion) {
    let sizes = [4 * 1024usize, 64 * 1024, 1024 * 1024];
    let mut group = c.benchmark_group("skip_bits");

    for &size in &sizes {
        let values = gen_gamma_values(size);
        let encoded = encode_gamma(&values);
        let skip_pattern = gen_skip_pattern(size);
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(encoded, skip_pattern),
            |b, (encoded, skip_pattern)| {
                b.iter(|| std::hint::black_box(skip_bits_only(encoded, skip_pattern)));
            },
        );
    }

    group.finish();
}

fn reader_benchmarks(c: &mut Criterion) {
    bench_reader_bits(c);
    bench_reader_values(c);
    bench_reader_peek_u32(c);
    bench_reader_skip_bits(c);
}

criterion_group!(benches, reader_benchmarks);
criterion_main!(benches);
