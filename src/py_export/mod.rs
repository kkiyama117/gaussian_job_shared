#![cfg(feature = "pyo3")]

pub mod entities;
pub mod error;

use pyo3::prelude::*;

pyo3_stub_gen::define_stub_info_gatherer!(stub_info);

/// A Python module implemented in Rust.
#[pymodule]
#[pyo3(name = "_core")]
mod gaussian_job_shared {
    use super::*;
    // TODO: constcat const PYTHON_LIBRARY_NAME: &str = "gaussian_job_shared";
    const PYTHON_MODULE_NAME: &str = "gaussian_job_shared._core";

    // ---- legacy demo function ----
    #[pymodule_export]
    use crate::py_export::sum_as_string;

    // ---- entities::slurm — newtypes & compound types ----
    #[pymodule_export]
    use crate::py_export::entities::slurm::job::{PyJob, PyJobEdge, PyJobId, PyJobSpec, PyProgram};

    // ---- entities::slurm::status ----
    #[pymodule_export]
    use crate::py_export::entities::slurm::status::{PyJobLifecycleStatus, PyStatusEntry};

    // ---- entities::slurm::dependency ----
    #[pymodule_export]
    use crate::py_export::entities::slurm::dependency::{
        PyDependencyClause, PyDependencyJobRef, PyDependencyJoin, PyDependencyType,
        PySlurmDependency,
    };

    // ---- entities::slurm::array_spec ----
    #[pymodule_export]
    use crate::py_export::entities::slurm::array_spec::{PyArrayIndex, PySlurmArraySpec};

    // ---- entities::slurm::resource_spec ----
    #[pymodule_export]
    use crate::py_export::entities::slurm::resource_spec::{
        PyMemory, PyMemoryUnit, PyResourceSpec, PyResourceSpecCPU, PyResourceSpecGPU,
    };

    // ---- entities::slurm::time_limit ----
    #[pymodule_export]
    use crate::py_export::entities::slurm::time_limit::PyJobTimeLimit;

    // ---- entities::slurm::config ----
    #[pymodule_export]
    use crate::py_export::entities::slurm::config::{
        PyMailType, PyMailTypeInput, PySlurmJobConfig,
    };

    // ---- entities::job_flow ----
    #[pymodule_export]
    use crate::py_export::entities::job_flow::{PyCalcType, PyJobFlow};

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        let py = m.py();
        py.import("sys")?
            .getattr("modules")?
            .set_item(PYTHON_MODULE_NAME, m)?;
        log::debug!("{} Rust module initialized", PYTHON_MODULE_NAME);
        Ok(())
    }
}

/// Formats the sum of two numbers as string.
#[pyo3_stub_gen::derive::gen_stub_pyfunction(module = "gaussian_job_shared._core")]
#[pyfunction]
fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
    Ok((a + b).to_string())
}
