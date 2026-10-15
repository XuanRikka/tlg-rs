use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::types::{PyTraceback, PyType};
use pyo3::buffer::PyBuffer;
use pyo3::exceptions::{PyIOError, PyRuntimeError};
use pyo3_file::PyFileLikeObject;

use tlg_core::{TlgWriter};
use crate::_types::{PyPixelLayout, PyTlgType};

#[pyclass(name = "TlgWriter", module = "tlg")]
pub struct PyTlgWriter
{
    inner: Option<TlgWriter>,
}

#[pymethods]
impl PyTlgWriter
{
    #[classmethod]
    fn from_gray(_cls: &Bound<'_, PyType>, data: PyBuffer<u8>, tags: HashMap<String, String>,
                 width: u32, height: u32, tlg_type: PyTlgType) -> PyResult<PyTlgWriter>
    {
        let data_owned = Python::attach(|py| { data.to_vec(py) })?;
        Ok(
            PyTlgWriter {
                inner: Some(TlgWriter::from_gray(data_owned, tags, width, height, tlg_type.into()))
            }
        )
    }

    #[classmethod]
    fn from_rgb(_cls: &Bound<'_, PyType>, data: PyBuffer<u8>, tags: HashMap<String, String>,
                 width: u32, height: u32, tlg_type: PyTlgType) -> PyResult<PyTlgWriter>
    {
        let data_owned = Python::attach(|py| { data.to_vec(py) })?;
        Ok(
            PyTlgWriter {
                inner: Some(TlgWriter::from_rgb(data_owned, tags, width, height, tlg_type.into()))
            }
        )
    }

    #[classmethod]
    fn from_rgba(_cls: &Bound<'_, PyType>, data: PyBuffer<u8>, tags: HashMap<String, String>,
                width: u32, height: u32, tlg_type: PyTlgType) -> PyResult<PyTlgWriter>
    {
        let data_owned = Python::attach(|py| { data.to_vec(py) })?;
        Ok(
            PyTlgWriter {
                inner: Some(TlgWriter::from_rgba(data_owned, tags, width, height, tlg_type.into()))
            }
        )
    }

    #[classmethod]
    fn from_raw(_cls: &Bound<'_, PyType>, data: PyBuffer<u8>, tags: HashMap<String, String>,
                width: u32, height: u32, pixel_layout: PyPixelLayout, tlg_type: PyTlgType)
        -> PyResult<PyTlgWriter>
    {
        let data_owned = Python::attach(|py| { data.to_vec(py) })?;
        Ok(PyTlgWriter {
                inner: Some(TlgWriter::from_raw(
                    data_owned, tags, width, height, pixel_layout.into(), tlg_type.into())
                ) })
    }

    fn write(&mut self) -> PyResult<Vec<u8>>
    {
        let inner = self.inner.take().ok_or_else(|| {
            PyRuntimeError::new_err("reader already consumed")
        })?;
        inner.write()
            .map_err(|e| PyIOError::new_err(e.to_string()))
    }

    fn write_to(&mut self, mut writer: PyFileLikeObject) -> PyResult<()>
    {
        let inner = self.inner.take().ok_or_else(|| {
            PyRuntimeError::new_err("reader already consumed")
        })?;
        inner.write_to(&mut writer)
            .map_err(|e| PyIOError::new_err(e.to_string()))
    }

    #[getter]
    fn width(&self) -> PyResult<u32> {
        Ok(self.inner_ref()?.width())
    }

    #[getter]
    fn height(&self) -> PyResult<u32> {
        Ok(self.inner_ref()?.height())
    }

    #[getter]
    fn pixel_layout(&self) -> PyResult<PyPixelLayout>
    {
        Ok(self.inner_ref()?.pixel_layout().into())
    }

    #[getter]
    fn tags(&self) -> PyResult<HashMap<String, String>>
    {
        Ok(self.inner_ref()?.tags().to_owned())
    }

    #[getter]
    fn tlg_type(&self) -> PyResult<PyTlgType>
    {
        Ok(self.inner_ref()?.tlg_type().into())
    }

    fn __repr__(&self) -> PyResult<String>
    {
        Ok(format!("TlgWriter(tags={:?}, width={}, height={}, tlg_type={:?}, pixel_layout={:?})",
                    self.tags()?, self.width()?, self.height()?, self.tlg_type()?, self.pixel_layout()?))
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

impl PyTlgWriter {
    fn inner_ref(&self) -> PyResult<&TlgWriter> {
        self.inner
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("TlgWriter has been consumed"))
    }
}
