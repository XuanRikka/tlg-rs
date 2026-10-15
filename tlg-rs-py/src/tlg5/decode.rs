use pyo3::prelude::*;
use pyo3::types::{PyTraceback, PyType};
use pyo3::exceptions::{PyIOError, PyRuntimeError};
use pyo3_file::PyFileLikeObject;

use tlg_core::tlg5::Tlg5Decoder;
use tlg_core::TlgDecoderTrait;
use crate::_types::PyImageInfo;

#[pyclass(name = "Tlg5Decoder", module = "tlg")]
pub struct PyTlg5Decoder
{
    inner: Option<Tlg5Decoder>
}

#[pymethods]
impl PyTlg5Decoder {
    #[new]
    fn new(reader: PyFileLikeObject) -> PyResult<PyTlg5Decoder>
    {
        Ok(
            PyTlg5Decoder {
                inner: Some(Tlg5Decoder::from_reader(reader)
                    .map_err(|e| PyIOError::new_err(e.to_string()))?)
            }
        )
    }

    #[classmethod]
    fn from_reader(_cls: &Bound<'_, PyType>, reader: PyFileLikeObject) -> PyResult<PyTlg5Decoder>
    {
        Ok(
            PyTlg5Decoder {
                inner: Some(Tlg5Decoder::from_reader(reader)
                    .map_err(|e| PyIOError::new_err(e.to_string()))?)
            }
        )
    }

    fn decode(&mut self) -> PyResult<(Vec<u8>, PyImageInfo)>
    {
        let inner = self.inner.take().ok_or_else(|| {
            PyRuntimeError::new_err("decoder already consumed")
        })?;
        inner.decode()
            .map(|(data, info)| (data, info.into()))
            .map_err(|e| PyIOError::new_err(e.to_string()))
    }

    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __exit__(
        &mut self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_val: Option<&Bound<'_, PyAny>>,
        _exc_tb: Option<&Bound<'_, PyTraceback>>,
    ) -> bool {
        self.inner = None;
        false
    }

    fn __del__(&mut self) {
        self.inner = None;
    }
}
