pub mod tlg5;
pub mod tlg6;
pub mod tlg_type;
pub mod writer;
pub mod reader;
#[cfg(any(test, feature = "__bench"))]
pub mod slide;
#[cfg(not(any(test, feature = "__bench")))]
pub(crate) mod slide;

pub use writer::TlgWriter;
pub use reader::TlgReader;

pub(crate) static SDS_MAGIC: &[u8; 11] = b"TLG0.0\x00sds\x1a";