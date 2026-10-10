use std::hint::black_box;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use slide::{SlideDecoder, SlideEncoder};

const CORPUS_SIZE: usize = 1 << 20;
const CHUNK_SIZE: usize = 1024;

struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn fnv1a(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn gen_text(size: usize) -> Vec<u8> {
    const WORDS: &[&str] = &[
        "the", "of", "and", "to", "in", "is", "that", "it", "was", "for", "on", "are", "with",
        "as", "his", "they", "be", "at", "one", "have", "this", "from", "or", "had", "by",
        "hot", "but", "some", "what", "there", "we", "can", "out", "other", "were", "all",
        "your", "when", "up", "use", "word", "how", "said", "an", "each", "she", "which",
        "do", "their", "time", "if", "will", "way", "about", "many", "then", "them", "write",
        "would", "like", "so", "these", "her", "long", "make", "thing", "see", "him", "two",
        "has", "look", "more", "day", "could", "go", "come", "did", "number", "sound", "no",
        "compression", "window", "dictionary", "literal", "pointer", "encoder", "decoder",
    ];
    let mut rng = XorShift(0x1234_5678_9ABC_DEF1);
    let mut out = Vec::with_capacity(size + 16);
    while out.len() < size {
        let r = rng.below(WORDS.len() as u64 * WORDS.len() as u64);
        let idx = WORDS.len() - 1 - ((r as f64).sqrt() as usize).min(WORDS.len() - 1);
        out.extend_from_slice(WORDS[idx].as_bytes());
        match rng.below(12) {
            0 => out.extend_from_slice(b".\n"),
            1 => out.extend_from_slice(b", "),
            _ => out.push(b' '),
        }
    }
    out.truncate(size);
    out
}

fn gen_records(size: usize) -> Vec<u8> {
    let mut rng = XorShift(0x0BAD_C0DE_1234_5678);
    let mut out = Vec::with_capacity(size + 32);
    let mut id: u32 = 0;
    while out.len() < size {
        id = id.wrapping_add(1 + rng.below(3) as u32);
        out.extend_from_slice(&id.to_le_bytes());
        out.extend_from_slice(&(rng.below(16) as u16).to_le_bytes());
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&(1000 + rng.below(64) as u32).to_le_bytes());
        out.extend_from_slice(b"REC\0");
        out.extend_from_slice(&[rng.below(256) as u8, rng.below(256) as u8]);
        out.extend_from_slice(&[0xFF, 0xFF]);
    }
    out.truncate(size);
    out
}

fn gen_periodic(size: usize) -> Vec<u8> {
    let mut rng = XorShift(0x5EED_5EED_5EED_5EED);
    let period: Vec<u8> = (0..37).map(|_| rng.below(4) as u8).collect();
    (0..size)
        .map(|i| {
            if rng.below(200) == 0 {
                rng.below(256) as u8
            } else {
                period[i % period.len()]
            }
        })
        .collect()
}

fn gen_low_entropy(size: usize) -> Vec<u8> {
    let mut rng = XorShift(0xFEED_FACE_0BAD_F00D);
    (0..size).map(|_| rng.below(3) as u8).collect()
}

fn gen_random(size: usize) -> Vec<u8> {
    let mut rng = XorShift(0xDEAD_BEEF_CAFE_F00D);
    (0..size).map(|_| rng.next() as u8).collect()
}

fn gen_zeros(size: usize) -> Vec<u8> {
    vec![0; size]
}

fn corpora() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("text", gen_text(CORPUS_SIZE)),
        ("records", gen_records(CORPUS_SIZE)),
        ("periodic", gen_periodic(CORPUS_SIZE)),
        ("low_entropy", gen_low_entropy(CORPUS_SIZE)),
        ("random", gen_random(CORPUS_SIZE)),
        ("zeros", gen_zeros(CORPUS_SIZE)),
    ]
}

fn encode_whole(data: &[u8]) -> Vec<u8> {
    SlideEncoder::new().encode(data)
}

fn encode_chunked(data: &[u8]) -> Vec<Vec<u8>> {
    let mut enc = SlideEncoder::new();
    data.chunks(CHUNK_SIZE).map(|c| enc.encode(c)).collect()
}

fn decode_whole(enc: &[u8]) -> Vec<u8> {
    SlideDecoder::new().decode(enc)
}

fn decode_chunked(chunks: &[Vec<u8>], out: &mut Vec<u8>) {
    let mut dec = SlideDecoder::new();
    for c in chunks {
        dec.decode_into(c, out);
    }
}

fn bench_slide(c: &mut Criterion) {
    let mut group = c.benchmark_group("slide");
    group
        .sample_size(20)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2))
        .throughput(Throughput::Bytes(CORPUS_SIZE as u64));

    for (name, data) in corpora() {
        let compressed = encode_whole(&data);
        assert_eq!(&decode_whole(&compressed), &data);

        let chunks = encode_chunked(&data);
        let mut decoded = Vec::with_capacity(data.len());
        decode_chunked(&chunks, &mut decoded);
        assert_eq!(decoded, data);

        let joined: Vec<u8> = chunks.concat();
        println!(
            "{name:<12} whole={:>6.1}% chunked={:>6.1}% fnv_whole={:016x} fnv_chunked={:016x}",
            compressed.len() as f64 * 100.0 / data.len() as f64,
            joined.len() as f64 * 100.0 / data.len() as f64,
            fnv1a(&compressed),
            fnv1a(&joined),
        );

        group.bench_with_input(BenchmarkId::new("encode/whole", name), &data, |b, d| {
            b.iter(|| black_box(encode_whole(black_box(d))));
        });
        group.bench_with_input(BenchmarkId::new("decode/whole", name), &compressed, |b, e| {
            b.iter(|| black_box(decode_whole(black_box(e))));
        });
        group.bench_with_input(BenchmarkId::new("encode/chunked", name), &data, |b, d| {
            b.iter(|| black_box(encode_chunked(black_box(d))));
        });

        let mut scratch = Vec::with_capacity(data.len());
        group.bench_with_input(BenchmarkId::new("decode/chunked", name), &chunks, |b, cs| {
            b.iter(|| {
                scratch.clear();
                decode_chunked(black_box(cs), &mut scratch);
                black_box(scratch.len())
            });
        });
    }
    group.finish();

    let mut construction = c.benchmark_group("slide/construction");
    construction.sample_size(50);
    construction.bench_function("encoder/new", |b| b.iter(|| black_box(SlideEncoder::new())));
    construction.bench_function("decoder/new", |b| b.iter(|| black_box(SlideDecoder::new())));
    construction.finish();
}

criterion_group!(benches, bench_slide);
criterion_main!(benches);
