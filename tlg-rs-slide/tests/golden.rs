//! Golden baselines: lock the exact compressed bytes for fixed, deterministic
//! corpora, in both whole-buffer and chunked modes.
//!
//! If an intentional format change alters the output, regenerate the table with
//!   cargo test --release --test golden -- --ignored --nocapture print_golden

mod common;

use common::*;
use slide::SlideEncoder;

const SIZE: usize = 64 * 1024;
const CHUNK: usize = 1000;

fn whole(data: &[u8]) -> Vec<u8> {
    SlideEncoder::new().encode(data)
}

fn chunked(data: &[u8]) -> Vec<u8> {
    let mut enc = SlideEncoder::new();
    let mut out = Vec::new();
    for c in data.chunks(CHUNK) {
        out.extend_from_slice(&enc.encode(c));
    }
    out
}

/// (corpus, whole: (len, hash), chunked: (len, hash))
const GOLDEN: &[(&str, (usize, u64), (usize, u64))] = &[
    (
        "zeros",
        (754, 0x57a2554557c5098f),
        (918, 0xebae189af25d9a90),
    ),
    ("ones", (771, 0xa17528c6e3038bd9), (937, 0xefe555486933bc6f)),
    (
        "alpha2",
        (11505, 0x5bf519e22fd49a94),
        (11653, 0xb971a2f1b2ff5a91),
    ),
    (
        "alpha3",
        (18521, 0xe04acd91b1dcac7c),
        (18658, 0x3de3772f5fd428a3),
    ),
    (
        "alpha16",
        (50011, 0xf677ff520579fefc),
        (50095, 0xea0d75ab137422f8),
    ),
    (
        "random",
        (73645, 0xe6daaeb23a09c274),
        (73661, 0xfd9f018a934fa251),
    ),
    (
        "text",
        (14842, 0xb09dd144bb9628f0),
        (14998, 0x40f6ec7137b79c3f),
    ),
    (
        "periodic",
        (2062, 0xc02f614569234429),
        (2302, 0x8db19bf16caa94ba),
    ),
    (
        "runs",
        (4436, 0x4ebc4dc388450786),
        (4591, 0xf779cc4cfd5c2863),
    ),
    (
        "mixed",
        (16155, 0x7b7144beb7caa7c7),
        (16329, 0xadb4eb4be0480353),
    ),
];

#[test]
#[ignore]
fn print_golden() {
    for (name, data) in corpora(SIZE) {
        let w = whole(&data);
        let c = chunked(&data);
        println!(
            "    ({:?}, ({}, {:#018x}), ({}, {:#018x})),",
            name,
            w.len(),
            fnv1a(&w),
            c.len(),
            fnv1a(&c)
        );
    }
}

#[test]
fn golden_outputs_are_unchanged() {
    let all = corpora(SIZE);
    assert_eq!(
        all.len(),
        GOLDEN.len(),
        "corpus list and GOLDEN table differ"
    );
    for ((name, data), (gname, gw, gc)) in all.iter().zip(GOLDEN) {
        assert_eq!(name, gname);
        let w = whole(data);
        let c = chunked(data);
        assert_eq!(
            (w.len(), fnv1a(&w)),
            *gw,
            "{name}: whole-buffer output changed"
        );
        assert_eq!((c.len(), fnv1a(&c)), *gc, "{name}: chunked output changed");
    }
}

/// The generators themselves are pinned so a change there cannot silently
/// invalidate (or refresh) the golden table.
#[test]
fn generators_are_stable() {
    let data = corpora(4096);
    let hashes: Vec<u64> = data.iter().map(|(_, d)| fnv1a(d)).collect();
    assert_eq!(hashes, GENERATOR_HASHES);
}

const GENERATOR_HASHES: [u64; 10] = [
    0xb93a0c83ce3b6325,
    0xc0b014328067f325,
    0x84baaafb41ed93ab,
    0x70fda7746482ad9a,
    0xd805efe276d6c06e,
    0x465eeb2f11fc7791,
    0xeef30fe97aa79c42,
    0x88fc77f5059fe9b4,
    0x4230275765ff081f,
    0x22b0b64766e95032,
];

#[test]
#[ignore]
fn print_generator_hashes() {
    let hashes: Vec<String> = corpora(4096)
        .iter()
        .map(|(_, d)| format!("{:#018x}", fnv1a(d)))
        .collect();
    println!(
        "const GENERATOR_HASHES: [u64; 10] = [{}];",
        hashes.join(", ")
    );
}
