use tlg_core::{TlgType, PixelLayout, ImageInfo};

use pyo3::prelude::*;

#[pyclass(eq, eq_int, from_py_object, name = "PixelLayout", module = "tlg")]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PyPixelLayout {
    Gray = 0,
    Rgb  = 1,
    Rgba = 2,
}

#[pyclass(eq, eq_int, from_py_object, name = "TlgType", module = "tlg")]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PyTlgType {
    Tlg5 = 5,
    Tlg6 = 6,
}

impl From<PixelLayout> for PyPixelLayout {
    fn from(v: PixelLayout) -> Self {
        match v {
            PixelLayout::Gray => PyPixelLayout::Gray,
            PixelLayout::Rgb  => PyPixelLayout::Rgb,
            PixelLayout::Rgba => PyPixelLayout::Rgba,
        }
    }
}

impl From<PyPixelLayout> for PixelLayout {
    fn from(v: PyPixelLayout) -> Self {
        match v {
            PyPixelLayout::Gray => PixelLayout::Gray,
            PyPixelLayout::Rgb  => PixelLayout::Rgb,
            PyPixelLayout::Rgba => PixelLayout::Rgba,
        }
    }
}

impl From<TlgType> for PyTlgType {
    fn from(v: TlgType) -> Self {
        match v {
            TlgType::Tlg5 => PyTlgType::Tlg5,
            TlgType::Tlg6 => PyTlgType::Tlg6,
        }
    }
}

impl From<PyTlgType> for TlgType {
    fn from(v: PyTlgType) -> Self {
        match v {
            PyTlgType::Tlg5 => TlgType::Tlg5,
            PyTlgType::Tlg6 => TlgType::Tlg6,
        }
    }
}

#[pyclass(name = "ImageInfo", module = "tlg", from_py_object, eq)]
#[derive(Clone, PartialEq, Eq)]
pub struct PyImageInfo {
    pub(crate) inner: ImageInfo,
}

#[pymethods]
impl PyImageInfo {
    #[new]
    #[pyo3(signature = (width, height, pixel_layout))]
    fn new(width: u32, height: u32, pixel_layout: PyPixelLayout) -> Self {
        PyImageInfo {
            inner: ImageInfo {
                width,
                height,
                pixel_layout: pixel_layout.into(),
            },
        }
    }

    #[getter]
    fn width(&self) -> u32 {
        self.inner.width
    }

    #[getter]
    fn height(&self) -> u32 {
        self.inner.height
    }

    #[getter]
    fn pixel_layout(&self) -> PyPixelLayout {
        self.inner.pixel_layout.into()
    }

    fn __repr__(&self) -> String {
        format!(
            "ImageInfo(width={}, height={}, pixel_layout={:?})",
            self.inner.width, self.inner.height, self.inner.pixel_layout
        )
    }
}

impl From<ImageInfo> for PyImageInfo {
    fn from(inner: ImageInfo) -> Self {
        PyImageInfo { inner }
    }
}

impl From<PyImageInfo> for ImageInfo {
    fn from(v: PyImageInfo) -> Self {
        v.inner
    }
}
