use pyo3::prelude::*;
use pyo3::types::{PyTraceback, PyType};
use pyo3::buffer::PyBuffer;
use pyo3::exceptions::{PyIOError, PyRuntimeError};
use pyo3_file::PyFileLikeObject;

use tlg_core::tlg6::Tlg6Encoder;
use tlg_core::TlgEncoderTrait;
use crate::_types::PyPixelLayout;

#[pyclass(name = "Tlg6Encoder", module = "tlg")]
pub struct PyTlg6Encoder
{
    inner: Option<Tlg6Encoder>
}

#[pymethods]
impl PyTlg6Encoder {
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

    fn encode_to(&mut self, mut writer: PyFileLikeObject) -> PyResult<()>
    {
        let inner = self.inner.take().ok_or_else(|| {
            PyRuntimeError::new_err("encoder already consumed")
        })?;

        inner.encode_to(&mut writer)
            .map_err(|e| PyIOError::new_err(e.to_string()))
    }

    fn encode(&mut self) -> PyResult<Vec<u8>>
    {
        let inner = self.inner.take().ok_or_else(|| {
            PyRuntimeError::new_err("encoder already consumed")
        })?;
        inner.encode()
            .map_err(|e| PyIOError::new_err(e.to_string()))
    }

    #[classmethod]
    fn from_gray(_cls: &Bound<'_, PyType>, data: PyBuffer<u8>, width: u32, height: u32)
        -> PyResult<PyTlg6Encoder>
    {
        let data_owned = Python::attach(|py| { data.to_vec(py) })?;
        Ok(
            PyTlg6Encoder {
                inner: Some(Tlg6Encoder::from_gray(data_owned, width, height))
            }
        )
    }

    #[classmethod]
    fn from_rgb(_cls: &Bound<'_, PyType>, data: PyBuffer<u8>, width: u32, height: u32)
        -> PyResult<PyTlg6Encoder>
    {
        let data_owned = Python::attach(|py| { data.to_vec(py) })?;
        Ok(
            PyTlg6Encoder {
                inner: Some(Tlg6Encoder::from_rgb(data_owned, width, height))
            }
        )
    }

    #[classmethod]
    fn from_rgba(_cls: &Bound<'_, PyType>, data: PyBuffer<u8>, width: u32, height: u32)
        -> PyResult<PyTlg6Encoder>
    {
        let data_owned = Python::attach(|py| { data.to_vec(py) })?;
        Ok(
            PyTlg6Encoder {
                inner: Some(Tlg6Encoder::from_rgba(data_owned, width, height))
            }
        )
    }

    #[classmethod]
    fn from_raw(_cls: &Bound<'_, PyType>, data: PyBuffer<u8>,
                width: u32, height: u32, pixel_layout: PyPixelLayout)
                -> PyResult<PyTlg6Encoder>
    {
        let data_owned = Python::attach(|py| { data.to_vec(py) })?;
        Ok(PyTlg6Encoder {
            inner: Some(Tlg6Encoder::from_raw(
                data_owned, pixel_layout.into(), width, height)
            ) })
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

impl PyTlg6Encoder
{
    fn inner_ref(&self) -> PyResult<&Tlg6Encoder> {
        self.inner
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("Tlg6Encoder has been consumed"))
    }
}
