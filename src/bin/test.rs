use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use image::ImageReader;
use tlg::TlgWriter;
use tlg::tlg_type::{TlgEncoderTrait, TlgType};

fn main()
{
    let i = ImageReader::open("aaa.png").unwrap();
    let d = i.decode().unwrap();

    let mut tags = HashMap::new();
    tags.insert("for".to_string(), "tlg-rs".to_string());
    let tlg5 = TlgWriter::from_image(&d, tags, TlgType::Tlg6).unwrap();
    let data = tlg5.write().unwrap();
    let mut file = File::create("aaa.tlg").unwrap();
    file.write_all(data.as_slice()).unwrap();
}
