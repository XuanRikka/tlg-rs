pub mod slide;

use std::error::Error;
use image::DynamicImage;


#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PixelLayout {
    Gray,
    Rgb,
    Rgba,
}

static TLG5_MAGIC: &[u8; 11] = b"TLG5.0\x00raw\x1a"; // 按理说后面还有个\x00，但是实际上这部分根本不会被写入，天知道他们为什么这么写

struct Tlg5Encode
{
    data: Vec<u8>,
    height: u32,
    width: u32,
    pixel: PixelLayout
}

impl Tlg5Encode
{
    pub fn from_image(image: &DynamicImage) -> Result<Tlg5Encode, Box<dyn Error>>
    {
        match image {
            DynamicImage::ImageLuma8(image) => {
                let width = image.width();
                let height = image.height();
                Ok(Tlg5Encode::from_gray(image.to_vec(), width, height))
            }
            DynamicImage::ImageRgb8(image) => {
                let width = image.width();
                let height = image.height();
                Ok(Tlg5Encode::from_rgb(image.to_vec(), width, height))
            }
            DynamicImage::ImageRgba8(image) => {
                let width = image.width();
                let height = image.height();
                Ok(Tlg5Encode::from_rgba(image.to_vec(), width, height))
            }
            _ => {
                Err(Box::from("Unimplemented"))
            },
        }
    }

    pub fn from_rgb(data: Vec<u8>, width: u32, height: u32) -> Tlg5Encode
    {
        Tlg5Encode { data,width,height,pixel: PixelLayout::Rgb }
    }

    pub fn from_rgba(data: Vec<u8>, width: u32, height: u32) -> Tlg5Encode
    {
        Tlg5Encode { data,width,height,pixel: PixelLayout::Rgba }
    }

    pub fn from_gray(data: Vec<u8>, width: u32, height: u32) -> Tlg5Encode
    {
        Tlg5Encode { data,width,height,pixel: PixelLayout::Gray }
    }

    pub fn encode(self)// -> Result<Vec<u8>, Box<dyn Error>>
    {

    }

    fn encode_rgb(self)
    {

    }
}

