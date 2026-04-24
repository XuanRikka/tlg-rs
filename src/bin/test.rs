use std::fs::File;
use std::io::Write;
use image::ImageReader;
use tlg::tlg5::encode::Tlg5Encoder;
use tlg::tlg6::Tlg6Encoder;
use tlg::tlg_trait::TlgEncoderTrait;

fn main()
{
    let i = ImageReader::open("aaa.png").unwrap();
    let d = i.decode().unwrap();

    let tlg5 = Tlg6Encoder::from_image(&d).unwrap();
    let data = tlg5.encode().unwrap();
    let mut file = File::create("aaa.tlg").unwrap();
    file.write_all(data.as_slice()).unwrap();
}
