use std::error::Error;
use std::io::{Cursor, Seek, SeekFrom, Write};
use std::collections::HashMap;
use byteorder::{LittleEndian, WriteBytesExt};
use image::DynamicImage;
use crate::SDS_MAGIC;
use crate::tlg5::Tlg5Encoder;
use crate::tlg6::{Tlg6Encoder};
use crate::tlg_type::{PixelLayout, TlgEncoderTrait, TlgType};

pub enum TlgEncoder {
    Tlg5(Tlg5Encoder),
    Tlg6(Tlg6Encoder),
}

impl TlgEncoder {
    pub fn width(&self) -> u32 {
        match self {
            TlgEncoder::Tlg5(e) => e.width(),
            TlgEncoder::Tlg6(e) => e.width(),
        }
    }

    pub fn height(&self) -> u32 {
        match self {
            TlgEncoder::Tlg5(e) => e.height(),
            TlgEncoder::Tlg6(e) => e.height(),
        }
    }

    pub fn pixel_layout(&self) -> PixelLayout {
        match self {
            TlgEncoder::Tlg5(e) => e.pixel_layout(),
            TlgEncoder::Tlg6(e) => e.pixel_layout(),
        }
    }

    pub fn encode(self) -> Result<Vec<u8>, Box<dyn Error>> {
        match self {
            TlgEncoder::Tlg5(e) => e.encode(),
            TlgEncoder::Tlg6(e) => e.encode(),
        }
    }

    pub fn encode_to<W: Write + Seek>(self, inner: &mut W) -> Result<(), Box<dyn Error>> {
        match self {
            TlgEncoder::Tlg5(e) => e.encode_to(inner),
            TlgEncoder::Tlg6(e) => e.encode_to(inner),
        }
    }
}

pub struct TlgWriter {
    encoder: TlgEncoder,
    tags: HashMap<String, String>,
}

impl TlgWriter {
    pub fn from_gray(
        data: Vec<u8>,
        tags: HashMap<String, String>,
        width: u32,
        height: u32,
        tlg_type: TlgType,
    ) -> TlgWriter {
        let encoder = match tlg_type {
            TlgType::Tlg5 => TlgEncoder::Tlg5(Tlg5Encoder::from_gray(data, width, height)),
            TlgType::Tlg6 => TlgEncoder::Tlg6(Tlg6Encoder::from_gray(data, width, height)),
        };
        TlgWriter::new(tags, encoder)
    }

    pub fn from_rgb(
        data: Vec<u8>,
        tags: HashMap<String, String>,
        width: u32,
        height: u32,
        tlg_type: TlgType,
    ) -> TlgWriter {
        let encoder = match tlg_type {
            TlgType::Tlg5 => TlgEncoder::Tlg5(Tlg5Encoder::from_rgb(data, width, height)),
            TlgType::Tlg6 => TlgEncoder::Tlg6(Tlg6Encoder::from_rgb(data, width, height)),
        };
        TlgWriter::new(tags, encoder)
    }

    pub fn from_rgba(
        data: Vec<u8>,
        tags: HashMap<String, String>,
        width: u32,
        height: u32,
        tlg_type: TlgType,
    ) -> TlgWriter {
        let encoder = match tlg_type {
            TlgType::Tlg5 => TlgEncoder::Tlg5(Tlg5Encoder::from_rgba(data, width, height)),
            TlgType::Tlg6 => TlgEncoder::Tlg6(Tlg6Encoder::from_rgba(data, width, height)),
        };
        TlgWriter::new(tags, encoder)
    }

    pub fn from_raw(
        data: Vec<u8>,
        tags: HashMap<String, String>,
        width: u32,
        height: u32,
        pixel_layout: PixelLayout,
        tlg_type: TlgType,
    ) -> TlgWriter {
        let encoder = match tlg_type {
            TlgType::Tlg5 => TlgEncoder::Tlg5(Tlg5Encoder::from_raw(data, pixel_layout, width, height)),
            TlgType::Tlg6 => TlgEncoder::Tlg6(Tlg6Encoder::from_raw(data, pixel_layout, width, height)),
        };
        TlgWriter::new(tags, encoder)
    }

    pub fn from_image(
        image: &DynamicImage,
        tags: HashMap<String, String>,
        tlg_type: TlgType,
    ) -> Result<TlgWriter, Box<dyn Error>> {
        let encoder = match tlg_type {
            TlgType::Tlg5 => TlgEncoder::Tlg5(Tlg5Encoder::from_image(image)?),
            TlgType::Tlg6 => TlgEncoder::Tlg6(Tlg6Encoder::from_image(image)?),
        };
        Ok(TlgWriter::new(tags, encoder))
    }

    fn new(tags: HashMap<String, String>, encoder: TlgEncoder) -> TlgWriter {
        TlgWriter { encoder, tags }
    }

    pub fn width(&self) -> u32 {
        self.encoder.width()
    }

    pub fn height(&self) -> u32 {
        self.encoder.height()
    }

    pub fn pixel_layout(&self) -> PixelLayout {
        self.encoder.pixel_layout()
    }

    pub fn tags(&self) -> &HashMap<String, String> {
        &self.tags
    }

    pub fn write_to<W: Write + Seek>(self, writer: &mut W) -> Result<(), Box<dyn Error>>
    {
        if !self.tags.is_empty() {
            writer.write_all(SDS_MAGIC)?;

            let tags = self.tags;

            let size_pos = writer.stream_position()?;
            writer.write_u32::<LittleEndian>(0u32)?;
            self.encoder.encode_to(writer)?;
            let stream_end = writer.stream_position()?;
            let size = stream_end - size_pos - 4;
            writer.seek(SeekFrom::Start(size_pos))?;
            writer.write_u32::<LittleEndian>(size as u32)?;
            writer.seek(SeekFrom::Start(stream_end))?;

            writer.write_all(b"tags".as_slice())?;
            let tags_data = tags_to_data(&tags);
            writer.write_u32::<LittleEndian>(tags_data.len() as u32)?;
            writer.write_all(tags_data.as_slice())?;
        }
        else {
            self.encoder.encode_to(writer)?;
        }
        Ok(())
    }

    pub fn write(self) -> Result<Vec<u8>, Box<dyn Error>>
    {
        let mut data = Vec::new();
        let mut cursor = Cursor::new(&mut data);
        self.write_to(&mut cursor)?;
        Ok(data)
    }
}

fn tags_to_data(tags: &HashMap<String, String>) -> Vec<u8> {
    let mut s = String::new();
    for (k, v) in tags {
        s.push_str(&format!("{}:{}={}:{},", k.len(), k, v.len(), v));
    };
    s.into_bytes()
}
