use std::error::Error;
use std::io::{Seek, Write};
use std::collections::HashMap;
use image::DynamicImage;
use crate::tlg5::Tlg5Encoder;
use crate::tlg6::Tlg6Encoder;
use crate::tlg_trait::{PixelLayout, TlgEncoderTrait, TlgType};

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

    pub fn encode(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        match self {
            TlgEncoder::Tlg5(e) => e.encode(),
            TlgEncoder::Tlg6(e) => e.encode(),
        }
    }

    pub fn encode_to<W: Write + Seek>(&self, inner: &mut W) -> Result<(), Box<dyn Error>> {
        match self {
            TlgEncoder::Tlg5(e) => e.encode_to(inner),
            TlgEncoder::Tlg6(e) => e.encode_to(inner),
        }
    }
}

pub struct TlgWrite {
    encoder: TlgEncoder,
    tags: HashMap<String, String>,
}

impl TlgWrite {
    pub fn from_gray(
        data: Vec<u8>,
        tags: HashMap<String, String>,
        width: u32,
        height: u32,
        tlg_type: TlgType,
    ) -> TlgWrite {
        let encoder = match tlg_type {
            TlgType::Tlg5 => TlgEncoder::Tlg5(Tlg5Encoder::from_gray(data, width, height)),
            TlgType::Tlg6 => TlgEncoder::Tlg6(Tlg6Encoder::from_gray(data, width, height)),
        };
        TlgWrite::new(tags, encoder)
    }

    pub fn from_rgb(
        data: Vec<u8>,
        tags: HashMap<String, String>,
        width: u32,
        height: u32,
        tlg_type: TlgType,
    ) -> TlgWrite {
        let encoder = match tlg_type {
            TlgType::Tlg5 => TlgEncoder::Tlg5(Tlg5Encoder::from_rgb(data, width, height)),
            TlgType::Tlg6 => TlgEncoder::Tlg6(Tlg6Encoder::from_rgb(data, width, height)),
        };
        TlgWrite::new(tags, encoder)
    }

    pub fn from_rgba(
        data: Vec<u8>,
        tags: HashMap<String, String>,
        width: u32,
        height: u32,
        tlg_type: TlgType,
    ) -> TlgWrite {
        let encoder = match tlg_type {
            TlgType::Tlg5 => TlgEncoder::Tlg5(Tlg5Encoder::from_rgba(data, width, height)),
            TlgType::Tlg6 => TlgEncoder::Tlg6(Tlg6Encoder::from_rgba(data, width, height)),
        };
        TlgWrite::new(tags, encoder)
    }

    pub fn from_raw(
        data: Vec<u8>,
        tags: HashMap<String, String>,
        width: u32,
        height: u32,
        pixel_layout: PixelLayout,
        tlg_type: TlgType,
    ) -> TlgWrite {
        let encoder = match tlg_type {
            TlgType::Tlg5 => TlgEncoder::Tlg5(Tlg5Encoder::from_raw(data, pixel_layout, width, height)),
            TlgType::Tlg6 => TlgEncoder::Tlg6(Tlg6Encoder::from_raw(data, pixel_layout, width, height)),
        };
        TlgWrite::new(tags, encoder)
    }

    pub fn from_image(
        image: &DynamicImage,
        tags: HashMap<String, String>,
        tlg_type: TlgType,
    ) -> Result<TlgWrite, Box<dyn Error>> {
        let encoder = match tlg_type {
            TlgType::Tlg5 => TlgEncoder::Tlg5(Tlg5Encoder::from_image(image)?),
            TlgType::Tlg6 => TlgEncoder::Tlg6(Tlg6Encoder::from_image(image)?),
        };
        Ok(TlgWrite::new(tags, encoder))
    }

    fn new(tags: HashMap<String, String>, encoder: TlgEncoder) -> TlgWrite {
        TlgWrite { encoder, tags }
    }

    pub fn encode(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        self.encoder.encode()
    }

    pub fn encode_to<W: Write + Seek>(&self, inner: &mut W) -> Result<(), Box<dyn Error>> {
        self.encoder.encode_to(inner)
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
}
