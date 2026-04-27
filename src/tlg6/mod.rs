mod golomb;
mod filter;
mod predict;
mod encode;
mod decode;
#[cfg(any(test, feature = "__bench"))]
pub mod bitstream;
#[cfg(not(any(test, feature = "__bench")))]
pub(crate) mod bitstream;

pub(crate) const W_BLOCK_SIZE: usize = 8;
pub(crate) const H_BLOCK_SIZE: usize = 8;
pub(crate) static TLG6_MAGIC: &[u8; 11] = b"TLG6.0\x00raw\x1a";

pub use encode::Tlg6Encoder;
pub use decode::Tlg6Decoder;
