use crate::{SLIDE_M, SLIDE_N};

/// 对该变量按位和等于对SLIDE_N取模
const MASK: usize = SLIDE_N - 1;
const HASH_BITS: u32 = 15;
const KEYS: usize = 1 << HASH_BITS;

/// 黄金乘数混合
#[inline(always)]
fn hash3(b0: u8, b1: u8, b2: u8) -> usize {
    let v = b0 as u32 | ((b1 as u32) << 8) | ((b2 as u32) << 16);
    (v.wrapping_mul(0x9E37_79B1) >> (32 - HASH_BITS)) as usize
}
const REBASE_AT: u32 = 1 << 31;

pub struct SlideEncoder {
    text: Box<[u8; SLIDE_N + SLIDE_M - 1]>,
    /// 下一个待写入字节的绝对位置
    /// 从SLIDE_N开始
    pos: u32,
    /// Number of input bytes ever consumed, saturated at `SLIDE_N`.
    steps: usize,
    /// 通过 hash 的 key 得到最新一个节点的绝对位置，也就是 `map[key]` 的值是链表的起始节点
    map: Box<[u32; KEYS]>,
    /// 该表的每一个位置对应一个滑动窗口的实际位置，对应位置的值则是下一个节点的绝对位置
    prev: Box<[u32; SLIDE_N]>,
    rebase_at: u32,

    // Snapshot taken by `store()`. `map`/`prev` are a pure function of
    // (text, cursor, steps) and are rebuilt by `restore()`.
    snap_text: Box<[u8; SLIDE_N + SLIDE_M - 1]>,
    snap_s: usize,
    snap_steps: usize,
}

impl SlideEncoder {
    pub fn new() -> Self {
        SlideEncoder {
            text: Box::new([0; SLIDE_N + SLIDE_M - 1]),
            pos: SLIDE_N as u32,
            steps: 0,
            map: vec![0u32; KEYS].into_boxed_slice().try_into().unwrap(),
            prev: Box::new([0; SLIDE_N]),
            rebase_at: REBASE_AT,

            snap_text: Box::new([0; SLIDE_N + SLIDE_M - 1]),
            snap_s: 0,
            snap_steps: 0,
        }
    }

    #[inline]
    fn cursor(&self) -> usize {
        self.pos as usize & MASK
    }

    #[inline(always)]
    fn push(&mut self, c: u8) {
        let slot = self.pos as usize & MASK;

        if slot < SLIDE_M - 1 {
            // SAFETY: slot 计算之后不可能会有越界值
            unsafe {
                *self.text.get_unchecked_mut(slot + SLIDE_N) = c;
            }
        }
        // SAFETY: 同上
        unsafe {
            *self.text.get_unchecked_mut(slot) = c;
        }

        if self.pos >= SLIDE_N as u32 + 1 {
            let q = slot.wrapping_sub(2) & MASK;
            // SAFETY: q 的计算也不可能产生越界值，而显然 hash3 的计算不可能产生 map 的越界值
            unsafe {
                let key = hash3(
                    *self.text.get_unchecked(q),
                    *self.text.get_unchecked((q + 1) & MASK),
                    c
                );
                *self.prev.get_unchecked_mut(q) = *self.map.get_unchecked(key);
                *self.map.get_unchecked_mut(key) = self.pos - 2;
            }
        }

        self.pos += 1;
    }

    /// Shifts all absolute positions down so `pos` falls back into
    /// `[SLIDE_N, 2 * SLIDE_N)`. Entries that would go negative were already
    /// out of the window and become 0 ("none").
    fn rebase(&mut self) {
        let shift = (self.pos - SLIDE_N as u32) & !(MASK as u32);
        if shift == 0 {
            return;
        }
        for e in self.map.iter_mut() {
            *e = e.saturating_sub(shift);
        }
        for e in self.prev.iter_mut() {
            *e = e.saturating_sub(shift);
        }
        self.pos -= shift;
    }

    #[inline(always)]
    fn probe(&self, cur: &[u8], p: usize, cap: usize, max_len: usize) -> usize {
        if cap < 3 || cap <= max_len {
            return 0;
        }
        if max_len >= 3 {
            debug_assert!(p + max_len < self.text.len());
            // SAFETY:
            // - max_len < cap <= SLIDE_M，p < SLIDE_N
            //   => p + max_len < SLIDE_N + SLIDE_M - 1 = self.text.len()
            // - max_len < cap <= remaining_len - 1 < cur.len()
            if unsafe {
                *self.text.get_unchecked(p + max_len) } != unsafe { *cur.get_unchecked(max_len)
            }  {
                return 0;
            }
        }

        let text = &self.text[..];
        let mut i = 0;
        let text_ptr = self.text.as_ptr();
        let cur_ptr = cur.as_ptr();
        while i < cap {
            if i + 8 <= cur.len() && p + i + 8 <= text.len()
            {
                // SAFETY:
                // - i + 8 <= cur.len() => cur[i..i+8] 合法
                // - p + i + 8 <= text.len() => text[p+i..p+i+8] 合法
                // 切片长度都是 8，因此 try_into 到 [u8; 8] 一定成功。
                let a = u64::from_le(unsafe {
                    (text_ptr.add(p + i) as *const u64).read_unaligned()
                });
                let b = u64::from_le(unsafe {
                    (cur_ptr.add(i) as *const u64).read_unaligned()
                });
                let x = a ^ b;
                if x == 0 {
                    i += 8;
                } else {
                    i += (x.trailing_zeros() / 8) as usize;
                    break;
                }
            }
            // SAFETY:
            // - i < cap <= remaining_len - 1 < cur.len()
            // - p + i < p + cap <= SLIDE_N + SLIDE_M - 1 = text.len()
            // 因此 text[p+i] 和 cur[i] 都是合法下标。
            else if unsafe { *text_ptr.add(p + i) == *cur_ptr.add(i) }
            {
                i += 1;
            }
            else
            {
                break;
            }
        }
        let len = i.min(cap);
        if len > max_len && len >= 3 { len } else { 0 }
    }

