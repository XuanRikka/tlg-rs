use pyo3::PyResult;
use pyo3::prelude::*;
use pyo3::exceptions::PyException;
use pyo3_file::PyFileLikeObject;

use tlg_core::helper::get_tlg_type;
use crate::_types::PyTlgType;

#[pyfunction(name = "get_tlg_type")]
pub fn py_get_tlg_type(mut reader: PyFileLikeObject) -> PyResult<Option<PyTlgType>>
{
    let res = get_tlg_type(&mut reader);
    if res.is_ok()
    {
        let res = res.unwrap();
        if res.is_none()
        {
            return Ok(None)
        }
        Ok(Some(res.unwrap().into()))
    }
    else
    {
        Err(PyException::new_err(res.unwrap_err().to_string()))
    }
}