//! Round-trip and decoder tests: the real decoder must agree with the reference
//! decoder, and hand-written streams pin down the wire format (including the
//! overlapping-copy semantics a fast decoder is most likely to break).

mod common;

use common::*;
use slide::{SlideDecoder, SlideEncoder};

#[test]
fn whole_roundtrip_on_all_corpora() {
    for (name, data) in corpora(100_000) {
        let c = SlideEncoder::new().encode(&data);
        assert_eq!(SlideDecoder::new().decode(&c), data, "{name}");
    }
}

#[test]
fn whole_roundtrip_many_lengths() {
    for kind in 0..KINDS {
        for len in (0..600).chain([4095, 4096, 4097, 8192, 9001]) {
            let mut rng = XorShift::new((kind * 10_007 + len) as u64);
            let data = data_for(kind, &mut rng, len);
            let c = SlideEncoder::new().encode(&data);
            assert_eq!(
                SlideDecoder::new().decode(&c),
                data,
                "kind={kind} len={len}"
            );
        }
    }
}

#[test]
fn streaming_roundtrip_random_chunking() {
    for kind in 0..KINDS {
        for seed in 0..4u64 {
            let mut rng = XorShift::new(seed * 131 + kind as u64);
            let total = 40_000;
            let data = data_for(kind, &mut rng, total);

            let mut enc = SlideEncoder::new();
            let mut dec = SlideDecoder::new();
            let mut got = Vec::new();
            let mut i = 0;
            while i < total {
                let n = (rng.below(2500) as usize).min(total - i);
                let c = enc.encode(&data[i..i + n]);
                dec.decode_into(&c, &mut got);
                i += n;
            }
            assert_eq!(got, data, "kind={kind} seed={seed}");
        }
    }
}

#[test]
fn decode_into_appends_and_decode_matches() {
    let mut rng = XorShift::new(5);
    let data = mixed(&mut rng, 20_000);
    let c = SlideEncoder::new().encode(&data);

    let mut out = b"prefix".to_vec();
    SlideDecoder::new().decode_into(&c, &mut out);
    assert_eq!(&out[..6], b"prefix");
    assert_eq!(&out[6..], &data[..]);
    assert_eq!(SlideDecoder::new().decode(&c), data);
}

#[test]
fn real_decoder_matches_reference_decoder_on_valid_streams() {
    for (name, data) in corpora(50_000) {
        let c = SlideEncoder::new().encode(&data);
        let (r, ok) = RefDecoder::new().decode(&c);
        assert!(ok, "{name}");
        assert_eq!(r, SlideDecoder::new().decode(&c), "{name}");
    }
}

/// Sanity check of the oracle itself, independent of the real encoder.
#[test]
fn reference_encoder_output_is_decodable() {
    for (name, data) in corpora(20_000) {
        let c = RefEncoder::new().encode(&data);
        let (r, ok) = RefDecoder::new().decode(&c);
        assert!(ok, "{name}");
        assert_eq!(r, data, "{name}");
    }
}

#[test]
fn empty_input() {
    assert!(SlideEncoder::new().encode(&[]).is_empty());
    assert!(SlideDecoder::new().decode(&[]).is_empty());
}

#[test]
fn literals_only() {
    let c = [
        0x00, b'a', b'b', b'c', b'd', b'e', b'f', b'g', b'h', 0x00, b'i',
    ];
    assert_eq!(SlideDecoder::new().decode(&c), b"abcdefghi");
}

#[test]
fn match_from_initial_zero_window() {
    // flag=1: one match, pos=0; nibble 0 -> 3 bytes of the zero window.
    assert_eq!(SlideDecoder::new().decode(&[0x01, 0x00, 0x00]), [0, 0, 0]);
    // nibble 14 -> 17 bytes (largest short form).
    assert_eq!(SlideDecoder::new().decode(&[0x01, 0x00, 0xE0]), [0u8; 17]);
    // nibble 15 + ext 0 -> 18 bytes (smallest long form).
    assert_eq!(
        SlideDecoder::new().decode(&[0x01, 0x00, 0xF0, 0x00]),
        [0u8; 18]
    );
    // ext 255 -> 273 bytes (maximum).
    assert_eq!(
        SlideDecoder::new().decode(&[0x01, 0x00, 0xF0, 0xFF]),
        vec![0u8; 273]
    );
}

#[test]
fn overlapping_copy_repeats_recent_bytes() {
    // "ab" then copy 3 bytes at pos 1: the source runs into this copy's output.
    let c = [0b0000_0100, b'a', b'b', 0x01, 0x00];
    assert_eq!(SlideDecoder::new().decode(&c), b"abbbb");

    // Distance 2, length 6 -> "ab" repeated.
    let c = [0b0000_0100, b'a', b'b', 0x00, 0x30];
    assert_eq!(SlideDecoder::new().decode(&c), b"abababab");

    // Distance 1, maximum length -> 273 copies of the same byte.
    let c = [0b0000_0010, b'x', 0x00, 0xF0, 0xFF];
    let mut want = vec![b'x'];
    want.extend(std::iter::repeat(b'x').take(273));
    assert_eq!(SlideDecoder::new().decode(&c), want);
}

#[test]
fn match_position_wraps_around_the_window() {
    // N+8 literals put the cursor at slot 8; copying 5 bytes from pos 4094
    // crosses the ring end (4094, 4095, 0, 1, 2) without self-overlap.
    let data: Vec<u8> = (0..N + 8).map(|i| (i % 251) as u8).collect();
    let mut stream = Vec::new();
    for group in data.chunks(8) {
        stream.push(0);
        stream.extend_from_slice(group);
    }
    stream.extend_from_slice(&[0x01, 0xFE, 0x0F | 0x20]); // pos=4094, len=5
    let mut want = data.clone();
    want.extend_from_slice(&[data[4094], data[4095], data[4096], data[4097], data[4098]]);
    assert_eq!(SlideDecoder::new().decode(&stream), want);
}

#[test]
fn match_with_distance_one_across_the_wrap_repeats_one_byte() {
    // Cursor at slot 0 after exactly N bytes; copying from slot 4095 is a
    // distance-1 overlapping copy, so every output byte equals data[N-1].
    let data: Vec<u8> = (0..N).map(|i| (i % 251) as u8).collect();
    let mut stream = Vec::new();
    for group in data.chunks(8) {
        stream.push(0);
        stream.extend_from_slice(group);
    }
    stream.extend_from_slice(&[0x01, 0xFF, 0x0F]); // pos=4095, len=3
    let mut want = data.clone();
    want.extend_from_slice(&[data[N - 1]; 3]);
    assert_eq!(SlideDecoder::new().decode(&stream), want);
}

#[test]
fn state_persists_between_decode_calls() {
    let mut dec = SlideDecoder::new();
    assert_eq!(dec.decode(&[0x00, b'h', b'e', b'l', b'l', b'o']), b"hello");
    // 'hello' now sits at slots 0..5; copy "ell" from pos 1.
    assert_eq!(dec.decode(&[0x01, 0x01, 0x00]), b"ell");
}

#[test]
fn init_with_text_primes_the_window() {
    let mut dec = SlideDecoder::new();
    dec.init_with_text(b"hello world");
    assert_eq!(dec.decode(&[0x01, 0x06, 0x20]), b"world"); // "world" from pos 6, len 5
}
