//! Robustness: arbitrary / truncated input must never panic, and the real
//! decoder must behave exactly like the reference decoder on it.

mod common;

use common::*;
use slide::{SlideDecoder, SlideEncoder};

#[test]
fn garbage_input_never_panics_and_matches_reference() {
    for seed in 0..300u64 {
        let mut rng = XorShift::new(seed);
        let len = rng.below(400) as usize;
        let alphabet = [256, 256, 16, 2][(seed % 4) as usize];
        let garbage = random_data(&mut rng, len, alphabet);

        let real = SlideDecoder::new().decode(&garbage);
        let (reference, _) = RefDecoder::new().decode(&garbage);
        assert_eq!(real, reference, "seed={seed}");
    }
}

#[test]
fn garbage_streams_across_multiple_calls_match_reference() {
    for seed in 0..50u64 {
        let mut rng = XorShift::new(9000 + seed);
        let mut real = SlideDecoder::new();
        let mut reference = RefDecoder::new();
        for call in 0..6 {
            let len = rng.below(300) as usize;
            let g = random_data(&mut rng, len, 256);
            let a = real.decode(&g);
            let (b, _) = reference.decode(&g);
            assert_eq!(a, b, "seed={seed} call={call}");
        }
    }
}

#[test]
fn every_truncation_of_a_valid_stream_is_handled() {
    for (name, data) in corpora(3000) {
        let c = SlideEncoder::new().encode(&data);
        for cut in 0..=c.len() {
            let real = SlideDecoder::new().decode(&c[..cut]);
            let (reference, _) = RefDecoder::new().decode(&c[..cut]);
            assert_eq!(real, reference, "{name} cut={cut}");
            assert!(data.starts_with(&real), "{name} cut={cut}: not a prefix");
        }
    }
}

#[test]
fn dangling_tokens_are_dropped() {
    // Flag says "match" but no bytes follow.
    assert!(SlideDecoder::new().decode(&[0x01]).is_empty());
    // Only one of the two position bytes.
    assert!(SlideDecoder::new().decode(&[0x01, 0x05]).is_empty());
    // Long form without its extension byte.
    assert!(SlideDecoder::new().decode(&[0x01, 0x00, 0xF0]).is_empty());
    // Flag says "literal" but no byte follows.
    assert!(SlideDecoder::new().decode(&[0x00]).is_empty());
    // Good literal followed by a dangling match is kept.
    assert_eq!(SlideDecoder::new().decode(&[0b10, b'z', 0x00]), b"z");
}

#[test]
fn all_match_positions_and_lengths_decode_in_bounds() {
    for pos in (0..N).step_by(37).chain([0, 1, 4094, 4095]) {
        for nib in 0..16usize {
            let mut c = vec![
                0x01,
                (pos & 0xff) as u8,
                ((pos >> 8) as u8) | ((nib as u8) << 4),
            ];
            let want = if nib == 15 {
                c.push(7);
                18 + 7
            } else {
                nib + 3
            };
            let out = SlideDecoder::new().decode(&c);
            assert_eq!(out.len(), want, "pos={pos} nib={nib}");
        }
    }
}

#[test]
fn encoder_handles_degenerate_inputs() {
    for len in [0usize, 1, 2, 3, 4, 273, 274, 275, 100_000] {
        for b in [0u8, 1, 0xFF] {
            let data = vec![b; len];
            let c = SlideEncoder::new().encode(&data);
            assert_eq!(SlideDecoder::new().decode(&c), data, "len={len} b={b}");
        }
    }
}

#[test]
fn encoder_output_is_bounded() {
    // Worst case: every byte a literal -> one flag byte per 8 literals.
    let mut rng = XorShift::new(1);
    let data = random_data(&mut rng, 100_000, 256);
    let c = SlideEncoder::new().encode(&data);
    assert!(
        c.len() <= data.len() + data.len() / 8 + 1,
        "len={}",
        c.len()
    );
}

