use super::{SLIDE_N, SLIDE_M};

pub struct SlideDecoder {
    text: Box<[u8; SLIDE_N + SLIDE_M - 1]>,
    s: usize,
}

impl SlideDecoder {
    pub fn new() -> Self {
        SlideDecoder {
            text: Box::new([0; SLIDE_N + SLIDE_M - 1]),
            s: 0,
        }
    }

    pub fn init_with_text(&mut self, data: &[u8]) {
        let len = data.len().min(SLIDE_N + SLIDE_M - 1);
        self.text[..len].copy_from_slice(&data[..len]);
    }

    pub fn decode(&mut self, input: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();

        let mut i = 0;
        let mut mask: u8 = 0;
        let mut code: u8 = 0;

        while i < input.len() {
            if mask == 0 {
                code = input[i];
                i += 1;
                mask = 1;
            }

            if (code & mask) != 0 {
                if i + 1 >= input.len() {
                    break;
                }

                let b1 = input[i] as usize;
                let b2 = input[i + 1] as usize;
                i += 2;

                let mut pos = ((b2 & 0x0f) << 8) | b1;
                let mut len = (b2 >> 4) & 0x0f;

                if len == 0xf {
                    if i >= input.len() {
                        break;
                    }
                    len = 18 + input[i] as usize;
                    i += 1;
                } else {
                    len += 3;
                }

                for _ in 0..len {
                    let c = self.text[pos];
                    out.push(c);

                    if self.s < SLIDE_M - 1 {
                        self.text[self.s + SLIDE_N] = c;
                    }
                    self.text[self.s] = c;

                    self.s = (self.s + 1) & (SLIDE_N - 1);
                    pos = (pos + 1) & (SLIDE_N - 1);
                }
            } else {
                if i >= input.len() {
                    break;
                }

                let c = input[i];
                i += 1;
                out.push(c);

                if self.s < SLIDE_M - 1 {
                    self.text[self.s + SLIDE_N] = c;
                }
                self.text[self.s] = c;

                self.s = (self.s + 1) & (SLIDE_N - 1);
            }

            mask <<= 1;
        }

        out
    }
}
