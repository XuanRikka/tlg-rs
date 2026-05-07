use std::error::Error;
use std::io::{Cursor, Read, Seek};

#[cfg(any(test, feature = "image"))]
use image::DynamicImage;

use crate::slide::SlideDecoder;
use crate::tlg6::{TLG6_MAGIC, H_BLOCK_SIZE, W_BLOCK_SIZE};
use crate::tlg_type::{ImageInfo, PixelLayout, TlgDecoderTrait};

use super::bitstream::TLG6BitReader;
use super::golomb::decode_golomb_channel;

// ---------------------------------------------------------------------------
// TLG6 Decoder
// ---------------------------------------------------------------------------

pub struct Tlg6Decoder {
    data: Vec<u8>,
}


impl TlgDecoderTrait for Tlg6Decoder {
    #[cfg(not(target_arch = "wasm32"))]
    fn from_path<P: AsRef<std::path::Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let data = std::fs::read(path)?;
        Self::from_data(data)
    }

    fn from_reader<R: Read + Seek>(mut reader: R) -> Result<Self, Box<dyn Error>> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        Self::from_data(data)
    }

    fn from_data(data: Vec<u8>) -> Result<Self, Box<dyn Error>> {
        if data.len() < 27 {
            return Err("Data too short for TLG6".into());
        }
        Ok(Tlg6Decoder { data })
    }

    fn decode(self) -> Result<(Vec<u8>, ImageInfo), Box<dyn Error>> {
        let mut cur = Cursor::new(&self.data);

        // ---- read header (magic already consumed by check in from_data) ----
        let mut magic = [0u8; 11];
        cur.read_exact(&mut magic)?;
        if &magic != TLG6_MAGIC {
            return Err("Not a TLG6 file".into());
        }

        let mut colors_byte = [0u8; 1];
        cur.read_exact(&mut colors_byte)?;
        let colors = colors_byte[0] as usize;

        let mut flags = [0u8; 3];
        cur.read_exact(&mut flags)?;

        let mut wb = [0u8; 4];

        cur.read_exact(&mut wb)?;
        let width = u32::from_le_bytes(wb) as usize;

        cur.read_exact(&mut wb)?;
        let height = u32::from_le_bytes(wb) as usize;

        cur.read_exact(&mut wb)?;
        let _max_bit_length = u32::from_le_bytes(wb);

        // ---- block counts ----
        let x_block_count = ((width + W_BLOCK_SIZE - 1) / W_BLOCK_SIZE).max(1);
        let _y_block_count = ((height + H_BLOCK_SIZE - 1) / H_BLOCK_SIZE).max(1);
        let main_count = width / W_BLOCK_SIZE;
        let fraction = width - main_count * W_BLOCK_SIZE;

        // ---- read filter types (LZSS compressed) ----
        let mut size_buf = [0u8; 4];
        cur.read_exact(&mut size_buf)?;
        let filter_size = u32::from_le_bytes(size_buf) as usize;

        let mut filter_data = vec![0u8; filter_size];
        cur.read_exact(&mut filter_data)?;

        // Build LZSS training text (same as encoder pre-init + decoder LZSS_text)
        let mut lzss_text = vec![0u8; 4096];
        {
            let mut p = 0usize;
            for i in 0u8..32 {
                for j in 0u8..16 {
                    lzss_text[p..p + 4].fill(i);
                    p += 4;
                    lzss_text[p..p + 4].fill(j);
                    p += 4;
                }
            }
        }

        let mut slide_dec = SlideDecoder::new();
        slide_dec.init_with_text(&lzss_text);
        let filter_types = slide_dec.decode(&filter_data);

        // ---- output buffer (always 4 bytes per pixel, even for grayscale) ----
        let out_bpp = 4;
        let out_row_bytes = width * out_bpp;
        let mut out = vec![0u8; height * out_row_bytes];

        // Zero line (virtual y=-1): for colors=3, alpha = 0xff (opaque), otherwise 0
        let zero_line = if colors == 3 {
            let mut zl = vec![0u8; out_row_bytes];
            for i in (3..out_row_bytes).step_by(4) {
                zl[i] = 0xff;
            }
            zl
        } else {
            vec![0u8; out_row_bytes]
        };

        // Read compressed data (remainder of the file)
        let compressed_start = cur.stream_position()? as usize;
        let compressed_data = &self.data[compressed_start..];
        let mut comp_pos = 0usize;

        // ---- main decode loop over rows of blocks ----
        let mut prevline_start: Option<usize> = None;

        for y in (0..height).step_by(H_BLOCK_SIZE) {
            let ylim = (y + H_BLOCK_SIZE).min(height);
            let bheight = ylim - y;
            let pixel_count = bheight * width;

            // Pixel buffer: each pixel as [B, G, R, A] signed bytes (4 bytes per pixel)
            let mut pixelbuf = vec![0i8; pixel_count * 4];

            // Decode each channel
            for c in 0..colors {
                if comp_pos + 4 > compressed_data.len() {
                    return Err("Unexpected end of compressed data".into());
                }
                let bit_length = u32::from_le_bytes(
                    compressed_data[comp_pos..comp_pos + 4].try_into()?,
                );
                comp_pos += 4;

                let method = (bit_length >> 30) & 3;
                let bit_length = bit_length & 0x3fffffff;
                let byte_length =
                    (bit_length as usize / 8) + if bit_length % 8 != 0 { 1 } else { 0 };

                if comp_pos + byte_length > compressed_data.len() {
                    return Err("Unexpected end of compressed data".into());
                }
                let bit_pool = &compressed_data[comp_pos..comp_pos + byte_length];
                comp_pos += byte_length;

                if method != 0 {
                    return Err("Unsupported entropy coding method".into());
                }

                let mut padded_bit_pool = Vec::with_capacity(byte_length + 5);
                padded_bit_pool.extend_from_slice(bit_pool);
                padded_bit_pool.resize(byte_length + 5, 0);

                let mut br = TLG6BitReader::new(&padded_bit_pool);
                decode_golomb_channel(
                    &mut br,
                    &mut pixelbuf,
                    pixel_count,
                    4,
                    c,
                    c == 0 && colors != 1,
                );
            }

            // Decode each line in this block row
            let ft_row = &filter_types[(y / H_BLOCK_SIZE) * x_block_count..];
            let skipblockbytes = bheight * W_BLOCK_SIZE;

            for yy in y..ylim {
                let curline_start = yy * out_row_bytes;
                let dir = (yy & 1) ^ 1; // 1=forward, 0=backward

                let (prevline, curline) = if let Some(pstart) = prevline_start {
                    let cstart = curline_start;
                    if pstart < cstart {
                        let (a, b) = out.split_at_mut(cstart);
                        (&a[pstart..pstart + out_row_bytes], &mut b[..out_row_bytes])
                    } else {
                        let (a, b) = out.split_at_mut(pstart);
                        (&b[..out_row_bytes], &mut a[cstart..cstart + out_row_bytes])
                    }
                } else {
                    (&zero_line[..], &mut out[curline_start..curline_start + out_row_bytes])
                };

                // Decode main blocks
                if main_count > 0 {
                    let w_first = if width < W_BLOCK_SIZE { width } else { W_BLOCK_SIZE };
                    let start = w_first * (yy - y);
                    decode_line(
                        prevline,
                        curline,
                        width,
                        0,
                        main_count,
                        ft_row,
                        skipblockbytes,
                        &pixelbuf[start * 4..],
                        if colors == 3 { 0xff000000u32 } else { 0 },
                        (ylim - yy - 1) as isize - (yy - y) as isize,
                        dir,
                        out_bpp,
                    );
                }

                // Decode fraction block
                if main_count != x_block_count {
                    let ww = if fraction > W_BLOCK_SIZE { W_BLOCK_SIZE } else { fraction };
                    let start = ww * (yy - y);
                    decode_line(
                        prevline,
                        curline,
                        width,
                        main_count,
                        x_block_count,
                        ft_row,
                        skipblockbytes,
                        &pixelbuf[start * 4..],
                        if colors == 3 { 0xff000000u32 } else { 0 },
                        (ylim - yy - 1) as isize - (yy - y) as isize,
                        dir,
                        out_bpp,
                    );
                }

                prevline_start = Some(curline_start);
            }
        }

        let pixel_layout = match colors {
            1 => PixelLayout::Gray,
            3 => PixelLayout::Rgb,
            4 => PixelLayout::Rgba,
            _ => return Err("Unsupported color count".into()),
        };

        let info = ImageInfo {
            width: width as u32,
            height: height as u32,
            pixel_layout
        };

        match colors {
            1 => {
                // BGRA → Luma (从 B 通道提取亮度)
                let luma: Vec<u8> = out.chunks_exact(4)
                    .map(|chunk| chunk[0])
                    .collect();
                Ok((luma, info))
            }
            3 => {
                // BGRA → RGB
                let mut rgb = vec![0u8; width * height * 3];
                for (i, chunk) in out.chunks_exact(4).enumerate() {
                    let idx = i * 3;
                    rgb[idx]     = chunk[2]; // R ← B
                    rgb[idx + 1] = chunk[1]; // G ← G
                    rgb[idx + 2] = chunk[0]; // B ← R
                }
                Ok((rgb, info))
            }
            4 => {
                // BGRA → RGBA
                let mut rgba = vec![0u8; width * height * 4];
                for (i, chunk) in out.chunks_exact(4).enumerate() {
                    let idx = i * 4;
                    rgba[idx]     = chunk[2]; // R ← B
                    rgba[idx + 1] = chunk[1]; // G ← G
                    rgba[idx + 2] = chunk[0]; // B ← R
                    rgba[idx + 3] = chunk[3]; // A ← A
                }
                Ok((rgba, info))
            }
            _ => Err("Unsupported color count".into()),
        }
    }

    #[cfg(any(test, feature = "image"))]
    fn decode_to_image(self) -> Result<DynamicImage, Box<dyn Error>> {
        let (data, info) = self.decode()?;

        match info.pixel_layout
        {
            PixelLayout::Gray => {
                Ok(DynamicImage::ImageLuma8(
                    image::GrayImage::from_raw(info.width, info.height, data)
                        .ok_or("Failed to create gray image")?,
                ))
            },
            PixelLayout::Rgb => {
                Ok(DynamicImage::ImageRgb8(
                    image::RgbImage::from_raw(info.width, info.height, data)
                        .ok_or("Failed to create rgb image")?,
                ))
            },
            PixelLayout::Rgba => {
                Ok(DynamicImage::ImageRgba8(
                    image::RgbaImage::from_raw(info.width, info.height, data)
                        .ok_or("Failed to create rgba image")?,
                ))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Per-byte MED / AVG prediction on packed u32 (operates on 4 bytes in parallel)
// ---------------------------------------------------------------------------

/// Byte-parallel "greater than" mask.
/// Returns 0x80 in each byte where a >= b, 0x00 otherwise.
#[inline]
fn make_gt_mask(a: u32, b: u32) -> u32 {
    let tmp2 = !b;
    let tmp = ((a & tmp2).wrapping_add(((a ^ tmp2) >> 1) & 0x7f7f7f7f)) & 0x80808080;
    ((tmp >> 7) + 0x7f7f7f7f) ^ 0x7f7f7f7f
}

/// Byte-parallel packed byte addition (a + b for each byte, no overflow between bytes).
#[inline]
fn packed_bytes_add(a: u32, b: u32) -> u32 {
    let tmp = (((a & b) << 1) + ((a ^ b) & 0xfefefefe)) & 0x01010100;
    a.wrapping_add(b) - tmp
}

/// Byte-parallel MED (Median Edge Detector) on packed u32.
/// For each byte: min(a,b) if c >= max(a,b), max(a,b) if c < min(a,b), a+b-c otherwise.
#[inline]
fn med2(a: u32, b: u32, c: u32) -> u32 {
    let aa_gt_bb = make_gt_mask(a, b);
    let a_xor_b_and_aa_gt_bb = (a ^ b) & aa_gt_bb;
    let aa = a_xor_b_and_aa_gt_bb ^ a;
    let bb = a_xor_b_and_aa_gt_bb ^ b;
    let n = make_gt_mask(c, bb);
    let nn = make_gt_mask(aa, c);
    let m = !(n | nn);
    (n & aa) | (nn & bb) | ((bb & m).wrapping_sub(c & m).wrapping_add(aa & m))
}

/// MED prediction: predict = med(p, u, up), then add residual.
#[inline]
fn med_predict_add(p: u32, u: u32, up: u32, residual: u32) -> u32 {
    packed_bytes_add(med2(p, u, up), residual)
}

/// AVG prediction: predict = (p + u + 1) >> 1, then add residual.
#[inline]
fn avg_predict_add(p: u32, u: u32, residual: u32) -> u32 {
    // (p & u) + ((p ^ u) >> 1)  gives floor average; (+1 correction for 0.5 rounding)
    // TLG6_AVG_PACKED: ((x&y) + (((x^y) & 0xfefefefe) >> 1)) + ((x^y) & 0x01010101)
    let avg = (p & u)
        + (((p ^ u) & 0xfefefefe) >> 1)
        + ((p ^ u) & 0x01010101);
    packed_bytes_add(avg, residual)
}
// ---------------------------------------------------------------------------
// Inverse color filter — undoes the encoder's color filter for a single pixel
// ---------------------------------------------------------------------------

/// Apply inverse color filter for a single pixel.
/// ft: filter type code (0..15)
/// (ib, ig, ir, ia): signed residuals from the bitstream
/// Returns (b, g, r, a) — the actual residual values after undoing the filter.
#[inline]
fn inverse_color_filter(ft: u8, ib: i8, ig: i8, ir: i8, ia: i8) -> (i32, i32, i32, i32) {
    let ib = ib as i32;
    let ig = ig as i32;
    let ir = ir as i32;
    let ia = ia as i32;

    // These expressions mirror the C++ DO_CHROMA_DECODE cases (inverse of encoder filter)
    match ft {
        0 => (ib, ig, ir, ia),
        1 => (ib + ig, ig, ir + ig, ia),
        2 => (ib, ig + ib, ir + ib + ig, ia),
        3 => (ib + ir + ig, ig + ir, ir, ia),
        4 => (ib + ir, ig + ib + ir, ir + ib + ir + ig, ia),
        5 => (ib + ir, ig + ib + ir, ir, ia),
        6 => (ib + ig, ig, ir, ia),
        7 => (ib, ig + ib, ir, ia),
        8 => (ib, ig, ir + ig, ia),
        9 => (ib + ig + ir + ib, ig + ir + ib, ir + ib, ia),
        10 => (ib + ir, ig + ir, ir, ia),
        11 => (ib, ig + ib, ir + ib, ia),
        12 => (ib, ig + ir + ib, ir + ib, ia),
        13 => (ib + ig, ig + ir + ib + ig, ir + ib + ig, ia),
        14 => (ib + ig + ir, ig + ir, ir + ib + ig + ir, ia),
        15 => (ib, ig + (ib << 1), ir + (ib << 1), ia),
        _ => (ib, ig, ir, ia),
    }
}

// ---------------------------------------------------------------------------
// Line decode — processes one line of blocks
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn decode_line(
    prevline: &[u8],
    curline: &mut [u8],
    width: usize,
    start_block: usize,
    block_limit: usize,
    filter_types: &[u8],
    skipblockbytes: usize,
    pixelbuf: &[i8],
    initialp: u32,
    oddskip: isize,
    dir: usize,
    out_bpp: usize,
) {
    let step: isize = if (dir & 1) != 0 { 1 } else { -1 };

    // p: previous output pixel (on current line)
    // up: pixel above-left (prevline at col-1)
    let mut p: u32;
    let mut up: u32;

    if start_block > 0 {
        let px = start_block * W_BLOCK_SIZE;
        p = read_u32_le(curline, (px.wrapping_sub(1)) * out_bpp);
        up = read_u32_le(prevline, (px.wrapping_sub(1)) * out_bpp);
    } else {
        p = initialp;
        up = initialp;
    }

    let mut in_off: isize = (skipblockbytes * start_block) as isize;

    for i in start_block..block_limit {
        let mut w = width as isize - (i * W_BLOCK_SIZE) as isize;
        if w > W_BLOCK_SIZE as isize {
            w = W_BLOCK_SIZE as isize;
        }
        let ww = w as usize;

        if step == -1 {
            in_off += w - 1;
        }
        if (i & 1) != 0 {
            in_off += oddskip * w;
        }

        let ft = filter_types[i];
        let ft_code = ft >> 1;
        let use_med = (ft & 1) == 0;

        let mut j = w;
        while j > 0 {
            let idx = in_off as usize * 4;
            let ib = pixelbuf[idx];
            let ig = pixelbuf[idx + 1];
            let ir = pixelbuf[idx + 2];
            let ia = pixelbuf[idx + 3];

            let (b, g, r, a_res) = inverse_color_filter(ft_code, ib, ig, ir, ia);

            let residual = ((r as u32 & 0xff) << 16)
                | ((g as u32 & 0xff) << 8)
                | (b as u32 & 0xff)
                | ((a_res as u32 & 0xff) << 24);

            // Column of current pixel — always forward (0,1,2,...) regardless of step direction.
            // The serpentine reordering in pixelbuf handles the backward read order.
            let col = i * W_BLOCK_SIZE + (ww - j as usize);
            let u = read_u32_le(prevline, col * out_bpp);

            let pixel = if use_med {
                med_predict_add(p, u, up, residual)
            } else {
                avg_predict_add(p, u, residual)
            };

            write_u32_le(curline, col * out_bpp, pixel);

            up = u;
            p = pixel;

            in_off += step;
            j -= 1;
        }

        // After block: skip to next block's data in pixelbuf
        if step == 1 {
            in_off += (skipblockbytes - ww) as isize;
        } else {
            in_off += (skipblockbytes + 1) as isize;
        }
        if (i & 1) != 0 {
            in_off -= oddskip * ww as isize;
        }
    }
}

#[inline]
fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    if offset + 4 > data.len() {
        0
    } else {
        u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]])
    }
}

