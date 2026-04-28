use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use byteorder::{LittleEndian, ReadBytesExt};

#[cfg(any(test, feature = "image"))]
use image::DynamicImage;

use crate::SDS_MAGIC;
use crate::tlg5::{Tlg5Decoder, TLG5_MAGIC};
use crate::tlg6::{Tlg6Decoder, TLG6_MAGIC};
#[allow(unused_imports)]
use crate::tlg_type::{ImageInfo, PixelLayout, TlgDecoderTrait, TlgType};
use crate::tlg_type::TlgType::{Tlg5, Tlg6};


pub struct TlgReader<R: Read + Seek>
{
    reader: R,
}

impl<R: Read + Seek> TlgReader<R>
{
    pub fn new(reader: R) -> Self
    {
        TlgReader { reader }
    }

    pub fn from_reader(reader: R) -> Self
    {
        TlgReader::new(reader)
    }

    pub fn read(mut self) -> Result<(Vec<u8>, ImageInfo, HashMap<String, String>), Box<dyn Error>>
    {
        let mut magic = [0u8; 11];
        self.reader.read_exact(&mut magic)?;

        let mut image_stream;
        let tlg_type: TlgType;

        if &magic == SDS_MAGIC
        {
            let raw_size = self.reader.read_u32::<LittleEndian>()?;
            image_stream = self.reader.by_ref().take(raw_size as u64);

            let start_pos = image_stream.stream_position()?;

            let mut raw_magic = [0u8; 11];
            image_stream.read_exact(&mut raw_magic)?;

            image_stream.seek(SeekFrom::Start(start_pos))?;
            tlg_type = if &magic == TLG5_MAGIC {Tlg5}
                else if &magic == TLG6_MAGIC {Tlg6}
                else {return Err("Invalid magic".into())}
        }
        else {
            image_stream = self.reader.by_ref().take(u64::MAX);
            tlg_type = if &magic == TLG5_MAGIC {Tlg5}
                else if &magic == TLG6_MAGIC {Tlg6}
                else {return Err("Invalid magic".into())}
        }


        let result = match tlg_type
        {
            Tlg5 => {
                let decoder = Tlg5Decoder::from_reader(image_stream)?;
                decoder.decode()?
            }
            Tlg6 => {
                let decoder = Tlg6Decoder::from_reader(image_stream)?;
                decoder.decode()?
            }
        };

        let tags: HashMap<String, String>;

        if &magic == SDS_MAGIC
        {
            let mut tags_magic = [0u8; 4];
            self.reader.read_exact(&mut tags_magic)?;

            let tags_size = self.reader.read_u32::<LittleEndian>()?;

            let mut tags_data = vec![0u8; tags_size as usize];
            self.reader.read_exact(&mut tags_data)?;

            tags = data_to_tags(tags_data.as_slice())?
        }
        else
        {
            tags = HashMap::new();
        }

        let (data, info) = result;

        Ok((data, info, tags))
    }

    #[cfg(any(test, feature = "image"))]
    pub fn read_to_image(self) -> Result<(DynamicImage, HashMap<String, String>), Box<dyn Error>>
    {
        let (data, info, tags) = self.read()?;

        match info.pixel_layout
        {
            PixelLayout::Gray => {
                Ok((DynamicImage::ImageRgb8(
                    image::RgbImage::from_raw(info.width, info.height, data)
                        .ok_or("Failed to create gray image")?,
                ), tags))
            },
            PixelLayout::Rgb => {
                Ok((DynamicImage::ImageRgb8(
                    image::RgbImage::from_raw(info.width, info.height, data)
                        .ok_or("Failed to create rgb image")?,
                ), tags))
            },
            PixelLayout::Rgba => {
                Ok((DynamicImage::ImageRgba8(
                    image::RgbaImage::from_raw(info.width, info.height, data)
                        .ok_or("Failed to create rgba image")?,
                ), tags))
            }
        }
    }
}

impl TlgReader<BufReader<File>> {
    pub fn from_file(file: File) -> Self
    {
        TlgReader::new(BufReader::new(file))
    }

    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>>
    {
        let file = File::open(path)?;
        Ok(TlgReader::new(BufReader::new(file)))
    }
}

fn data_to_tags(data: &[u8]) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let s = String::from_utf8(data.to_vec())?;
    let mut tags = HashMap::new();

    for entry in s.split(',').filter(|e| !e.is_empty()) {
        let parts: Vec<&str> = entry.splitn(2, '=').collect();
        if parts.len() != 2 {
            continue;
        }

        let key_part = parts[0];
        let key_components: Vec<&str> = key_part.splitn(2, ':').collect();
        if key_components.len() != 2 {
            continue;
        }
        let key_len: usize = key_components[0].parse()?;
        let key = key_components[1];

        if key.len() != key_len {
            continue;
        }

        let value_part = parts[1];
        let value_components: Vec<&str> = value_part.splitn(2, ':').collect();
        if value_components.len() != 2 {
            continue;
        }
        let value_len: usize = value_components[0].parse()?;
        let value = value_components[1];

        if value.len() != value_len {
            continue;
        }

        tags.insert(key.to_string(), value.to_string());
    }

    Ok(tags)
}
