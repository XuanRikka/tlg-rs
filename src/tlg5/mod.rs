pub mod encode;
pub mod decode;

pub static BLOCK_HEIGHT: usize = 4;
pub static TLG5_MAGIC: &[u8; 11] = b"TLG5.0\x00raw\x1a"; // 按理说后面还有个\x00，但是实际上这部分根本不会被写入，天知道他们为什么这么写
