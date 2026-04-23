use super::{SLIDE_N, SLIDE_M};

#[derive(Clone, Copy)]
struct Chain {
    prev: i32,
    next: i32,
}

pub struct SlideEncoder {
    text: Box<[u8; SLIDE_N + SLIDE_M - 1]>,
    s: usize,
    map: Box<[i32; 256 * 256]>,
    chains: Vec<Chain>,

    backup_text: Box<[u8; SLIDE_N + SLIDE_M - 1]>,
    backup_s: usize,
    backup_map: Box<[i32; 256 * 256]>,
    backup_chains: Vec<Chain>,
}

impl SlideEncoder {
    pub fn new() -> Self {
        let mut enc = SlideEncoder {
            text: Box::new([0; SLIDE_N + SLIDE_M - 1]),
            s: 0,
            map: Box::new([-1; 256 * 256]),
            chains: vec![Chain { prev: -1, next: -1 }; SLIDE_N],

            backup_text: Box::new([0; SLIDE_N + SLIDE_M - 1]),
            backup_s: 0,
            backup_map: Box::new([-1; 256 * 256]),
            backup_chains: vec![Chain { prev: -1, next: -1 }; SLIDE_N],
        };

        for i in (0..SLIDE_N).rev() {
            enc.add_map(i);
        }

        enc
    }

    #[inline]
    fn add_map(&mut self, index: usize) {
        let key = self.text[index] as usize
            | ((self.text[(index + 1) & (SLIDE_N - 1)] as usize) << 8);

        let head = self.map[key];
        if head == -1 {
            self.map[key] = index as i32;
        } else {
            let old = head as usize;
            self.map[key] = index as i32;
            self.chains[old].prev = index as i32;
            self.chains[index].next = old as i32;
            self.chains[index].prev = -1;
        }
    }

    #[inline]
    fn delete_map(&mut self, index: usize) {
        let next = self.chains[index].next;
        if next != -1 {
            self.chains[next as usize].prev = self.chains[index].prev;
        }

        let prev = self.chains[index].prev;
        if prev != -1 {
            self.chains[prev as usize].next = self.chains[index].next;
        } else {
            let key = self.text[index] as usize
                | ((self.text[(index + 1) & (SLIDE_N - 1)] as usize) << 8);

            if next != -1 {
                self.map[key] = next;
            } else {
                self.map[key] = -1;
            }
        }

        self.chains[index].prev = -1;
        self.chains[index].next = -1;
    }

    fn get_match(&self, cur: &[u8], remaining_len: usize, s: usize) -> Option<(usize, usize)> {
        if remaining_len < 3 {
            return None;
        }

        let key = cur[0] as usize | ((cur[1] as usize) << 8);
        let mut head = self.map[key];
        if head == -1 {
            return None;
        }

        let mut max_len = 0;
        let mut pos = 0usize;
        let curlen = remaining_len - 1;

        while head != -1 {
            let place_org = head as usize;

            if s == place_org || s == ((place_org + 1) & (SLIDE_N - 1)) {
                head = self.chains[place_org].next;
                continue;
            }

            let mut place_idx = place_org + 2;
            let mut c_idx = 2;

            let mut lim = (if SLIDE_M < curlen { SLIDE_M } else { curlen }) + place_org;

            if lim >= SLIDE_N {
                if place_org <= s && s < SLIDE_N {
                    lim = s;
                } else if s < (lim & (SLIDE_N - 1)) {
                    lim = s + SLIDE_N;
                }
            } else if place_org <= s && s < lim {
                lim = s;
            }

            while place_idx < lim {
                unsafe {
                    if *self.text.get_unchecked(place_idx) != *cur.get_unchecked(c_idx) {
                        break;
                    }
                }
                place_idx += 1;
                c_idx += 1;
            }

            let match_len = place_idx - place_org;
            if match_len > max_len {
                max_len = match_len;
                pos = place_org;
                if match_len == SLIDE_M {
                    return Some((pos, max_len));
                }
            }

            head = self.chains[place_org].next;
        }

        if max_len >= 3 {
            Some((pos, max_len))
        } else {
            None
        }
    }

    pub fn encode(&mut self, input: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();

        let mut code = [0u8; 40];
        let mut codeptr = 1usize;
        let mut mask: u8 = 1;

        let mut i = 0;

        while i < input.len() {
            if let Some((pos, len)) = self.get_match(&input[i..], input.len() - i, self.s) {
                code[0] |= mask;

                if len >= 18 {
                    code[codeptr] = (pos & 0xff) as u8;
                    codeptr += 1;
                    code[codeptr] = ((pos >> 8) as u8) | 0xf0;
                    codeptr += 1;
                    code[codeptr] = (len - 18) as u8;
                    codeptr += 1;
                } else {
                    code[codeptr] = (pos & 0xff) as u8;
                    codeptr += 1;
                    code[codeptr] = ((pos >> 8) as u8) | (((len - 3) as u8) << 4);
                    codeptr += 1;
                }

                for _ in 0..len {
                    let c = input[i];
                    let prev = (self.s + SLIDE_N - 1) & (SLIDE_N - 1);

                    self.delete_map(prev);
                    self.delete_map(self.s);

                    if self.s < SLIDE_M - 1 {
                        self.text[self.s + SLIDE_N] = c;
                    }
                    self.text[self.s] = c;

                    self.add_map(prev);
                    self.add_map(self.s);

                    self.s = (self.s + 1) & (SLIDE_N - 1);
                    i += 1;
                }
            } else {
                let c = input[i];
                let prev = (self.s + SLIDE_N - 1) & (SLIDE_N - 1);

                self.delete_map(prev);
                self.delete_map(self.s);

                if self.s < SLIDE_M - 1 {
                    self.text[self.s + SLIDE_N] = c;
                }
                self.text[self.s] = c;

                self.add_map(prev);
                self.add_map(self.s);

                code[codeptr] = c;
                codeptr += 1;

                self.s = (self.s + 1) & (SLIDE_N - 1);
                i += 1;
            }

            mask <<= 1;

            if mask == 0 {
                out.extend_from_slice(&code[..codeptr]);
                code[0] = 0;
                codeptr = 1;
                mask = 1;
            }
        }

        if mask != 1 {
            out.extend_from_slice(&code[..codeptr]);
        }

        out
    }

    pub fn store(&mut self) {
        self.backup_text.copy_from_slice(self.text.as_slice());
        self.backup_s = self.s;
        self.backup_map.copy_from_slice(&*self.map);
        self.backup_chains.copy_from_slice(&self.chains);
    }

    pub fn restore(&mut self) {
        self.text.copy_from_slice(self.backup_text.as_slice());
        self.s = self.backup_s;
        self.map.copy_from_slice(&*self.backup_map);
        self.chains.copy_from_slice(&self.backup_chains);
    }
}
