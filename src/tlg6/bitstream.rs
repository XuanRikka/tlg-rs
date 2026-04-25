// ---------------------------------------------------------------------------
// TLG6 bit-stream (writes bits LSB-first into a byte buffer)
// ---------------------------------------------------------------------------

pub struct TLG6BitStream {
    buf: Vec<u8>,
    byte_pos: usize,
    bit_pos: u8,
    byte_capacity: usize,
}

impl TLG6BitStream {
    pub fn new() -> Self {
        TLG6BitStream {
            buf: Vec::new(),
            byte_pos: 0,
            bit_pos: 0,
            byte_capacity: 0,
        }
    }

    pub fn get_byte_pos(&self) -> i32 {
        self.byte_pos as i32
    }

    pub fn get_bit_length(&self) -> u32 {
        self.byte_pos as u32 * 8 + self.bit_pos as u32
    }

    fn ensure(&mut self) {
        if self.byte_pos >= self.byte_capacity {
            self.byte_capacity = self.byte_pos + 0x1000;
            self.buf.resize(self.byte_capacity, 0);
        }
    }

    pub fn put_1bit(&mut self, b: bool) {
        self.ensure();
        if b {
            self.buf[self.byte_pos] |= 1 << self.bit_pos;
        }
        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
    }

    pub fn put_value(&mut self, mut v: u32, mut len: u32) {
        while len > 0 {
            self.put_1bit((v & 1) != 0);
            v >>= 1;
            len -= 1;
        }
    }

    pub fn put_gamma(&mut self, mut v: u32) {
        let mut t = v >> 1;
        let mut cnt = 0u32;
        while t > 0 {
            self.put_1bit(false);
            t >>= 1;
            cnt += 1;
        }
        self.put_1bit(true);
        while cnt > 0 {
            self.put_1bit((v & 1) != 0);
            v >>= 1;
            cnt -= 1;
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
        self.byte_capacity = 0;
        data
    }
}

// ---------------------------------------------------------------------------
// TLG6 bit-stream reader (reads bits LSB-first from a byte slice)
// ---------------------------------------------------------------------------

pub struct TLG6BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: u8,
}

impl<'a> TLG6BitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        TLG6BitReader {
            data,
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

    /// Returns true if the reader has exhausted its data buffer.
    #[inline(always)]
    pub fn exhausted(&self) -> bool {
        self.byte_pos >= self.data.len()
    }

    pub fn skip_bits(&mut self, n: u32) {
        let total = self.bit_pos as u32 + n;
        self.byte_pos += (total >> 3) as usize;
        self.bit_pos = (total & 7) as u8;
    }

    pub fn get_1bit(&mut self) -> bool {
        if self.exhausted() {
            return false;
        }
        let b = (self.data[self.byte_pos] >> self.bit_pos) & 1 != 0;
        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
        b
    }

    pub fn get_value(&mut self, mut len: u32) -> u32 {
        if len == 0 || self.exhausted() {
            return 0;
        }

        let mut v = 0u32;
        let mut shift = 0u32;

        while len > 0 && !self.exhausted() {
            let bits_available = 8 - self.bit_pos;
            let bits_to_read = len.min(bits_available as u32);

            let mask = ((1u32 << bits_to_read) - 1) as u8;
            let chunk = ((self.data[self.byte_pos] >> self.bit_pos) & mask) as u32;

            v |= chunk << shift;

            shift += bits_to_read;
            len -= bits_to_read;
            self.bit_pos += bits_to_read as u8;

            if self.bit_pos == 8 {
                self.bit_pos = 0;
                self.byte_pos += 1;
            }
        }

        v
    }

    /// Read a gamma code, returns the value (always >= 1)
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

    /// Read a raw byte at an absolute offset (no state change)
    pub fn peek_byte_at(&self, byte_offset: usize) -> u8 {
        if byte_offset >= self.data.len() {
            0
        } else {
            self.data[byte_offset]
        }
    }

    /// Peek at a 32-bit value starting from the current bit position
    pub fn peek_u32_le(&self) -> u32 {
        let byte_offset = self.byte_pos;
        if byte_offset >= self.data.len() {
            return 0;
        }
        let remaining = self.data.len() - byte_offset;
        if remaining >= 4 {
            let b = &self.data[byte_offset..];
            let raw = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
            raw >> self.bit_pos
        } else {
            let mut raw = 0u32;
            for i in 0..remaining.min(4) {
                raw |= (self.data[byte_offset + i] as u32) << (i * 8);
            }
            raw >> self.bit_pos
        }
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
