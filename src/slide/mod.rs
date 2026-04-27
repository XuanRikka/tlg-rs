mod encode;
mod decode;

pub use encode::SlideEncoder;
pub use decode::SlideDecoder;

pub(crate) const SLIDE_N: usize = 4096;
pub(crate) const SLIDE_M: usize = 18 + 255;

#[cfg(test)]
mod test {
    use super::{SlideEncoder, SlideDecoder};

    #[test]
    fn test_encode() {
        let data = "你其实是猪".repeat(32).into_bytes();
        let c = SlideEncoder::new().encode(&data);
        let test_data: &[u8] = include_bytes!("test.bin");
        assert_eq!(c, test_data);
    }

    #[test]
    fn encode_and_decode() {
        let data = "你其实是猪".repeat(32).into_bytes();
        let c = SlideEncoder::new().encode(&data);
        let d = SlideDecoder::new().decode(&c);
        assert_eq!(data, d);
    }

    #[test]
    fn test_decode() {
        let test_data: &[u8] = include_bytes!("test.bin");
        let data = "你其实是猪".repeat(32).into_bytes();
        let c = SlideDecoder::new().decode(test_data);
        assert_eq!(c, data);
    }
}
