use pyo3::{PyErr, exceptions::PyValueError};

use crate::error::StrictSchemaError;

impl From<StrictSchemaError> for PyErr {
    fn from(value: StrictSchemaError) -> Self {
        PyValueError::new_err(value.to_string())
    }
}
