pub mod tlg5;
pub mod tlg_trait;
pub mod tlg6;
// use crate::tlg5::encode::Tlg5Encoder;
// use crate::tlg_trait::{PixelLayout, TlgEncoderTrait};
// use crate::TlgType::Tlg5;

// pub mod tlg5;
// mod tlg_trait;
//
// enum TlgType
// {
//     Tlg5,
//     Tlg6
// }
//
//
// struct TlgWrite
// {
//     tlg_type: TlgType,
//     encoder: Box<dyn TlgEncoderTrait>,
//     tags: HashMap<String, String>
// }
//
// impl TlgWrite {
//     pub fn from_gray(
//         data: Vec<u8>,
//         tags: HashMap<String, String>,
//         width: u32,
//         height: u32,
//         tlg_type: TlgType,
//     ) -> TlgWrite {
//         let encoder: Box<dyn TlgEncoderTrait> = match tlg_type {
//             TlgType::Tlg5 => Box::new(Tlg5Encoder::from_gray(data, width, height)),
//             TlgType::Tlg6 => todo!(),
//         };
//         TlgWrite::new(tlg_type, tags, encoder)
//     }
//
//     pub fn from_rgb(
//         data: Vec<u8>,
//         tags: HashMap<String, String>,
//         width: u32,
//         height: u32,
//         tlg_type: TlgType,
//     ) -> TlgWrite {
//         let encoder: Box<dyn TlgEncoderTrait> = match tlg_type {
//             TlgType::Tlg5 => Box::new(Tlg5Encoder::from_rgb(data, width, height)),
//             TlgType::Tlg6 => todo!(),
//         };
//         TlgWrite::new(tlg_type, tags, encoder)
//     }
//
//     pub fn from_rgba(
//         data: Vec<u8>,
//         tags: HashMap<String, String>,
//         width: u32,
//         height: u32,
//         tlg_type: TlgType,
//     ) -> TlgWrite {
//         let encoder: Box<dyn TlgEncoderTrait> = match tlg_type {
//             TlgType::Tlg5 => Box::new(Tlg5Encoder::from_rgba(data, width, height)),
//             TlgType::Tlg6 => todo!(),
//         };
//         TlgWrite::new(tlg_type, tags, encoder)
//     }
//
//     pub fn from_raw(
//         data: Vec<u8>,
//         tags: HashMap<String, String>,
//         width: u32,
//         height: u32,
//         pixel_layout: PixelLayout,
//         tlg_type: TlgType,
//     ) -> TlgWrite {
//         let encoder: Box<dyn TlgEncoderTrait> = match tlg_type {
//             TlgType::Tlg5 => {
//                 Box::new(Tlg5Encoder::from_raw(data, pixel_layout, width, height))
//             }
//             TlgType::Tlg6 => {
//                 todo!()
//             }
//         };
//         TlgWrite::new(tlg_type, tags, encoder)
//     }
//
//     fn new(
//         tlg_type: TlgType,
//         tags: HashMap<String, String>,
//         encoder: Box<dyn TlgEncoderTrait>,
//     ) -> TlgWrite {
//         TlgWrite {
//             tlg_type,
//             tags,
//             encoder,
//         }
//     }
//
//     pub fn encode(&self) -> Result<Vec<u8>, Box<dyn Error>> {
//         let encoded_data = self.encoder.encode()?;
//
//
//
//         Ok(encoded_data)
//     }
//
//     pub fn width(&self) -> u32 {
//         self.encoder.width()
//     }
//
//     pub fn height(&self) -> u32 {
//         self.encoder.height()
//     }
// }
