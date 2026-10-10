//! Differential tests: the encoder must produce exactly the same bytes as the
//! naive reference encoder in common.

mod common;

use common::*;
use slide::SlideEncoder;

const LENS: &[usize] = &[
    0, 1, 2, 3, 4, 5, 6, 17, 18, 19, 20, 100, 272, 273, 274, 275, 276, 600, 4094, 4095, 4096, 4097,
    4098, 6000, 8191, 8192, 8193,
];

#[test]
fn whole_input_matches_reference() {
    for kind in 0..KINDS {
        for &len in LENS {
            let mut rng = XorShift::new(1000 + (kind * 100_000 + len) as u64);
            let data = data_for(kind, &mut rng, len);
            let a = SlideEncoder::new().encode(&data);
            let b = RefEncoder::new().encode(&data);
            assert_eq!(a, b, "kind={kind} len={len}");
        }
    }
}

/// One long-lived encoder fed with chunks of random (often tiny, sometimes
/// zero) sizes: exercises the cold-start regime, the wrap and steady state,
/// with encode() boundaries at arbitrary places.
fn check_stream(kind: usize, seed: u64, total: usize, max_chunk: u64) {
    let mut rng = XorShift::new(seed);
    let data = data_for(kind, &mut rng, total);

    let mut real = SlideEncoder::new();
    let mut reference = RefEncoder::new();
    let mut i = 0;
    let mut calls = 0;
    while i < total {
        let n = (rng.below(max_chunk + 1) as usize).min(total - i);
        let a = real.encode(&data[i..i + n]);
        let b = reference.encode(&data[i..i + n]);
        assert_eq!(
            a, b,
            "kind={kind} seed={seed} call={calls} offset={i} chunk={n}"
        );
        i += n;
        calls += 1;
    }
}

#[test]
fn chunked_streams_match_reference() {
    for kind in 0..KINDS {
        for seed in 0..3u64 {
            check_stream(kind, seed, 3 * N + 700, 1500);
        }
    }
}

#[test]
fn tiny_chunk_streams_match_reference() {
    for kind in 0..KINDS {
        check_stream(kind, 77, 2 * N + 300, 5);
    }
}

/// The second call must see the window left by the first, including the case
/// where the first call ends mid flag-group.
#[test]
fn consecutive_calls_equal_split_state() {
    for kind in 0..KINDS {
        let mut rng = XorShift::new(31 + kind as u64);
        let a = data_for(kind, &mut rng, 1234);
        let b = data_for(kind, &mut rng, 4321);

        let mut real = SlideEncoder::new();
        let mut reference = RefEncoder::new();
        assert_eq!(real.encode(&a), reference.encode(&a), "kind={kind} first");
        assert_eq!(real.encode(&b), reference.encode(&b), "kind={kind} second");
    }
}
