pub mod tlg5;
pub mod tlg6;
pub mod writer;
pub mod reader;
pub mod helper;
mod _types;

pub use writer::TlgWriter;
pub use reader::TlgReader;
pub use _types::{ImageInfo, PixelLayout, TlgType, TlgEncoderTrait, TlgDecoderTrait};

pub(crate) const SDS_MAGIC: &[u8; 11] = b"TLG0.0\x00sds\x1a";