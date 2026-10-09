#![allow(dead_code)]

pub const N: usize = 4096;
pub const M: usize = 18 + 255;

pub struct XorShift(pub u64);

impl XorShift {
    pub fn new(seed: u64) -> Self {
        XorShift(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

pub fn fnv1a(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub fn random_data(rng: &mut XorShift, len: usize, alphabet: u64) -> Vec<u8> {
    (0..len).map(|_| rng.below(alphabet) as u8).collect()
}

pub fn text_like(rng: &mut XorShift, len: usize) -> Vec<u8> {
    const WORDS: &[&str] = &[
        "the",
        "of",
        "and",
        "to",
        "in",
        "is",
        "that",
        "it",
        "was",
        "for",
        "on",
        "are",
        "with",
        "window",
        "dictionary",
        "literal",
        "pointer",
        "encoder",
        "decoder",
        "compression",
    ];
    let mut out = Vec::with_capacity(len + 16);
    while out.len() < len {
        out.extend_from_slice(WORDS[rng.below(WORDS.len() as u64) as usize].as_bytes());
        out.push(if rng.below(10) == 0 { b'\n' } else { b' ' });
    }
    out.truncate(len);
    out
}

pub fn periodic(rng: &mut XorShift, len: usize) -> Vec<u8> {
    let period_len = 1 + rng.below(60) as usize;
    let period = random_data(rng, period_len, 4);
    (0..len)
        .map(|i| {
            if rng.below(150) == 0 {
                rng.below(256) as u8
            } else {
                period[i % period_len]
            }
        })
        .collect()
}

pub fn runs(rng: &mut XorShift, len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len + 600);
    while out.len() < len {
        let b = rng.below(256) as u8;
        let n = 1 + rng.below(600) as usize;
        out.resize(out.len() + n, b);
    }
    out.truncate(len);
    out
}

pub fn mixed(rng: &mut XorShift, len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len + 3000);
    while out.len() < len {
        let seg = 1 + rng.below(3000) as usize;
        let part = match rng.below(5) {
            0 => random_data(rng, seg, 256),
            1 => random_data(rng, seg, 3),
            2 => text_like(rng, seg),
            3 => periodic(rng, seg),
            _ => runs(rng, seg),
        };
        out.extend_from_slice(&part);
    }
    out.truncate(len);
    out
}

pub const KINDS: usize = 10;

pub fn data_for(kind: usize, rng: &mut XorShift, len: usize) -> Vec<u8> {
    match kind {
        0 => random_data(rng, len, 1),
        1 => random_data(rng, len, 2),
        2 => random_data(rng, len, 3),
        3 => random_data(rng, len, 4),
        4 => random_data(rng, len, 16),
        5 => random_data(rng, len, 256),
        6 => text_like(rng, len),
        7 => periodic(rng, len),
        8 => runs(rng, len),
        _ => mixed(rng, len),
    }
}

pub fn corpora(size: usize) -> Vec<(&'static str, Vec<u8>)> {
    let mut rng = XorShift::new(0xC0FFEE);
    vec![
        ("zeros", vec![0u8; size]),
        ("ones", vec![0xFFu8; size]),
        ("alpha2", random_data(&mut rng, size, 2)),
        ("alpha3", random_data(&mut rng, size, 3)),
        ("alpha16", random_data(&mut rng, size, 16)),
        ("random", random_data(&mut rng, size, 256)),
        ("text", text_like(&mut rng, size)),
        ("periodic", periodic(&mut rng, size)),
        ("runs", runs(&mut rng, size)),
        ("mixed", mixed(&mut rng, size)),
    ]
}

pub struct RefDecoder {
    ring: Vec<u8>,
    s: usize,
}

impl RefDecoder {
    pub fn new() -> Self {
        RefDecoder {
            ring: vec![0; N],
            s: 0,
        }
    }

    pub fn init_with_text(&mut self, data: &[u8]) {
        let len = data.len().min(N);
        self.ring[..len].copy_from_slice(&data[..len]);
    }

    /// Decodes one self-contained chunk (starting on a flag byte) and returns
    /// false if it was truncated in the middle of a token.
    pub fn decode(&mut self, input: &[u8]) -> (Vec<u8>, bool) {
        let mut out = Vec::new();
        let mut i = 0;
        while i < input.len() {
            let flags = input[i];
            i += 1;
            for bit in 0..8 {
                if i >= input.len() {
                    break;
                }
                if (flags >> bit) & 1 == 1 {
                    if i + 2 > input.len() {
                        return (out, false);
                    }
                    let b1 = input[i] as usize;
                    let b2 = input[i + 1] as usize;
                    i += 2;
                    let mut pos = ((b2 & 0x0f) << 8) | b1;
                    let nib = b2 >> 4;
                    let len = if nib == 15 {
                        if i >= input.len() {
                            return (out, false);
                        }
                        let ext = input[i] as usize;
                        i += 1;
                        18 + ext
                    } else {
                        nib + 3
                    };
                    for _ in 0..len {
                        let c = self.ring[pos];
                        out.push(c);
                        self.ring[self.s] = c;
                        self.s = (self.s + 1) % N;
                        pos = (pos + 1) % N;
                    }
                } else {
                    let c = input[i];
                    i += 1;
                    out.push(c);
                    self.ring[self.s] = c;
                    self.s = (self.s + 1) % N;
                }
            }
        }
        (out, true)
    }
}

/// Naive encoder modelling the match-selection rule without hash chains.
/// After steps consumed bytes (saturating at N), candidates are visited in
/// chain order, most recently inserted first:
///   steps == 0      : 0, 1, 2, ..., N-1
///   0 < steps < N   : steps-1, ..., 1, 0, N-1, steps, steps+1, ..., N-2
///   steps >= N      : s-1, s-2, ..., s (cyclic, descending)
pub struct RefEncoder {
    ring: Vec<u8>,
    s: usize,
    steps: usize,
}

impl RefEncoder {
    pub fn new() -> Self {
        RefEncoder {
            ring: vec![0; N],
            s: 0,
            steps: 0,
        }
    }

    fn candidate(&self, k: usize) -> usize {
        if self.steps == 0 {
            k
        } else if self.steps < N {
            let t = self.steps;
            if k < t {
                t - 1 - k
            } else if k == t {
                N - 1
            } else {
                t + (k - t - 1)
            }
        } else {
            (self.s + N - 1 - k) % N
        }
    }

    fn find_match(&self, cur: &[u8]) -> Option<(usize, usize)> {
        if cur.len() < 3 {
            return None;
        }
        let curlen = cur.len() - 1;
        let mut best_len = 0;
        let mut best_pos = 0;

        for k in 0..N {
            let p = self.candidate(k);
            if p == self.s || (p + 1) % N == self.s {
                continue;
            }
            if self.ring[p] != cur[0] || self.ring[(p + 1) % N] != cur[1] {
                continue;
            }
            let dist = (self.s + N - p) % N;
            let cap = M.min(curlen).min(dist);
            let mut len = 2;
            while len < cap && self.ring[(p + len) % N] == cur[len] {
                len += 1;
            }
            if len > best_len {
                best_len = len;
                best_pos = p;
                if len == M {
                    break;
                }
            }
        }

        if best_len >= 3 {
            Some((best_pos, best_len))
        } else {
            None
        }
    }

    fn push(&mut self, c: u8) {
        self.ring[self.s] = c;
        self.s = (self.s + 1) % N;
        self.steps = (self.steps + 1).min(N);
    }

    pub fn encode(&mut self, input: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut group: Vec<u8> = vec![0];
        let mut items = 0;
        let mut i = 0;

        while i < input.len() {
            if let Some((pos, len)) = self.find_match(&input[i..]) {
                group[0] |= 1 << items;
                if len >= 18 {
                    group.push((pos & 0xff) as u8);
                    group.push(((pos >> 8) as u8) | 0xf0);
                    group.push((len - 18) as u8);
                } else {
                    group.push((pos & 0xff) as u8);
                    group.push(((pos >> 8) as u8) | (((len - 3) as u8) << 4));
                }
                for _ in 0..len {
                    self.push(input[i]);
                    i += 1;
                }
            } else {
                group.push(input[i]);
                self.push(input[i]);
                i += 1;
            }
            items += 1;
            if items == 8 {
                out.extend_from_slice(&group);
                group = vec![0];
                items = 0;
            }
        }
        if items != 0 {
            out.extend_from_slice(&group);
        }
        out
    }
}
