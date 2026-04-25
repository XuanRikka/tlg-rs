use super::bitstream::TLG6BitStream;

pub(super) const GOLOMB_N_COUNT: usize = 4;
pub(super) const GOLOMB_GIVE_UP_BYTES: i32 = 4;

const GOLOMB_COMPRESSED: [[i16; 9]; GOLOMB_N_COUNT] = [
    [3, 7, 15, 27, 63, 108, 223, 448, 130],
    [3, 5, 13, 24, 51, 95, 192, 384, 257],
    [2, 5, 12, 21, 39, 86, 155, 320, 384],
    [2, 3, 9, 18, 33, 61, 129, 258, 511],
];

pub(super) const GOLOMB_TABLE: [[u8; GOLOMB_N_COUNT]; GOLOMB_N_COUNT * 2 * 128] = {
    let mut table = [[0u8; GOLOMB_N_COUNT]; GOLOMB_N_COUNT * 2 * 128];
    let mut n = 0;
    while n < GOLOMB_N_COUNT {
        let mut a = 0;
        let mut i = 0;
        while i < 9 {
            let mut j = 0;
            while j < GOLOMB_COMPRESSED[n][i] {
                table[a][n] = i as u8;
                a += 1;
                j += 1;
            }
            i += 1;
        }
        n += 1;
    }
    table
};

// ---------------------------------------------------------------------------
// Gamma code utilities
// ---------------------------------------------------------------------------

pub(super) fn gamma_bit_length(v: u32) -> u32 {
    if v <= 1 { return 1; }
    if v <= 3 { return 3; }
    if v <= 7 { return 5; }
    if v <= 15 { return 7; }
    if v <= 31 { return 9; }
    if v <= 63 { return 11; }
    if v <= 127 { return 13; }
    if v <= 255 { return 15; }
    if v <= 511 { return 17; }
    let mut need = 1u32;
    let mut t = v >> 1;
    while t > 0 {
        need += 2;
        t >>= 1;
    }
    need
}

// ---------------------------------------------------------------------------
// Try-compress: estimate Golomb compressed size without encoding
// ---------------------------------------------------------------------------

pub(super) struct TryCompress {
    total_bits: u32,
    count: u32,
    n: i32,
    a: i32,
    last_nonzero: bool,
}

impl TryCompress {
    pub(super) fn new() -> Self {
        TryCompress {
            total_bits: 0,
            count: 0,
            n: 0,
            a: 0,
            last_nonzero: false,
        }
    }

    pub(super) fn reset(&mut self) {
        self.total_bits = 1;
        self.count = 0;
        self.n = (GOLOMB_N_COUNT - 1) as i32;
        self.a = 0;
        self.last_nonzero = false;
    }

    pub(super) fn try_compress(&mut self, buf: &[i8]) -> u32 {
        let mut i = 0;
        while i < buf.len() {
            if buf[i] != 0 {
                if !self.last_nonzero {
                    if self.count > 0 {
                        self.total_bits += gamma_bit_length(self.count);
                    }
                    self.count = 0;
                }

                while i < buf.len() {
                    let e = buf[i] as i32;
                    if e == 0 { break; }
                    self.count += 1;

                    let k = GOLOMB_TABLE[self.a as usize][self.n as usize] as u32;
                    let m = if e >= 0 { 2 * e } else { -2 * e - 1 } - 1;
                    let mut unexp_bits = (m >> k) as u32;

                    let cap = GOLOMB_GIVE_UP_BYTES as u32 * 8 - 4;
                    if unexp_bits >= cap { unexp_bits = cap + 8; }

                    self.total_bits += unexp_bits + 1 + k;
                    self.a += m >> 1;
                    self.n -= 1;
                    if self.n < 0 {
                        self.a >>= 1;
                        self.n = (GOLOMB_N_COUNT - 1) as i32;
                    }
                    i += 1;
                }

                self.last_nonzero = true;
            } else {
                if self.last_nonzero {
                    if self.count > 0 {
                        self.total_bits += gamma_bit_length(self.count);
                        self.count = 0;
                    }
                }
                self.count += 1;
                self.last_nonzero = false;
                i += 1;
            }
        }
        self.total_bits
    }

    pub(super) fn flush(&mut self) -> u32 {
        if self.count > 0 {
            self.total_bits += gamma_bit_length(self.count);
            self.count = 0;
        }
        self.total_bits
    }
}

// ---------------------------------------------------------------------------
// Golomb entropy encoding (actual compression)
// ---------------------------------------------------------------------------

