use std::error::Error;
use std::io::{Cursor, Seek, SeekFrom, Write};
use image::DynamicImage;
use crate::tlg_trait::{PixelLayout, TlgEncoderTrait};
use crate::slide::SlideEncoder;

use super::bitstream::TLG6BitStream;
use super::golomb::compress_values_golomb;
use super::filter::{apply_color_filter, detect_color_filter};
use super::predict::{pixel_channel, med_predict};

const W_BLOCK_SIZE: usize = 8;
const H_BLOCK_SIZE: usize = 8;
const TLG6_MAGIC: &[u8; 11] = b"TLG6.0\x00raw\x1a";

// ---------------------------------------------------------------------------
// TLG6 Encoder
// ---------------------------------------------------------------------------

pub struct Tlg6Encoder {
    data: Vec<u8>,
    width: u32,
    height: u32,
    pixel: PixelLayout,
}

impl TlgEncoderTrait for Tlg6Encoder {
    fn width(&self) -> u32 { self.width }
    fn height(&self) -> u32 { self.height }
    fn pixel_layout(&self) -> PixelLayout { self.pixel }

    fn encode_to<W: Write + Seek>(&self, inner: &mut W) -> Result<(), Box<dyn Error>> {
        let colors = match self.pixel {
            PixelLayout::Gray => 1usize,
            PixelLayout::Rgb => 3usize,
            PixelLayout::Rgba => 4usize,
        };

        let width = self.width as usize;
        let height = self.height as usize;
        let w_block_count = (width + W_BLOCK_SIZE - 1) / W_BLOCK_SIZE;
        let h_block_count = (height + H_BLOCK_SIZE - 1) / H_BLOCK_SIZE;

        // ---- header ----
        inner.write_all(TLG6_MAGIC)?;
        inner.write_all(&[colors as u8])?;
        inner.write_all(&[0u8; 3])?; // data_flag, color_type, external_golomb_table
        inner.write_all(&(self.width as u32).to_le_bytes())?;
        inner.write_all(&(self.height as u32).to_le_bytes())?;

        // ---- write placeholder for max_bit_length ----
        let max_bit_pos = inner.stream_position()?;
        inner.write_all(&0u32.to_le_bytes())?;

        // ---- compressed data buffer (memory stream) ----
        let mut bs = TLG6BitStream::new();
        let mut memstream = Vec::new();

        // per-channel temporary buffers
        // buf[c]: holds prediction residuals + reordered data
        //   [0..63]     raw prediction (pre-reorder) for MED
        //   [64..127]   reordered MED residuals
        //   [128..191]  reordered AVG residuals
        let block_pixel_count = W_BLOCK_SIZE * H_BLOCK_SIZE * 3;
        let mut buf: Vec<Vec<u8>> =
            (0..colors).map(|_| vec![0u8; block_pixel_count]).collect();
        // block_buf[c]: accumulated residuals for one row of 8-height blocks
        let mut block_buf: Vec<Vec<i8>> =
            (0..colors).map(|_| vec![0i8; H_BLOCK_SIZE * width]).collect();

        // filter types for each block
        let mut filtertypes = Vec::with_capacity(w_block_count * h_block_count);
        let mut max_bit_length = 0u32;

        // ---- main encoding loop over rows of 8x8 blocks ----
        for y in (0..height).step_by(H_BLOCK_SIZE) {
            let ylim = (y + H_BLOCK_SIZE).min(height);
            let bheight = ylim - y;

            let mut gwp = 0usize;
            let mut xp = 0usize;

            for x in (0..width).step_by(W_BLOCK_SIZE) {
                let xlim = (x + W_BLOCK_SIZE).min(width);
                let bw = xlim - x;
                let wp_pixels = bw * bheight;

                // ---- try both MED (p=0) and AVG (p=1) ----
                for p in 0..2 {
                    let dbofs = (p + 1) * W_BLOCK_SIZE * H_BLOCK_SIZE;

                    // compute prediction residuals for each channel
                    for c in 0..colors {
                        let mut wpo = 0usize;
                        for yy in y..ylim {
                            for xx in x..xlim {
                                let pa = if xx > 0 {
                                    pixel_channel(&self.data, xx - 1, yy, width, colors, c)
                                } else {
                                    0u8
                                };
                                let pb = if yy > 0 {
                                    pixel_channel(&self.data, xx, yy - 1, width, colors, c)
                                } else {
                                    0u8
                                };
                                let px = pixel_channel(&self.data, xx, yy, width, colors, c);

                                let py = if p == 0 {
                                    let pc = if xx > 0 && yy > 0 {
                                        pixel_channel(&self.data, xx - 1, yy - 1, width, colors, c)
                                    } else {
                                        0u8
                                    };
                                    med_predict(pa, pb, pc)
                                } else {
                                    ((pa as u16 + pb as u16 + 1) >> 1) as u8
                                };

                                buf[c][wpo] = px.wrapping_sub(py);
                                wpo += 1;
                            }
                        }
                    }

                    // serpentine reordering
                    let mut wpo = 0usize;
                    for yy in y..ylim {
                        let ofs = if (xp & 1) == 0 {
                            (yy - y) * bw
                        } else {
                            (ylim - yy - 1) * bw
                        };

                        let dir = if (bheight & 1) == 0 {
                            ((yy & 1) ^ (xp & 1)) != 0
                        } else if (xp & 1) != 0 {
                            (yy & 1) != 0
                        } else {
                            ((yy & 1) ^ (xp & 1)) != 0
                        };

                        if !dir {
                            for xx in 0..bw {
                                for c in 0..colors {
                                    buf[c][wpo + dbofs] = buf[c][ofs + xx];
                                }
                                wpo += 1;
                            }
                        } else {
                            for xx in (0..bw).rev() {
                                for c in 0..colors {
                                    buf[c][wpo + dbofs] = buf[c][ofs + xx];
                                }
                                wpo += 1;
                            }
                        }
                    }
                }

                // ---- detect best color filter for MED/AVG ----
                let (ft, minp) = if colors >= 3 {
                    let (ft0, p0size) = detect_color_filter(
                        &buf[0][W_BLOCK_SIZE * H_BLOCK_SIZE..][..wp_pixels],
                        &buf[1][W_BLOCK_SIZE * H_BLOCK_SIZE..][..wp_pixels],
                        &buf[2][W_BLOCK_SIZE * H_BLOCK_SIZE..][..wp_pixels],
                        wp_pixels,
                    );

                    let (ft1, p1size) = detect_color_filter(
                        &buf[0][2 * W_BLOCK_SIZE * H_BLOCK_SIZE..][..wp_pixels],
                        &buf[1][2 * W_BLOCK_SIZE * H_BLOCK_SIZE..][..wp_pixels],
                        &buf[2][2 * W_BLOCK_SIZE * H_BLOCK_SIZE..][..wp_pixels],
                        wp_pixels,
                    );

                    if p0size >= p1size { (ft1, 1) } else { (ft0, 0) }
                } else {
                    (0u32, 0u32)
                };

                // ---- apply best prediction and filter ----
                let dbofs = (minp as usize + 1) * W_BLOCK_SIZE * H_BLOCK_SIZE;
                for wp in 0..wp_pixels {
                    for c in 0..colors {
                        block_buf[c][gwp + wp] = buf[c][wp + dbofs] as i8;
                    }
                }

                if colors >= 3 {
                    let (slice0, rest) = block_buf.split_at_mut(1);
                    let (slice1, slice2) = rest.split_at_mut(1);
                    let b = &mut slice0[0][gwp..gwp + wp_pixels];
                    let g = &mut slice1[0][gwp..gwp + wp_pixels];
                    let r = &mut slice2[0][gwp..gwp + wp_pixels];
                    apply_color_filter(b, g, r, wp_pixels, ft);
                }

                filtertypes.push(((ft << 1) | minp) as u8);
                gwp += wp_pixels;
                xp += 1;
            }

            // ---- Golomb-encode this row of blocks for each channel ----
            for c in 0..colors {
                compress_values_golomb(&mut bs, &block_buf[c][..gwp]);

                let bit_length = bs.get_bit_length();
                if bit_length & 0xc0000000 != 0 {
                    return Err("TLG6: bit length overflow".into());
                }
                if max_bit_length < bit_length {
                    max_bit_length = bit_length;
                }

                let channel_data = bs.take_data();
                memstream.extend_from_slice(&bit_length.to_le_bytes());
                memstream.extend_from_slice(&channel_data);
            }
        }

        // ---- write max_bit_length ----
        let current_pos = inner.stream_position()?;
        inner.seek(SeekFrom::Start(max_bit_pos))?;
        inner.write_all(&max_bit_length.to_le_bytes())?;
        inner.seek(SeekFrom::Start(current_pos))?;

        // ---- write filter types (compressed with Slide/LZSS) ----
        let mut slide = SlideEncoder::new();
        // pre-initialize LZSS dictionary with same training data as decoder's LZSS_text
        {
            let mut train = vec![0u8; 4096];
            let mut p = 0usize;
            for i in 0u8..32 {
                for j in 0u8..16 {
                    train[p..p + 4].fill(i);
                    p += 4;
                    train[p..p + 4].fill(j);
                    p += 4;
                }
            }
            slide.encode(&train);
        }
        let compressed = slide.encode(&filtertypes);
        inner.write_all(&(compressed.len() as u32).to_le_bytes())?;
        inner.write_all(&compressed)?;

        // ---- write the compressed bitstream data ----
        inner.write_all(&memstream)?;

        Ok(())
    }

