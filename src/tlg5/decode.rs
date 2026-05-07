use std::error::Error;
use std::fs::File;
use std::io::{Cursor, Read, Seek};
use std::path::Path;

use byteorder::{LittleEndian, ReadBytesExt};

#[cfg(any(test, feature = "image"))]
use image::{DynamicImage, GrayImage, RgbImage, RgbaImage};

use super::{BLOCK_HEIGHT, TLG5_MAGIC};
use crate::slide::SlideDecoder;
use crate::tlg_type::{ImageInfo, PixelLayout, TlgDecoderTrait};

pub struct Tlg5Decoder {
    data: Vec<u8>,
}

impl TlgDecoderTrait for Tlg5Decoder {
    #[cfg(not(target_arch = "wasm32"))]
    fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized,
    {
        let mut file = File::open(path)?;
        let mut data = Vec::with_capacity(file.metadata()?.len() as usize);
        std::io::copy(&mut file, &mut data)?;
        Ok(Tlg5Decoder { data })
    }

    fn from_reader<R: Read + Seek>(mut reader: R) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized,
    {
        let mut data = Vec::new();
        std::io::copy(&mut reader, &mut data)?;
        Ok(Tlg5Decoder { data })
    }

    fn from_data(data: Vec<u8>) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized,
    {
        Ok(Tlg5Decoder { data })
    }

    fn decode(self) -> Result<(Vec<u8>, ImageInfo), Box<dyn Error>> {
        let mut cur = Cursor::new(&self.data);

        let mut magic = [0u8; TLG5_MAGIC.len()];
        cur.read_exact(&mut magic)?;
        if &magic != TLG5_MAGIC {
            return Err("tlg5 decode wrong magic".into());
        }

        let colors = cur.read_u8()? as usize;
        let pixel_layout = match colors {
            1 => PixelLayout::Gray,
            3 => PixelLayout::Rgb,
            4 => PixelLayout::Rgba,
            _ => return Err(format!("unsupported TLG5 color count: {colors}").into()),
        };

        let width = cur.read_u32::<LittleEndian>()? as usize;
        let height = cur.read_u32::<LittleEndian>()? as usize;
        let block_height = cur.read_u32::<LittleEndian>()? as usize;

        if block_height != BLOCK_HEIGHT {
            return Err(
                format!("unsupported TLG5 block height: {block_height}, expected {BLOCK_HEIGHT}")
                    .into(),
            );
        }

        let block_count = height.div_ceil(BLOCK_HEIGHT);
        let mut block_sizes = Vec::with_capacity(block_count);
        for _ in 0..block_count {
            block_sizes.push(cur.read_u32::<LittleEndian>()? as usize);
        }

        let mut compressor = SlideDecoder::new();
        let stride = width * colors;
        let mut output = vec![0u8; stride * height];

        for (block_index, block_size) in block_sizes.into_iter().enumerate() {
            let block_start = cur.position() as usize;
            let y_start = block_index * BLOCK_HEIGHT;
            let y_end = (y_start + BLOCK_HEIGHT).min(height);
            let rows_in_block = y_end - y_start;
            let plane_len = width * rows_in_block;

            let mut planes = vec![vec![0u8; plane_len]; colors];
            for plane in &mut planes {
                let compressed_flag = cur.read_u8()?;
                let data_size = cur.read_u32::<LittleEndian>()? as usize;
                let mut data = vec![0u8; data_size];
                cur.read_exact(&mut data)?;

                let decoded = match compressed_flag {
                    0 => compressor.decode(&data),
                    1 => data,
                    _ => {
                        return Err(
                            format!("invalid TLG5 compression flag: {compressed_flag}").into()
                        )
                    }
                };

                if decoded.len() != plane_len {
                    return Err(
                        format!(
                            "plane size mismatch in block {block_index}: expected {plane_len}, got {}",
                            decoded.len()
                        ).into(),
                    );
                }

                plane.copy_from_slice(&decoded);
            }

            let consumed = cur.position() as usize - block_start;
            if consumed != block_size {
                return Err(
                    format!(
                        "block size mismatch in block {block_index}: expected {block_size}, consumed {consumed}"
                    ).into(),
                );
            }

            let mut plane_index = 0;
            for y in y_start..y_end {
                let row_start = y * stride;
                let upper_row_start = y.checked_sub(1).map(|upper_y| upper_y * stride);
                let mut prev_cl = [0u8; 4];

                for x in 0..width {
                    let mut values = [0u8; 4];
                    // 编码器把像素差分写成了 G / (B - G) / (R - G) / A，这里先把每像素的通道差分还原回来。
                    match pixel_layout {
                        PixelLayout::Gray => {
                            values[0] = planes[0][plane_index];
                        }
                        PixelLayout::Rgb => {
                            let g = planes[1][plane_index];
                            values[0] = planes[2][plane_index].wrapping_add(g);
                            values[1] = g;
                            values[2] = planes[0][plane_index].wrapping_add(g);
                        }
                        PixelLayout::Rgba => {
                            let g = planes[1][plane_index];
                            values[0] = planes[2][plane_index].wrapping_add(g);
                            values[1] = g;
                            values[2] = planes[0][plane_index].wrapping_add(g);
                            values[3] = planes[3][plane_index];
                        }
                    }

                    // 每个通道先做横向差分逆变换，再加回上一行对应像素，顺序与编码器中的
                    // `cl = cur - up` 和 `val = cl - prevcl` 完全相反。这里必须使用 wrapping_add，
                    // 因为编码器把差分按 8-bit 模 256 写入，溢出要按字节环绕而不是报错。
                    for c in 0..colors {
                        let cl = prev_cl[c].wrapping_add(values[c]);
                        prev_cl[c] = cl;

                        let upper = upper_row_start
                            .map(|upper| output[upper + x * colors + c])
                            .unwrap_or(0);
                        output[row_start + x * colors + c] = upper.wrapping_add(cl);
                    }

                    plane_index += 1;
                }
            }
        }

        let info = ImageInfo {
            width: width as u32,
            height: height as u32,
            pixel_layout
        };

        Ok((output, info))
    }

