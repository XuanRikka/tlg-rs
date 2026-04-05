pub const SLIDE_N: usize = 4096;
pub const SLIDE_M: usize = 18 + 255;

#[derive(Clone, Copy)]
struct Chain {
    prev: i32,
    next: i32,
}

pub struct SlideCompressor {
    text: Vec<u8>,          // 循环缓冲区 + 镜像区
    s: usize,               // 当前写入位置
    map: [i32; 256 * 256],  // 2 字节 key -> 链表头
    chains: Vec<Chain>,     // 位置链表
}

impl SlideCompressor {
    pub fn new() -> Self {
        let mut compressor = SlideCompressor {
            text: vec![0; SLIDE_N + SLIDE_M],
            s: 0,
            map: [-1; 256 * 256],
            chains: vec![
                Chain {
                    prev: -1,
                    next: -1
                };
                SLIDE_N
            ],
        };

        for i in (0..SLIDE_N).rev() {
            compressor.add_map(i);
        }

        compressor
    }

    fn add_map(&mut self, index: usize) {
        let place = self.text[index] as usize
            + ((self.text[(index + 1) & (SLIDE_N - 1)] as usize) << 8);

        if self.map[place] == -1 {
            self.map[place] = index as i32;
        } else {
            let old = self.map[place] as usize;
            self.map[place] = index as i32;
            self.chains[old].prev = index as i32;
            self.chains[index].next = old as i32;
            self.chains[index].prev = -1;
        }
    }

    fn delete_map(&mut self, index: usize) {
        let next = self.chains[index].next;
        if next != -1 {
            self.chains[next as usize].prev = self.chains[index].prev;
        }

        let prev = self.chains[index].prev;
        if prev != -1 {
            self.chains[prev as usize].next = self.chains[index].next;
        } else if self.chains[index].next != -1 {
            let place = self.text[index] as usize
                + ((self.text[(index + 1) & (SLIDE_N - 1)] as usize) << 8);
            self.map[place] = self.chains[index].next;
        } else {
            let place = self.text[index] as usize
                + ((self.text[(index + 1) & (SLIDE_N - 1)] as usize) << 8);
            self.map[place] = -1;
        }

        self.chains[index].prev = -1;
        self.chains[index].next = -1;
    }

    // 查找最长匹配
    fn get_match(&self, cur: &[u8], s: usize) -> Option<(usize, usize)> {
        if cur.len() < 3 {
            return None;
        }

        let mut max_len = 0;
        let mut pos = 0usize;
        let key = cur[0] as usize + ((cur[1] as usize) << 8);
        let head = self.map[key];
        if head == -1 {
            return None;
        }

        let mut curlen = cur.len();
        curlen -= 1;

        let mut p = head;
        while p != -1 {
            let place_org = p as usize;
            if s == place_org || s == ((place_org + 1) & (SLIDE_N - 1)) {
                p = self.chains[place_org].next;
                continue;
            }

            let mut place_idx = place_org + 2;
            let mut lim = (if SLIDE_M < curlen { SLIDE_M } else { curlen }) + place_org;
            let mut c_idx = 2usize;

            if lim >= SLIDE_N {
                if place_org <= s && s < SLIDE_N {
                    lim = s;
                } else if s < (lim & (SLIDE_N - 1)) {
                    lim = s + SLIDE_N;
                }
            } else if place_org <= s && s < lim {
                lim = s;
            }

            while place_idx < lim && self.text[place_idx] == cur[c_idx] {
                place_idx += 1;
                c_idx += 1;
            }

            let matchlen = place_idx - place_org;
            if matchlen > max_len {
                pos = place_org;
                max_len = matchlen;
            }
            if matchlen == SLIDE_M {
                return Some((pos, max_len));
            }

            p = self.chains[place_org].next;
        }

        if max_len >= 3 {
            Some((pos, max_len))
        } else {
            None
        }
    }

    pub fn encode(&mut self, input: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut mask: u8 = 1;
        let mut code: Vec<u8> = vec![0]; // 第一位是标记位

        let mut i = 0usize;
        while i < input.len() {
            if let Some((pos, len)) = self.get_match(&input[i..], self.s) {
                code[0] |= mask;

                if len >= 18 {
                    code.push((pos & 0xff) as u8);
                    code.push(((pos & 0xf00) >> 8) as u8 | 0xf0);
                    code.push((len - 18) as u8);
                } else {
                    code.push((pos & 0xff) as u8);
                    code.push(((pos & 0xf00) >> 8) as u8 | ((len - 3) << 4) as u8);
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
                code.push(c);
                self.s = (self.s + 1) & (SLIDE_N - 1);
                i += 1;
            }

            mask <<= 1;
            if mask == 0 {
                out.extend_from_slice(&code);
                code = vec![0];
                mask = 1;
            }
        }

        if mask != 1 {
            out.extend_from_slice(&code);
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

            if code & mask != 0 {
                // 匹配引用
                if i + 1 >= input.len() {
                    break; // 防越界
                }

                let b1 = input[i] as usize;
                let b2 = input[i + 1] as usize;
                let mut pos = (b2 & 0x0f) << 8 | b1;
                let mut len = (b2 >> 4) & 0x0f;
                i += 2;

                if len == 0xf {
                    // 长匹配 18+
                    if i >= input.len() {
                        break;
                    }
                    len = 18 + input[i] as usize;
                    i += 1;
                } else {
                    len += 3; // 短匹配 3~17
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
                // 字面值
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
            if mask == 0 {
                mask = 0;
            }
        }

        out
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
