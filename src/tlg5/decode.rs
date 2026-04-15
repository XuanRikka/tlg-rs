use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek};
use std::path::Path;
use byteorder::ReadBytesExt;
use image::DynamicImage;
use crate::tlg5::slide::SlideCompressor;
use crate::tlg_trait::{PixelLayout, TlgDecoderTrait};
use super::{TLG5_MAGIC, BLOCK_HEIGHT};


struct Tlg5Decoder
{
    data: Vec<u8>,
}

impl TlgDecoderTrait for Tlg5Decoder {
    fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized
    {
        let mut file = File::open(path)?;
        let mut data = Vec::with_capacity(file.metadata()?.len() as usize);
        std::io::copy(&mut file, &mut data)?;
        Ok(Tlg5Decoder { data })
    }

    fn from_reader<R: Read + Seek>(mut reader: R) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized
    {
        let mut data = Vec::new();
        std::io::copy(&mut reader, &mut data)?;
        Ok(Tlg5Decoder { data })
    }

    fn from_data(data: Vec<u8>) -> Result<Self, Box<dyn Error>>
    where
        Self: Sized
    {
        Ok(Tlg5Decoder { data })
    }

    fn decode(&self) -> Result<DynamicImage, Box<dyn Error>> {
        let mut cur = Cursor::new(self.data);
        let mut magic = vec![0u8; TLG5_MAGIC.len() as usize];
        cur.read_exact(magic.as_mut())?;
        if magic != TLG5_MAGIC {
            Err("Tlg5 decode wrong magic")?
        };

        let color = cur.read_u8()?;

        let width = cur.read_u32()? as usize;
        let height = cur.read_u32()? as usize;
        let block_height = cur.read_u32()?;

        let block_count = (height + BLOCK_HEIGHT - 1) / BLOCK_HEIGHT;
        let mut block_sizes = Vec::with_capacity(block_count as usize);

        for _ in 0..block_count
        {
            let size = cur.read_u32()? as usize;
            block_sizes.push(size);
        };

        let mut compressor = SlideCompressor::new();

        let mut blocks_data: Vec<Vec<u8>> = Vec::with_capacity(block_sizes.len());
        for block_size in block_sizes
        {
            let compressor_flag = cur.read_u8()?;
            let data_size = cur.read_u32()? as usize;

            let mut data = vec![0u8; data_size];
            cur.read_exact(&mut data)?;


            blocks_data.push(data);
        };


    }
}