    fn encode(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut cursor = Cursor::new(Vec::new());
        self.encode_to(&mut cursor)?;
        Ok(cursor.into_inner())
    }

    fn from_image(image: &DynamicImage) -> Result<Tlg6Encoder, Box<dyn Error>> {
        match image {
            DynamicImage::ImageLuma8(img) => {
                Ok(Tlg6Encoder::from_gray(img.to_vec(), img.width(), img.height()))
            }
            DynamicImage::ImageRgb8(img) => {
                Ok(Tlg6Encoder::from_rgb(img.to_vec(), img.width(), img.height()))
            }
            DynamicImage::ImageRgba8(img) => {
                Ok(Tlg6Encoder::from_rgba(img.to_vec(), img.width(), img.height()))
            }
            _ => Err(Box::from("Unimplemented image type")),
        }
    }

    fn from_gray(data: Vec<u8>, width: u32, height: u32) -> Self {
        Tlg6Encoder { data, width, height, pixel: PixelLayout::Gray }
    }

    fn from_rgb(data: Vec<u8>, width: u32, height: u32) -> Self {
        Tlg6Encoder { data, width, height, pixel: PixelLayout::Rgb }
    }

    fn from_rgba(data: Vec<u8>, width: u32, height: u32) -> Self {
        Tlg6Encoder { data, width, height, pixel: PixelLayout::Rgba }
    }

