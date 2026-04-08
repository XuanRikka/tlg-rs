pub const SLIDE_N: usize = 4096;
pub const SLIDE_M: usize = 18 + 255;

#[derive(Clone, Copy)]
struct Chain {
    prev: i32,
    next: i32,
}

pub struct SlideCompressor {
    text: Vec<u8>,
    s: usize,
    map: Box<[i32; 256 * 256]>,
    chains: Vec<Chain>,

    backup_text: Vec<u8>,
    backup_s: usize,
    backup_map: Box<[i32; 256 * 256]>,
    backup_chains: Vec<Chain>,
}

impl SlideCompressor {
    pub fn new() -> Self {
        let mut compressor = SlideCompressor {
            text: vec![0; SLIDE_N + SLIDE_M - 1],
            s: 0,
            map: Box::new([-1; 256 * 256]),
            chains: vec![
                Chain {
                    prev: -1,
                    next: -1
                };
                SLIDE_N
            ],

            backup_text: vec![0; SLIDE_N + SLIDE_M - 1],
            backup_s: 0,
            backup_map: Box::new([-1; 256 * 256]),
            backup_chains: vec![Chain { prev: -1, next: -1 }; SLIDE_N],
        };

        for i in (0..SLIDE_N).rev() {
            compressor.add_map(i);
        }

        compressor
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

            // 跳过与当前写入位置冲突的节点
            if s == place_org || s == ((place_org + 1) & (SLIDE_N - 1)) {
                head = self.chains[place_org].next;
                continue;
            }

            let mut place_idx = place_org + 2;
            let mut c_idx = 2;

            let mut lim = (if SLIDE_M < curlen { SLIDE_M } else { curlen }) + place_org;

            // 处理环形缓冲区的边界
            if lim >= SLIDE_N {
                if place_org <= s && s < SLIDE_N {
                    lim = s;
                } else if s < (lim & (SLIDE_N - 1)) {
                    lim = s + SLIDE_N;
                }
            } else if place_org <= s && s < lim {
                lim = s;
            }

            // 比较字符，不检查c_idx边界（由curlen保证）
            while place_idx < lim {
                // 安全：place_idx < lim ≤ SLIDE_N + SLIDE_M，且 cur 切片长度足够
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
                    code[codeptr] =
                        ((pos >> 8) as u8) | (((len - 3) as u8) << 4);
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
                    pos = (pos + 1) & (SLIDE_N - 1);
                }
            } else {
                if i >= input.len() {
                    break;
                }

                let c = input[i];
                i += 1;
                out.push(c);

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
            }

            mask <<= 1;
        }

        out
    }

    pub fn store(&mut self) {
        self.backup_text.copy_from_slice(&self.text);
        self.backup_s = self.s;
        self.backup_map.copy_from_slice(&*self.map);
        self.backup_chains.copy_from_slice(&self.chains);
    }

    pub fn restore(&mut self) {
        self.text.copy_from_slice(&self.backup_text);
        self.s = self.backup_s;
        self.map.copy_from_slice(&*self.backup_map);
        self.chains.copy_from_slice(&self.backup_chains);
    }
}


mod test
{
    use super::SlideCompressor;

    #[test]
    fn test_encode()
    {
        let data = "你其实是猪".repeat(32).into_bytes();

        let c = SlideCompressor::new().encode(&data);

        let test_data: &[u8] = include_bytes!("test.bin");

        assert_eq!(c, test_data);
    }

    #[test]
    fn encode_and_decode()
    {
        let data = "你其实是猪".repeat(32).into_bytes();

        let c = SlideCompressor::new().encode(&data);

        let d = SlideCompressor::new().decode(&c);

        assert_eq!(data, d);
    }

    #[test]
    fn test_decode()
    {
        let test_data: &[u8] = include_bytes!("test.bin"); // 压缩数据
        let data = "你其实是猪".repeat(32).into_bytes(); // 原始数据

        let c = SlideCompressor::new().decode(test_data); // 用压缩数据解压

        assert_eq!(c, data);
    }
}
