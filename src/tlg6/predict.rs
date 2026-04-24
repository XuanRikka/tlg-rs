// ---------------------------------------------------------------------------
// Pixel data helpers: accesses pixel channels in BGR order (internal TLG6
// convention) regardless of the image crate's native RGB order.
// ---------------------------------------------------------------------------

#[inline]
pub(super) fn pixel_channel(
    data: &[u8],
    x: usize,
    y: usize,
    width: usize,
    colors: usize,
    ch: usize,
) -> u8 {
    let offset = y * width * colors + x * colors;
    match colors {
        1 => data[offset],
        3 | 4 => match ch {
            0 => data[offset + 2], // B
            1 => data[offset + 1], // G
            2 => data[offset + 0], // R
            _ => data[offset + 3], // A
        },
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// MED (Median Edge Detector) prediction
// ---------------------------------------------------------------------------

#[inline]
pub(super) fn med_predict(pa: u8, pb: u8, pc: u8) -> u8 {
    let min_a_b = if pa > pb { pb } else { pa };
    let max_a_b = if pa < pb { pb } else { pa };
    if pc >= max_a_b {
        min_a_b
    } else if pc < min_a_b {
        max_a_b
    } else {
        pa.wrapping_add(pb).wrapping_sub(pc)
    }
}
