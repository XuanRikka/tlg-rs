// ---------------------------------------------------------------------------
// TLG6 bit-stream (writes bits LSB-first into a byte buffer)
// ---------------------------------------------------------------------------

pub(super) struct TLG6BitStream {
    buf: Vec<u8>,
    byte_pos: usize,
    bit_pos: u8,
    byte_capacity: usize,
}

impl TLG6BitStream {
    pub(super) fn new() -> Self {
        TLG6BitStream {
            buf: Vec::new(),
            byte_pos: 0,
            bit_pos: 0,
            byte_capacity: 0,
        }
    }

    pub(super) fn get_byte_pos(&self) -> i32 {
        self.byte_pos as i32
    }

    pub(super) fn get_bit_length(&self) -> u32 {
        self.byte_pos as u32 * 8 + self.bit_pos as u32
    }

    fn ensure(&mut self) {
        if self.byte_pos >= self.byte_capacity {
            self.byte_capacity = self.byte_pos + 0x1000;
            self.buf.resize(self.byte_capacity, 0);
        }
    }

    pub(super) fn put_1bit(&mut self, b: bool) {
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

    pub(super) fn put_value(&mut self, mut v: u32, mut len: u32) {
        while len > 0 {
            self.put_1bit((v & 1) != 0);
            v >>= 1;
            len -= 1;
        }
    }

    pub(super) fn put_gamma(&mut self, mut v: u32) {
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

    pub(super) fn take_data(&mut self) -> Vec<u8> {
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
}
