use super::golomb::TryCompress;

pub(super) const FILTER_TRY_COUNT: usize = 16;

// ---------------------------------------------------------------------------
// Color correlation filters (16 filters, codes 0-15)
// ---------------------------------------------------------------------------

pub(super) fn apply_color_filter(
    b: &mut [i8],
    g: &mut [i8],
    r: &mut [i8],
    len: usize,
    code: u32,
) {
    match code {
        0 => {} // identity
        1 => {
            for i in 0..len {
                r[i] = r[i].wrapping_sub(g[i]);
                b[i] = b[i].wrapping_sub(g[i]);
            }
        }
        2 => {
            for i in 0..len {
                r[i] = r[i].wrapping_sub(g[i]);
                g[i] = g[i].wrapping_sub(b[i]);
            }
        }
        3 => {
            for i in 0..len {
                b[i] = b[i].wrapping_sub(g[i]);
                g[i] = g[i].wrapping_sub(r[i]);
            }
        }
        4 => {
            for i in 0..len {
                r[i] = r[i].wrapping_sub(g[i]);
                g[i] = g[i].wrapping_sub(b[i]);
                b[i] = b[i].wrapping_sub(r[i]);
            }
        }
        5 => {
            for i in 0..len {
                g[i] = g[i].wrapping_sub(b[i]);
                b[i] = b[i].wrapping_sub(r[i]);
            }
        }
        6 => {
            for i in 0..len {
                b[i] = b[i].wrapping_sub(g[i]);
            }
        }
        7 => {
            for i in 0..len {
                g[i] = g[i].wrapping_sub(b[i]);
            }
        }
        8 => {
            for i in 0..len {
                r[i] = r[i].wrapping_sub(g[i]);
            }
        }
        9 => {
            for i in 0..len {
                b[i] = b[i].wrapping_sub(g[i]);
                g[i] = g[i].wrapping_sub(r[i]);
                r[i] = r[i].wrapping_sub(b[i]);
            }
        }
        10 => {
            for i in 0..len {
                g[i] = g[i].wrapping_sub(r[i]);
                b[i] = b[i].wrapping_sub(r[i]);
            }
        }
        11 => {
            for i in 0..len {
                r[i] = r[i].wrapping_sub(b[i]);
                g[i] = g[i].wrapping_sub(b[i]);
            }
        }
        12 => {
            for i in 0..len {
                g[i] = g[i].wrapping_sub(r[i]);
                r[i] = r[i].wrapping_sub(b[i]);
            }
        }
        13 => {
            for i in 0..len {
                g[i] = g[i].wrapping_sub(r[i]);
                r[i] = r[i].wrapping_sub(b[i]);
                b[i] = b[i].wrapping_sub(g[i]);
            }
        }
        14 => {
            for i in 0..len {
                r[i] = r[i].wrapping_sub(b[i]);
                b[i] = b[i].wrapping_sub(g[i]);
                g[i] = g[i].wrapping_sub(r[i]);
            }
        }
        15 => {
            for i in 0..len {
                let t = (b[i] as i16 * 2) as i8;
                r[i] = r[i].wrapping_sub(t);
                g[i] = g[i].wrapping_sub(t);
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Detect best color filter for a block
// ---------------------------------------------------------------------------

pub(super) fn detect_color_filter(
    b: &[u8],
    g: &[u8],
    r: &[u8],
    size: usize,
) -> (u32, u32) {
    let mut minbits = u32::MAX;
    let mut mincode = 0u32;

    let mut bbuf = vec![0i8; size];
    let mut gbuf = vec![0i8; size];
    let mut rbuf = vec![0i8; size];

    for code in 0..FILTER_TRY_COUNT {
        for i in 0..size {
            bbuf[i] = b[i] as i8;
            gbuf[i] = g[i] as i8;
            rbuf[i] = r[i] as i8;
        }

        apply_color_filter(&mut bbuf, &mut gbuf, &mut rbuf, size, code as u32);

        let mut comp = TryCompress::new();

        let mut bits = 0u32;

        comp.reset();
        comp.try_compress(&bbuf);
        bits += comp.flush();
        if minbits < bits {
            continue;
        }

        comp.reset();
        comp.try_compress(&gbuf);
        bits += comp.flush();
        if minbits < bits {
            continue;
        }

        comp.reset();
        comp.try_compress(&rbuf);
        bits += comp.flush();

        if bits < minbits {
            minbits = bits;
            mincode = code as u32;
        }
    }

    (mincode, minbits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_filter_identity() {
        let mut b = vec![10i8, 20, 30];
        let mut g = vec![5i8, 15, 25];
        let mut r = vec![0i8, 1, 2];
        apply_color_filter(&mut b, &mut g, &mut r, 3, 0);
        assert_eq!(b, vec![10, 20, 30]);
        assert_eq!(g, vec![5, 15, 25]);
        assert_eq!(r, vec![0, 1, 2]);
    }
}