#[inline]
fn write_u32_le(data: &mut [u8], offset: usize, value: u32) {
    if offset + 4 <= data.len() {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tlg_type::TlgEncoderTrait;
    use crate::tlg6::Tlg6Encoder;

    fn roundtrip(img: &DynamicImage) {
        let encoder = Tlg6Encoder::from_image(img).unwrap();
        let data = encoder.encode().unwrap();
        let decoder = Tlg6Decoder::from_data(data).unwrap();
        let decoded = decoder.decode_to_image().unwrap();

        assert_eq!(img.width(), decoded.width());
        assert_eq!(img.height(), decoded.height());

        let orig = img.to_rgba8();
        let round = decoded.to_rgba8();
        assert_eq!(orig.as_raw(), round.as_raw(), "roundtrip mismatch");
    }

    #[test]
    fn roundtrip_gray_8x8() {
        let img = DynamicImage::ImageLuma8(image::GrayImage::from_fn(
            8,
            8,
            |x, y| image::Luma([(x + y * 8) as u8]),
        ));
        roundtrip(&img);
    }

    #[test]
    fn roundtrip_rgb_8x8() {
        let img =
            DynamicImage::ImageRgb8(image::RgbImage::from_fn(8, 8, |x, y| {
                image::Rgb([(x * 17) as u8, (y * 17) as u8, 128u8])
            }));
        roundtrip(&img);
    }

    #[test]
    fn roundtrip_rgba_8x8() {
        let img =
            DynamicImage::ImageRgba8(image::RgbaImage::from_fn(8, 8, |x, y| {
                image::Rgba([(x * 32) as u8, (y * 32) as u8, 128u8, 255u8])
            }));
        roundtrip(&img);
    }
}