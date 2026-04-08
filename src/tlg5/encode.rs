use std::error::Error;
use std::io::{Cursor, Seek, Write, SeekFrom};
use image::DynamicImage;
use crate::tlg5::slide::SlideCompressor;
use crate::tlg_trait::{PixelLayout, TlgEncoderTrait};

static BLOCK_HEIGHT: usize = 4;
static TLG5_MAGIC: &[u8; 11] = b"TLG5.0\x00raw\x1a"; // 按理说后面还有个\x00，但是实际上这部分根本不会被写入，天知道他们为什么这么写

pub struct Tlg5Encoder
{
    data: Vec<u8>,
    height: u32,
    width: u32,
    pixel: PixelLayout
}

impl TlgEncoderTrait for Tlg5Encoder
{
    fn width(&self) -> u32 {
        self.width
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn pixel_layout(&self) -> PixelLayout {
        self.pixel
    }

    fn encode_to<W: Write + Seek>(&self, inner: &mut W) -> Result<(), Box<dyn Error>> {
        let colors = match self.pixel {
            PixelLayout::Gray => 1u8,
            PixelLayout::Rgb => 3u8,
            PixelLayout::Rgba => 4u8,
        };

        // 写入头部
        inner.write_all(TLG5_MAGIC)?;
        inner.write_all(&[colors])?;
        inner.write_all(&self.width.to_le_bytes())?;
        inner.write_all(&self.height.to_le_bytes())?;
        inner.write_all(&(BLOCK_HEIGHT as u32).to_le_bytes())?;

        let width = self.width as usize;
        let height = self.height as usize;
        let block_count = (height + BLOCK_HEIGHT - 1) / BLOCK_HEIGHT;

        // 占位写入块大小表
        let block_size_pos = inner.stream_position()?;
        for _ in 0..block_count {
            inner.write_all(&0u32.to_le_bytes())?;
        }

        let mut block_sizes = vec![0u32; block_count];
        let mut cmpbuf: Vec<Vec<u8>> = (0..colors)
            .map(|_| vec![0u8; width * BLOCK_HEIGHT])
            .collect();
        let stride = width * colors as usize;

        // 创建一个压缩器实例（复用）
        let mut compressor = SlideCompressor::new();

        for block in 0..block_count {
            let blk_y = block * BLOCK_HEIGHT;
            let ylim = (blk_y + BLOCK_HEIGHT).min(height);
            let mut inp = 0;

            // ---- 填充当前块的 cmpbuf（与之前相同）----
            for y in blk_y..ylim {
                let row = &self.data[y * stride..(y + 1) * stride];
                let upper = if y > 0 {
                    Some(&self.data[(y - 1) * stride..y * stride])
                } else {
                    None
                };
                let mut prevcl = [0i32; 4];

                for x in 0..width {
                    let mut val = [0i32; 4];
                    for c in 0..colors as usize {
                        let cur = row[x * colors as usize + c] as i32;
                        let cl = if let Some(up) = upper {
                            cur - up[x * colors as usize + c] as i32
                        } else {
                            cur
                        };
                        val[c] = cl - prevcl[c];
                        prevcl[c] = cl;
                    }
                    // 原编码器使用的是BGR颜色空间，所以此处修改为RGB
                    match self.pixel {
                        PixelLayout::Gray => cmpbuf[0][inp] = val[0] as i8 as u8,
                        PixelLayout::Rgb => {
                            cmpbuf[0][inp] = (val[2] - val[1]) as i8 as u8; // B - G
                            cmpbuf[1][inp] = val[1] as i8 as u8;            // G
                            cmpbuf[2][inp] = (val[0] - val[1]) as i8 as u8; // R - G
                        }
                        PixelLayout::Rgba => {
                            cmpbuf[0][inp] = (val[2] - val[1]) as i8 as u8; // B - G
                            cmpbuf[1][inp] = val[1] as i8 as u8;            // G
                            cmpbuf[2][inp] = (val[0] - val[1]) as i8 as u8; // R - G
                            cmpbuf[3][inp] = val[3] as i8 as u8;            // A
                        }
                    }
                    inp += 1;
                }
            }

            // ---- 压缩并写入当前块的每个通道 ----
            let mut block_size = 0u32;
            for c in 0..colors as usize {
                let raw_data = &cmpbuf[c][..inp];

                // 保存压缩器当前状态（模仿 C++ 的 Store）
                compressor.store();

                let compressed = compressor.encode(raw_data);
                if compressed.len() < raw_data.len() {
                    // 压缩有效，保留压缩后的状态（不 Restore）
                    inner.write_all(&[0])?;
                    inner.write_all(&(compressed.len() as u32).to_le_bytes())?;
                    inner.write_all(&compressed)?;
                    block_size += 1 + 4 + compressed.len() as u32;
                } else {
                    // 压缩无效，恢复到压缩前的状态（Restore）
                    compressor.restore();
                    inner.write_all(&[1])?;
                    inner.write_all(&(raw_data.len() as u32).to_le_bytes())?;
                    inner.write_all(raw_data)?;
                    block_size += 1 + 4 + raw_data.len() as u32;
                }
            }
            block_sizes[block] = block_size;
        }

        // 回填块大小表
        let current_pos = inner.stream_position()?;
        inner.seek(SeekFrom::Start(block_size_pos))?;
        for size in block_sizes {
            inner.write_all(&size.to_le_bytes())?;
        }
        inner.seek(SeekFrom::Start(current_pos))?;

        Ok(())
    }

    // encode 方法通过 Cursor 调用 encode_to
    fn encode(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut cursor = Cursor::new(Vec::new());
        self.encode_to(&mut cursor)?;
        Ok(cursor.into_inner())
    }

    fn from_image(image: &DynamicImage) -> Result<Tlg5Encoder, Box<dyn Error>>
    {
        match image {
            DynamicImage::ImageLuma8(image) => {
                let width = image.width();
                let height = image.height();
                Ok(Tlg5Encoder::from_gray(image.to_vec(), width, height))
            }
            DynamicImage::ImageRgb8(image) => {
                let width = image.width();
                let height = image.height();
                Ok(Tlg5Encoder::from_rgb(image.to_vec(), width, height))
            }
            DynamicImage::ImageRgba8(image) => {
                let width = image.width();
                let height = image.height();
                Ok(Tlg5Encoder::from_rgba(image.to_vec(), width, height))
            }
            _ => {
                Err(Box::from("Unimplemented"))
            },
        }
    }

    fn from_gray(data: Vec<u8>, width: u32, height: u32) -> Tlg5Encoder
    {
        Tlg5Encoder { data,width,height,pixel: PixelLayout::Gray }
    }

    fn from_rgb(data: Vec<u8>, width: u32, height: u32) -> Tlg5Encoder
    {
        Tlg5Encoder { data,width,height,pixel: PixelLayout::Rgb }
    }

    fn from_rgba(data: Vec<u8>, width: u32, height: u32) -> Tlg5Encoder
    {
        Tlg5Encoder { data,width,height,pixel: PixelLayout::Rgba }
    }

    fn from_raw(data: Vec<u8>, pixel_layout: PixelLayout, width: u32, height: u32) -> Self
    {
        match pixel_layout {
            PixelLayout::Gray => Tlg5Encoder::from_gray(data, width ,height),
            PixelLayout::Rgb => Tlg5Encoder::from_rgb(data, width, height),
            PixelLayout::Rgba => Tlg5Encoder::from_rgba(data, width, height),
        }
    }
}
