use std::fs::File;
use std::io::BufReader;

use tlg::TlgReader;

fn main() {
    let file = File::open("aaa.tlg").unwrap();
    let reader = BufReader::new(file);
    let decoder = TlgReader::from_reader(reader);

    let (image, tags) = decoder.read().unwrap();
    println!("{:?}", tags);
    image.save("out.png").unwrap();
}
