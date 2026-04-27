use std::fs::File;
use std::io::BufReader;

use tlg::tlg5::Tlg5Decoder;
use tlg::tlg6::Tlg6Decoder;
use tlg::tlg_type::TlgDecoderTrait;

fn main() {
    let file = File::open("aaa.tlg").unwrap();
    let reader = BufReader::new(file);
    let decoder = Tlg6Decoder::from_reader(reader).unwrap();

    let image = decoder.decode().unwrap();
    image.save("out.png").unwrap();
}
