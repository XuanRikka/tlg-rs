use std::collections::HashMap;
use pyo3::prelude::*;
use pyo3::types::{PyTraceback, PyType};
use pyo3_file::PyFileLikeObject;
use tlg_core::reader::TlgReader;
use crate::_types::PyImageInfo;

#[pyclass(name = "TlgReader", module = "tlg")]
pub struct PyTlgReader
{
    inner: Option<TlgReader<PyFileLikeObject>>,
}

#[pymethods]
impl PyTlgReader {
    #[new]
    pub fn new(obj: PyFileLikeObject) -> PyTlgReader
    {
        PyTlgReader { inner: Some(TlgReader::new(obj)) }
    }

    #[classmethod]
    pub fn from_reader(_cls: &Bound<'_, PyType>, reader: PyFileLikeObject) -> PyTlgReader
    {
        PyTlgReader { inner: Some(TlgReader::new(reader)) }
    }

    pub fn read(&mut self) -> PyResult<(Vec<u8>, PyImageInfo, HashMap<String, String>)>
    {
        let inner = self.inner.take().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("reader already consumed")
        })?;
        inner.read()
            .map(
                |(data, info, metadata)|
                    (data, info.into(), metadata)
            )
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
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

