use std::io::{Read, Seek, SeekFrom};

use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt};
use crate::{tlg5, TlgType, tlg6, SDS_MAGIC};

pub fn get_tlg_type<R: Read + Seek>(reader: &mut R) -> Result<Option<TlgType>>
{
    let start_pos = reader.stream_position()?;

    let mut magic = [0u8; 11];
    reader.read_exact(&mut magic)?;

    if &magic == SDS_MAGIC
    {
        let _ = reader.read_u32::<LittleEndian>();
        reader.read_exact(&mut magic)?;
    }

    reader.seek(SeekFrom::Start(start_pos))?;

    match &magic
    {
        tlg5::TLG5_MAGIC => Ok(Some(TlgType::Tlg5)),
        tlg6::TLG6_MAGIC => Ok(Some(TlgType::Tlg6)),
        _ => Ok(None)
    }
}