#[test]
fn encoder_is_deterministic_and_independent_of_other_instances() {
    let mut rng = XorShift::new(2);
    let data = mixed(&mut rng, 30_000);
    let a = SlideEncoder::new().encode(&data);
    let mut other = SlideEncoder::new();
    other.encode(&random_data(&mut rng, 5000, 256));
    let b = SlideEncoder::new().encode(&data);
    assert_eq!(a, b);
}

#[test]
fn long_garbage_single_call_matches_reference() {
    // Garbage is ~50% match tokens, so 30 KB of input yields well over N bytes.
    for seed in 0..40u64 {
        let mut rng = XorShift::new(70_000 + seed);
        let alphabet = [256, 256, 64, 4][(seed % 4) as usize];
        let g = random_data(&mut rng, 30_000, alphabet);
        let real = SlideDecoder::new().decode(&g);
        let (reference, _) = RefDecoder::new().decode(&g);
        assert!(
            real.len() > N,
            "seed={seed}: output too small to be interesting"
        );
        assert_eq!(real, reference, "seed={seed}");
    }
}

#[test]
fn garbage_with_primed_window_and_state_matches_reference() {
    for seed in 0..60u64 {
        let mut rng = XorShift::new(80_000 + seed);
        let prime_len = [0usize, 7, 4096, 5000][(seed % 4) as usize];
        let prime = random_data(&mut rng, prime_len, 256);

        let mut real = SlideDecoder::new();
        let mut reference = RefDecoder::new();
        real.init_with_text(&prime);
        reference.init_with_text(&prime);

        let mut real_out = Vec::new();
        let mut ref_out = Vec::new();
        for call in 0..8 {
            // Mix tiny calls (< 1 group) with large ones.
            let len = [1usize, 2, 3, 9, 100, 700, 5000, 13][call] + rng.below(5) as usize;
            let g = random_data(&mut rng, len, 256);
            real.decode_into(&g, &mut real_out);
            let (b, _) = reference.decode(&g);
            ref_out.extend_from_slice(&b);
            assert_eq!(real_out, ref_out, "seed={seed} call={call}");
        }
    }
}

#[test]
fn matches_whose_distance_is_exactly_the_window_size() {
    // Fill the window with a counter, then copy from the slot at the cursor
    // (pos == s -> distance N): the byte written exactly N positions ago.
    for extra in 0..20usize {
        let fill = N + extra * 8;
        let data: Vec<u8> = (0..fill).map(|i| (i % 251) as u8).collect();
        let mut stream = Vec::new();
        for group in data.chunks(8) {
            stream.push(0);
            stream.extend_from_slice(group);
        }
        let s = fill % N;
        stream.extend_from_slice(&[0x01, (s & 0xff) as u8, ((s >> 8) as u8) | 0x20]); // pos=s, len=5

        let real = SlideDecoder::new().decode(&stream);
        let (reference, ok) = RefDecoder::new().decode(&stream);
        assert!(ok);
        assert_eq!(real, reference, "extra={extra}");
        let tail = &real[fill..];
        let want: Vec<u8> = (0..5).map(|k| data[fill - N + k]).collect();
        assert_eq!(tail, &want[..], "extra={extra}");
    }
}

/// The encoder only splits streams on flag-group boundaries per encode() call,
/// so decoding every encode() chunk with its own call must reproduce the data.
#[test]
fn encoded_streams_decode_identically_with_arbitrary_call_boundaries() {
    for kind in 0..KINDS {
        let mut rng = XorShift::new(90_000 + kind as u64);
        let data = data_for(kind, &mut rng, 60_000);
        let mut enc = SlideEncoder::new();
        let mut dec = SlideDecoder::new();
        let mut reference = RefDecoder::new();
        let mut got = Vec::new();
        let mut want = Vec::new();
        let mut i = 0;
        while i < data.len() {
            let n = (1 + rng.below(9000) as usize).min(data.len() - i);
            let c = enc.encode(&data[i..i + n]);
            dec.decode_into(&c, &mut got);
            want.extend_from_slice(&reference.decode(&c).0);
            i += n;
        }
        assert_eq!(got, data, "kind={kind}");
        assert_eq!(want, data, "kind={kind} (reference)");
    }
}