pub(super) fn compress_values_golomb(bs: &mut TLG6BitStream, buf: &[i8]) {
    bs.put_1bit(buf.first().map_or(false, |&v| v != 0));

    let mut n = (GOLOMB_N_COUNT - 1) as i32;
    let mut a = 0i32;
    let mut count = 0u32;

    let mut i = 0;
    while i < buf.len() {
        if buf[i] != 0 {
            if count > 0 {
                bs.put_gamma(count);
            }

            count = 0;
            let mut ii = i;
            while ii < buf.len() && buf[ii] != 0 {
                count += 1;
                ii += 1;
            }

            bs.put_gamma(count);

            while i < ii {
                let e = buf[i] as i32;
                let k = GOLOMB_TABLE[a as usize][n as usize] as u32;
                let m = if e >= 0 { 2 * e } else { -2 * e - 1 } - 1;
                let store_limit = bs.get_byte_pos() + GOLOMB_GIVE_UP_BYTES;

                let mut put1 = true;
                let zeros = (m >> k) as u32;
                for _ in 0..zeros {
                    if store_limit == bs.get_byte_pos() {
                        bs.put_value((m >> k) as u32, 8);
                        put1 = false;
                        break;
                    }
                    bs.put_1bit(false);
                }
                if store_limit == bs.get_byte_pos() {
                    bs.put_value((m >> k) as u32, 8);
                    put1 = false;
                }
                if put1 {
                    bs.put_1bit(true);
                }
                bs.put_value(m as u32, k);

                a += m >> 1;
                n -= 1;
                if n < 0 {
                    a >>= 1;
                    n = (GOLOMB_N_COUNT - 1) as i32;
                }
                i += 1;
            }

            count = 0;
        } else {
            count += 1;
            i += 1;
        }
    }

    if count > 0 {
        bs.put_gamma(count);
    }
}

// ---------------------------------------------------------------------------
// Leading zero table for fast gamma decoding
// ---------------------------------------------------------------------------

const LEADING_ZERO_TABLE_BITS: usize = 12;
const LEADING_ZERO_TABLE_SIZE: usize = 1 << LEADING_ZERO_TABLE_BITS;

const LEADING_ZERO_TABLE: [u8; LEADING_ZERO_TABLE_SIZE] = {
    let mut table = [0u8; LEADING_ZERO_TABLE_SIZE];
    let mut i = 0;
    while i < LEADING_ZERO_TABLE_SIZE {
        let mut cnt = 0;
        let mut j = 1;
        while j != LEADING_ZERO_TABLE_SIZE && (i & j) == 0 {
            j <<= 1;
            cnt += 1;
        }
        cnt += 1;
        if j == LEADING_ZERO_TABLE_SIZE {
            cnt = 0;
        }
        table[i] = cnt as u8;
        i += 1;
    }
    table
};

// ---------------------------------------------------------------------------
// Gamma decode (reads from a bit reader)
// ---------------------------------------------------------------------------

pub(super) fn decode_gamma(br: &mut super::bitstream::TLG6BitReader) -> u32 {
    let mut bit_count = 0u32;

    loop {
        if br.exhausted() {
            return 1;
        }
        let t = br.peek_u32_le();
        let b = LEADING_ZERO_TABLE[(t & (LEADING_ZERO_TABLE_SIZE as u32 - 1)) as usize];
        if b != 0 {
            bit_count += b as u32;
            br.skip_bits(b as u32);
            break;
        }
        bit_count += LEADING_ZERO_TABLE_BITS as u32;
        br.skip_bits(LEADING_ZERO_TABLE_BITS as u32);
    }

    bit_count -= 1;
    let count = (1u32 << bit_count) + br.get_value(bit_count);
    count
}

// ---------------------------------------------------------------------------
// Golomb value decode
// ---------------------------------------------------------------------------

