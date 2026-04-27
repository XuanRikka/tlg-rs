pub mod tlg5;
pub mod tlg6;
pub mod tlg_type;
pub mod slide;
pub mod writer;
pub mod reader;

pub use writer::TlgWriter;
pub use reader::TlgReader;

pub(crate) static SDS_MAGIC: &[u8; 11] = b"TLG0.0\x00sds\x1a";