    fn from_raw(data: Vec<u8>, pixel_layout: PixelLayout, width: u32, height: u32) -> Self {
        match pixel_layout {
            PixelLayout::Gray => Tlg6Encoder::from_gray(data, width, height),
            PixelLayout::Rgb => Tlg6Encoder::from_rgb(data, width, height),
            PixelLayout::Rgba => Tlg6Encoder::from_rgba(data, width, height),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_small_rgb() {
        let img = image::DynamicImage::ImageRgb8(
            image::RgbImage::from_fn(16, 16, |x, y| {
                image::Rgb([(x * 17) as u8, (y * 17) as u8, 128u8])
            })
        );
        let encoder = Tlg6Encoder::from_image(&img).unwrap();
        let result = encoder.encode();
        assert!(result.is_ok(), "encoding should succeed");
        let data = result.unwrap();
        assert!(data.len() > 22, "should have at least header bytes");
        assert_eq!(&data[..11], TLG6_MAGIC.as_slice(), "should start with TLG6 magic");
    }

    #[test]
    fn test_encode_small_gray() {
        let img = image::DynamicImage::ImageLuma8(
            image::GrayImage::from_fn(8, 8, |x, y| {
                image::Luma([(x + y) as u8])
            })
        );
        let encoder = Tlg6Encoder::from_image(&img).unwrap();
        let result = encoder.encode();
        assert!(result.is_ok(), "gray encoding should succeed");
    }

    #[test]
    fn test_encode_small_rgba() {
        let img = image::DynamicImage::ImageRgba8(
            image::RgbaImage::from_fn(8, 8, |x, y| {
                image::Rgba([(x * 32) as u8, (y * 32) as u8, 128u8, 255u8])
            })
        );
        let encoder = Tlg6Encoder::from_image(&img).unwrap();
        let result = encoder.encode();
        assert!(result.is_ok(), "rgba encoding should succeed");
    }
}