/// Decode a single non-zero signed value from the bitstream.
/// Returns the signed value e (never zero).
pub(super) fn decode_golomb_value(
    br: &mut super::bitstream::TLG6BitReader,
    a: &mut i32,
    n: &mut i32,
) -> i32 {
    let k = GOLOMB_TABLE[*a as usize][*n as usize] as u32;

    // Read 32-bit window BEFORE advancing the stream (same as C++ TVP_TLG6_FETCH_32BITS >> bit_pos)
    let mut t = br.peek_u32_le();
    let (bit_count, b) = if t != 0 {
        let mut b = LEADING_ZERO_TABLE[(t & (LEADING_ZERO_TABLE_SIZE as u32 - 1)) as usize];
        let mut bit_count = b as u32;
        while b == 0 {
            if br.exhausted() {
                // exhausted mid-loop — give up reading this value
                let old_pos = br.get_byte_pos();
                let bc = br.peek_byte_at((old_pos + 4) as usize) as u32;
                br.set_byte_pos(old_pos + 5);
                t = br.peek_u32_le();
                return finish_golomb_value(t, bc, k, a, n);
            }
            bit_count += LEADING_ZERO_TABLE_BITS as u32;
            br.skip_bits(LEADING_ZERO_TABLE_BITS as u32);
            t = br.peek_u32_le();
            b = LEADING_ZERO_TABLE[(t & (LEADING_ZERO_TABLE_SIZE as u32 - 1)) as usize];
            bit_count += b as u32;
        }
        bit_count -= 1;
        // Don't skip bits yet — t still holds the window before the '1' terminator
        (bit_count, b as u32)
    } else {
        // give-up: C++ does bit_pool += 5; bit_count = bit_pool[-1]; then re-read t
        let old_pos = br.get_byte_pos();
        let bit_count = br.peek_byte_at((old_pos + 4) as usize) as u32;
        br.set_byte_pos(old_pos + 5);
        t = br.peek_u32_le();
        (bit_count, 0u32)
    };

    let v = (bit_count << k) + ((t >> b) & ((1 << k) - 1));
    let sign = (v & 1).wrapping_sub(1) as i32;
    let v_shifted = (v >> 1) as i32;
    *a += v_shifted;
    let e = (v_shifted ^ sign) + sign + 1;

    // Now advance past the '1' terminator + k lower bits
    br.skip_bits(b + k);

    *n -= 1;
    if *n < 0 {
        *a >>= 1;
        *n = (GOLOMB_N_COUNT - 1) as i32;
    }

    e
}

/// Shared tail of decode_golomb_value — compute the signed value and update state.
#[inline]
fn finish_golomb_value(t: u32, bit_count: u32, k: u32, a: &mut i32, n: &mut i32) -> i32 {
    let v = (bit_count << k) + (t & ((1 << k) - 1));
    let sign = (v & 1).wrapping_sub(1) as i32;
    let v_shifted = (v >> 1) as i32;
    *a += v_shifted;
    let e = (v_shifted ^ sign) + sign + 1;

    *n -= 1;
    if *n < 0 {
        *a >>= 1;
        *n = (GOLOMB_N_COUNT - 1) as i32;
    }

    e
}

// ---------------------------------------------------------------------------
// Golomb block decode — decodes a full channel's worth of values
// ---------------------------------------------------------------------------

pub(super) fn decode_golomb_channel(
    br: &mut super::bitstream::TLG6BitReader,
    dst: &mut [i8],
    pixel_count: usize,
    dst_stride: usize,
    dst_offset: usize,
    is_first_color: bool,
) {
    let mut n = (GOLOMB_N_COUNT - 1) as i32;
    let mut a = 0i32;

    let mut zero = !br.get_1bit();
    let mut pixel_idx = 0usize;

    while pixel_idx < pixel_count {
        let count = decode_gamma(br) as usize;

        if zero {
            // zero run
            if is_first_color {
                for _ in 0..count {
                    dst[pixel_idx * dst_stride + dst_offset] = 0;
                    pixel_idx += 1;
                }
            } else {
                for _ in 0..count {
                    dst[pixel_idx * dst_stride + dst_offset] = 0;
                    pixel_idx += 1;
                }
            }
        } else {
            // non-zero run
            for _ in 0..count {
                let e = decode_golomb_value(br, &mut a, &mut n);
                dst[pixel_idx * dst_stride + dst_offset] = e as i8;
                pixel_idx += 1;
            }
        }

        zero = !zero;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_golomb_table_size() {
        assert_eq!(GOLOMB_TABLE.len(), GOLOMB_N_COUNT * 2 * 128);
    }

    #[test]
    fn test_gamma_bit_length() {
        assert_eq!(gamma_bit_length(1), 1);
        assert_eq!(gamma_bit_length(2), 3);
        assert_eq!(gamma_bit_length(3), 3);
        assert_eq!(gamma_bit_length(4), 5);
    }

    #[test]
    fn test_leading_zero_table() {
        // bit 0 set → position 1
        assert_eq!(LEADING_ZERO_TABLE[1], 1);
        // bit 1 set → position 2
        assert_eq!(LEADING_ZERO_TABLE[2], 2);
        // all zeros → 0
        assert_eq!(LEADING_ZERO_TABLE[0], 0);
    }
}
