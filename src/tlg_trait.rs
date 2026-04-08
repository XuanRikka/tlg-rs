use std::error::Error;
use std::io::{Seek,Write};
use image::DynamicImage;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PixelLayout {
    Gray,
    Rgb,
    Rgba,
}


/// TLG 编码器 trait
pub trait TlgEncoderTrait {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn pixel_layout(&self) -> PixelLayout;

    fn encode_to<W: Write + Seek>(&self, inner: &mut W) -> Result<(), Box<dyn Error>>;
    fn encode(&self) -> Result<Vec<u8>, Box<dyn Error>>;

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