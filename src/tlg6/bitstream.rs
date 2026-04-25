// ---------------------------------------------------------------------------
// TLG6 bit-stream (writes bits LSB-first into a byte buffer)
// ---------------------------------------------------------------------------

pub struct TLG6BitStream {
    buf: Vec<u8>,
    byte_pos: usize,
    bit_pos: u8,
}

impl TLG6BitStream {
    pub fn new() -> Self {
        TLG6BitStream {
            buf: Vec::new(),
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    pub fn get_byte_pos(&self) -> i32 {
        self.byte_pos as i32
    }

    pub fn get_bit_length(&self) -> u32 {
        self.byte_pos as u32 * 8 + self.bit_pos as u32
    }

    #[inline(always)]
    fn ensure(&mut self, need: usize) {
        if self.byte_pos + need >= self.buf.len() {
            self.buf.resize(self.byte_pos + need + 0x1000, 0);
        }
    }

    #[inline(always)]
    pub fn put_1bit(&mut self, b: bool) {
        self.ensure(1);
        if b {
            unsafe {
                *self.buf.get_unchecked_mut(self.byte_pos) |= 1 << self.bit_pos;
            }
        }
        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
    }

    #[inline(always)]
    pub fn put_value(&mut self, v: u32, len: u32) {
        if len == 0 {
            return;
        }
        self.ensure(5);
        let mut remaining = len;
        let mut val = v;
        let mut bp = self.bit_pos as u32;
        let mut pos = self.byte_pos;

        while remaining > 0 {
            let space = 8 - bp;
            let n = remaining.min(space);
            let mask = ((1u32 << n) - 1) as u8;
            let bits = (val as u8 & mask) << bp;
            unsafe {
                *self.buf.get_unchecked_mut(pos) |= bits;
            }
            val >>= n;
            remaining -= n;
            bp += n;
            if bp == 8 {
                bp = 0;
                pos += 1;
            }
        }

        self.byte_pos = pos;
        self.bit_pos = bp as u8;
    }

    pub fn put_gamma(&mut self, v: u32) {
        let cnt = if v <= 1 { 0 } else { 31 - (v >> 1).leading_zeros() + 1 };
        self.ensure((cnt * 2 + 1) as usize / 8 + 2);
        for _ in 0..cnt {
            self.put_1bit(false);
        }
        self.put_1bit(true);
        if cnt > 0 {
            self.put_value(v, cnt);
        }
    }

    pub fn take_data(&mut self) -> Vec<u8> {
        if self.bit_pos != 0 {
            self.byte_pos += 1;
        }
        self.buf.truncate(self.byte_pos);
        let data = std::mem::take(&mut self.buf);
        self.byte_pos = 0;
        self.bit_pos = 0;
        data
    }
}

// ---------------------------------------------------------------------------
// TLG6 bit-stream reader (reads bits LSB-first from a byte slice)
// Pads input with 8 zero bytes so u32 reads never go out of bounds,
// allowing unsafe unchecked access on the hot path.
// ---------------------------------------------------------------------------

const PADDING: usize = 8;

pub struct TLG6BitReader {
    data: Vec<u8>,
    original_len: usize,
    byte_pos: usize,
    bit_pos: u8,
}

impl TLG6BitReader {
    pub fn new(data: &[u8]) -> Self {
        let original_len = data.len();
        let mut padded = Vec::with_capacity(original_len + PADDING);
        padded.extend_from_slice(data);
        padded.resize(original_len + PADDING, 0);
        TLG6BitReader {
            data: padded,
            original_len,
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    pub fn get_byte_pos(&self) -> i32 {
        self.byte_pos as i32
    }

    pub fn set_byte_pos(&mut self, pos: i32) {
        self.byte_pos = pos as usize;
        self.bit_pos = 0;
    }

    #[inline(always)]
    pub fn exhausted(&self) -> bool {
        self.byte_pos >= self.original_len
    }

    #[inline(always)]
    pub fn skip_bits(&mut self, n: u32) {
        let total = self.bit_pos as u32 + n;
        self.byte_pos += (total >> 3) as usize;
        self.bit_pos = (total & 7) as u8;
    }

    #[inline(always)]
    pub fn get_1bit(&mut self) -> bool {
        if self.exhausted() {
            return false;
        }
        let b = unsafe { (*self.data.get_unchecked(self.byte_pos) >> self.bit_pos) & 1 != 0 };
        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
        b
    }

    #[inline(always)]
    pub fn get_value(&mut self, len: u32) -> u32 {
        if len == 0 || self.exhausted() {
            return 0;
        }
        debug_assert!(len <= 32 - self.bit_pos as u32);
        let raw = self.peek_u32_le();
        let v = raw & ((1u32 << len) - 1);
        self.skip_bits(len);
        v
    }

    pub fn get_gamma(&mut self) -> u32 {
        let mut cnt = 0u32;
        while !self.get_1bit() {
            cnt += 1;
            if self.exhausted() {
                return 1;
            }
        }
        if cnt == 0 {
            return 1;
        }
        self.get_value(cnt) + (1 << cnt)
    }

    #[inline(always)]
    pub fn peek_byte_at(&self, byte_offset: usize) -> u8 {
        debug_assert!(byte_offset < self.data.len());
        unsafe { *self.data.get_unchecked(byte_offset) }
    }

    #[inline(always)]
    pub fn peek_u32_le(&self) -> u32 {
        if self.byte_pos >= self.original_len {
            return 0;
        }
        let raw = unsafe {
            (self.data.as_ptr().add(self.byte_pos) as *const u32).read_unaligned()
        };
        u32::from_le(raw) >> self.bit_pos
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitstream() {
        let mut bs = TLG6BitStream::new();
        bs.put_1bit(true);
        bs.put_1bit(false);
        bs.put_value(5, 3); // 101 (LSB first)
        let data = bs.take_data();
        assert!(!data.is_empty());
    }

    #[test]
    fn test_bitreader_gamma() {
        let mut bs = TLG6BitStream::new();
        bs.put_gamma(1);
        bs.put_gamma(2);
        bs.put_gamma(5);
        let data = bs.take_data();

        let mut br = TLG6BitReader::new(&data);
        assert_eq!(br.get_gamma(), 1);
        assert_eq!(br.get_gamma(), 2);
        assert_eq!(br.get_gamma(), 5);
    }

    #[test]
    fn test_bitreader_roundtrip() {
        let mut bs = TLG6BitStream::new();
        bs.put_1bit(true);
        bs.put_1bit(false);
        bs.put_value(0x55, 8);
        bs.put_gamma(10);
        bs.put_value(0x123, 12);
        let data = bs.take_data();

        let mut br = TLG6BitReader::new(&data);
        assert_eq!(br.get_1bit(), true);
        assert_eq!(br.get_1bit(), false);
        assert_eq!(br.get_value(8), 0x55);
        assert_eq!(br.get_gamma(), 10);
        assert_eq!(br.get_value(12), 0x123);
    }
}
