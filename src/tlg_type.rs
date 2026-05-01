use std::error::Error;
use std::io::{Read, Seek, Write};
use std::path::Path;

#[cfg(any(test, feature = "image"))]
use image::DynamicImage;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PixelLayout {
    Gray,
    Rgb,
    Rgba,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TlgType
{
    Tlg5,
    Tlg6
}

pub struct  ImageInfo
{
    pub width: u32,
    pub height: u32,
    pub pixel_layout: PixelLayout,
}


/// TLG 编码器 trait
pub trait TlgEncoderTrait {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn pixel_layout(&self) -> PixelLayout;

    fn encode_to<W: Write + Seek>(self, inner: &mut W) -> Result<(), Box<dyn Error>>;
    fn encode(self) -> Result<Vec<u8>, Box<dyn Error>>;

    #[cfg(any(test, feature = "image"))]
    fn from_image(image: &DynamicImage) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized;

    fn from_gray(data: Vec<u8>, width: u32, height: u32) -> Self
    where
        Self: Sized;

    fn from_rgb(data: Vec<u8>, width: u32, height: u32) -> Self
    where
        Self: Sized;

    fn from_rgba(data: Vec<u8>, width: u32, height: u32) -> Self
    where
        Self: Sized;

    fn from_raw(data: Vec<u8>, pixel_layout: PixelLayout, width: u32, height: u32) -> Self
    where
        Self: Sized;
}


pub trait TlgDecoderTrait {
    #[cfg(not(target_arch = "wasm32"))]
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized;

    fn from_reader<R: Read + Seek>(reader: R) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized;

    fn from_data(data: Vec<u8>) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized;

    fn decode(self) -> Result<(Vec<u8>, ImageInfo), Box<dyn Error>>;

    #[cfg(any(test, feature = "image"))]
    fn decode_to_image(self) -> Result<DynamicImage, Box<dyn Error>>;
}
