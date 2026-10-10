use crate::SLIDE_N;

const MASK: usize = SLIDE_N - 1;
const SLACK: usize = 8;

pub struct SlideDecoder {
    text: Box<[u8; SLIDE_N + SLACK]>,
    s: usize,
}

impl SlideDecoder {
    pub fn new() -> Self {
        SlideDecoder {
            text: Box::new([0; SLIDE_N + SLACK]),
            s: 0,
        }
    }

    pub fn init_with_text(&mut self, data: &[u8]) {
        let len = data.len().min(SLIDE_N);
        self.text[..len].copy_from_slice(&data[..len]);
    }

    pub fn decode(&mut self, input: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        self.decode_into(input, &mut out);
        out
    }

    pub fn decode_into(&mut self, input: &[u8], out: &mut Vec<u8>) {
        debug_assert!(self.s < SLIDE_N);
        let s0 = self.s & MASK;
        let start = out.len();
        let text = &*self.text;

        let mut i = 0;

        'groups: while i < input.len() {
            let code = input[i];
            i += 1;

            // Fast path: a group of eight literals.
            if code == 0 && i + 8 <= input.len() {
                out.extend_from_slice(&input[i..i + 8]);
                i += 8;
                continue;
            }

            for bit in 0..8 {
                if i >= input.len() {
                    break 'groups;
                }

                if (code >> bit) & 1 == 0 {
                    out.push(input[i]);
                    i += 1;
                    continue;
                }

                if i + 1 >= input.len() {
                    break 'groups;
                }
                let b1 = input[i] as usize;
                let b2 = input[i + 1] as usize;
                i += 2;

                let pos = ((b2 & 0x0f) << 8) | b1;
                let nibble = b2 >> 4;
                let len = if nibble == 0xf {
                    if i >= input.len() {
                        break 'groups;
                    }
                    let l = 18 + input[i] as usize;
                    i += 1;
                    l
                } else {
                    nibble + 3
                };

                let produced = out.len() - start;
                let s = (s0 + produced) & MASK;
                let mut d = s.wrapping_sub(pos) & MASK;
                if d == 0 {
                    d = SLIDE_N;
                }

                out.reserve(len + 16);

                let mut remaining = len;
                let mut w = out.len();
                unsafe {
                    let base = out.as_mut_ptr();
                    let ring = text.as_ptr();

                    if produced < d {
                        let r = remaining.min(d - produced);
                        let first = r.min(SLIDE_N - pos);
                        let mut k = 0;
                        while k < first {
                            core::ptr::copy_nonoverlapping(ring.add(pos + k), base.add(w + k), 8);
                            k += 8;
                        }
                        w += first;
                        let rest = r - first;
                        let mut k = 0;
                        while k < rest {
                            core::ptr::copy_nonoverlapping(ring.add(k), base.add(w + k), 8);
                            k += 8;
                        }
                        w += rest;
                        remaining -= r;
                    }

                    if remaining > 0 && d >= 8 {
                        let src = base.add(w - d);
                        let dst = base.add(w);
                        let mut k = 0;
                        while k < remaining {
                            core::ptr::copy_nonoverlapping(src.add(k), dst.add(k), 8);
                            k += 8;
                        }
                        w += remaining;
                        remaining = 0;
                    }
                    out.set_len(w);
                }

                if remaining > 0 {
                    let src = out.len() - d;
                    while remaining > 0 {
                        let n = remaining.min(out.len() - src);
                        out.extend_from_within(src..src + n);
                        remaining -= n;
                    }
                }
            }
        }

        let produced = out.len() - start;
        let n = produced.min(SLIDE_N);
        if n > 0 {
            let tail = &out[out.len() - n..];
            let slot = (s0 + produced - n) & MASK;
            let first = n.min(SLIDE_N - slot);
            self.text[slot..slot + first].copy_from_slice(&tail[..first]);
            self.text[..n - first].copy_from_slice(&tail[first..]);
        }
        self.s = (s0 + produced) & MASK;
    }
}