    #[cfg(any(test, feature = "image"))]
    fn decode_to_image(self) -> Result<DynamicImage, Box<dyn Error>> {
        let (data, info) = self.decode()?;

        match info.pixel_layout
        {
            PixelLayout::Gray => {
                Ok(
                    DynamicImage::ImageLuma8(
                        GrayImage::from_raw(info.width, info.height, data).
                            ok_or("failed to build gray image")?
                    )
                )
            },
            PixelLayout::Rgb => {
                Ok(
                    DynamicImage::ImageRgb8(
                        RgbImage::from_raw(info.width, info.height, data).
                            ok_or("failed to build rgb image")?
                    )
                )
            },
            PixelLayout::Rgba => {
                Ok(
                    DynamicImage::ImageRgba8(
                        RgbaImage::from_raw(info.width, info.height, data).
                            ok_or("failed to build rgba image")?
                    )
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use image::{DynamicImage, GrayImage, RgbImage, RgbaImage};

    use super::Tlg5Decoder;
    use crate::tlg5::encode::Tlg5Encoder;
    use crate::tlg_type::{TlgDecoderTrait, TlgEncoderTrait};

    #[test]
    fn roundtrip_gray() {
        let width = 7;
        let height = 5;
        let data = (0..width * height)
            .map(|i| (i as u8).wrapping_mul(17))
            .collect::<Vec<_>>();
        let image = DynamicImage::ImageLuma8(
            GrayImage::from_raw(width, height, data.clone()).expect("gray image"),
        );

        let encoded = Tlg5Encoder::from_image(&image).unwrap().encode().unwrap();
        let decoded = Tlg5Decoder::from_data(encoded).unwrap().decode_to_image().unwrap();

        assert_eq!(decoded.into_bytes(), data);
    }

    #[test]
    fn roundtrip_rgb() {
        let width = 9;
        let height = 6;
        let mut data = Vec::with_capacity((width * height * 3) as usize);
        for y in 0..height {
            for x in 0..width {
                data.push((x * 19 + y * 7) as u8);
                data.push((x * 5 + y * 23) as u8);
                data.push((x * 29 + y * 11) as u8);
            }
        }

        let image =
            DynamicImage::ImageRgb8(RgbImage::from_raw(width, height, data.clone()).expect("rgb"));

        let encoded = Tlg5Encoder::from_image(&image).unwrap().encode().unwrap();
        let decoded = Tlg5Decoder::from_data(encoded).unwrap().decode_to_image().unwrap();

        assert_eq!(decoded.into_bytes(), data);
    }

    #[test]
    fn roundtrip_rgba() {
        let width = 8;
        let height = 7;
        let mut data = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                data.push((x * 31 + y * 3) as u8);
                data.push((x * 13 + y * 17) as u8);
                data.push((x * 7 + y * 27) as u8);
                data.push(255u16.saturating_sub((x * 9 + y * 5) as u16) as u8);
            }
        }

        let image = DynamicImage::ImageRgba8(
            RgbaImage::from_raw(width, height, data.clone()).expect("rgba"),
        );

        let encoded = Tlg5Encoder::from_image(&image).unwrap().encode().unwrap();
        let decoded = Tlg5Decoder::from_data(encoded).unwrap().decode_to_image().unwrap();

        assert_eq!(decoded.into_bytes(), data);
    }
}
