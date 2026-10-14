pub(crate) mod _types;
mod reader;
mod writer;
mod helper;

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
    use crate::helper::py_get_tlg_type;
}
