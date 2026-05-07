use pyo3::{PyErr, exceptions::PyValueError};

use crate::error::SchemaParseError;

impl From<SchemaParseError> for PyErr {
    fn from(value: SchemaParseError) -> Self {
        PyValueError::new_err(value.to_string())
    }
}