    fn get_match(&self, cur: &[u8], remaining_len: usize) -> Option<(usize, usize)> {
        if remaining_len < 3 {
            return None;
        }

        // SAFETY: 观察调用点，不难看出这里是安全的
        let b0 = unsafe { *cur.get_unchecked(0) };
        let b1 = unsafe { *cur.get_unchecked(1) };
        let b2 = unsafe { *cur.get_unchecked(2) };
        let key = hash3(b0, b1, b2);
        let pos = self.pos;
        let base_cap = SLIDE_M.min(remaining_len - 1);

        let mut max_len = 0;
        let mut best = 0usize;

        // SAFETY: hash3 不可能产出对于 map 越界的值
        let mut e = unsafe { *self.map.get_unchecked(key) };
        loop {
            let d = pos.wrapping_sub(e) as usize;
            if d >= SLIDE_N {
                break;
            }
            let p = e as usize & MASK;
            let len = self.probe(cur, p, base_cap.min(d), max_len);
            if len > 0 {
                max_len = len;
                best = p;
                if len == SLIDE_M {
                    return Some((best, max_len));
                }
            }
            // SAFETY: p 的计算保证了不可能越界
            e = unsafe { *self.prev.get_unchecked(p) };
        }

        if self.steps < SLIDE_N && b0 == 0 && b1 == 0 {
            let t = self.steps;
            let lo = t + 1;
            let hi = SLIDE_N - 2;
            let lo = if b2 == 0 { lo } else { lo.max(hi) };
            for p in lo..=hi {
                let d = SLIDE_N + t - p;
                let len = self.probe(cur, p, base_cap.min(d), max_len);
                if len > 0 {
                    max_len = len;
                    best = p;
                    if len == SLIDE_M {
                        return Some((best, max_len));
                    }
                }
            }
        }

        if max_len >= 3 { Some((best, max_len)) } else { None }
    }

    pub fn encode(&mut self, input: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();

        let mut code = [0u8; 40];
        let mut codeptr = 1usize;
        let mut mask: u8 = 1;

        let mut i = 0;

        while i < input.len() {
            if self.pos >= self.rebase_at {
                self.rebase();
            }

            if let Some((pos, len)) = self.get_match(&input[i..], input.len() - i) {
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
                    self.push(input[i]);
                    i += 1;
                }
                self.steps = (self.steps + len).min(SLIDE_N);
            } else {
                let c = input[i];
                self.push(c);
                self.steps = (self.steps + 1).min(SLIDE_N);

                code[codeptr] = c;
                codeptr += 1;
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
        self.snap_text.copy_from_slice(self.text.as_slice());
        self.snap_s = self.cursor();
        self.snap_steps = self.steps;
    }

    pub fn restore(&mut self) {
        self.text.copy_from_slice(self.snap_text.as_slice());
        self.pos = (SLIDE_N + self.snap_s) as u32;
        self.steps = self.snap_steps;
        self.rebuild_maps();
    }

    /// Rebuild `map`/`prev` so that chain walks are identical to those of an
    /// encoder that organically reached state (text, cursor, steps).
    ///
    /// Chains hold every position whose 3-byte key is complete, in insertion
    /// order: with `pos = N + s` those are absolute positions
    ///   * steps <  N: `N-1` (inserted by the second byte), then
    ///                 `N ..= pos-3` (slots `0 ..= s-3`);
    ///   * steps >= N: `pos-N+1 ..= pos-3`.
    /// Inserting them oldest-first reproduces the structure exactly.
    fn rebuild_maps(&mut self) {
        self.map.fill(0);
        self.prev.fill(0);

        let pos = self.pos as usize;
        let first = if self.steps >= SLIDE_N {
            pos - SLIDE_N + 1
        } else if self.steps < 2 {
            return;
        } else {
            SLIDE_N - 1
        };

        for abs in first..=pos - 3 {
            let q = abs & MASK;
            let key = hash3(self.text[q], self.text[(q + 1) & MASK], self.text[(q + 2) & MASK]);
            self.prev[q] = self.map[key];
            self.map[key] = abs as u32;
        }
    }
}