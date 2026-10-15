pub(crate) mod _types;
mod reader;
mod writer;
mod helper;
pub mod tlg5;
pub mod tlg6;

use pyo3::prelude::*;

#[pymodule]
mod tlg {
    #[pymodule_export]
    use crate::_types::{PyTlgType, PyPixelLayout, PyImageInfo};
    #[pymodule_export]
    use crate::reader::PyTlgReader;
    #[pymodule_export]
    use crate::writer::PyTlgWriter;
    #[pymodule_export]
    use crate::tlg5::{PyTlg5Encoder, PyTlg5Decoder};
    #[pymodule_export]
    use crate::tlg6::{PyTlg6Encoder, PyTlg6Decoder};
    #[pymodule_export]
    use crate::helper::py_get_tlg_type;
}
