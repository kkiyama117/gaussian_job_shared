//! Python-facing wrappers for `crate::entities::*`.
//! See `docs/superpowers/specs/2026-05-08-rust-python-ffi-design.md`.

pub mod workflow;

use pyo3::prelude::*;

#[pymodule(name = "entities")]
pub(crate) mod inner_module {
    use super::*;

    const PYTHON_MODULE_NAME: &str = "gaussian_job_shared._core.entities";

    // Slurm vocab pymodule lives in slurm_async_runner now (per the
    // Pyclass Single Owner architecture rule). Python users import
    // slurm types from slurm_async_runner._slurm_async_runner_core.entities.slurm.*
    // directly.
    #[pymodule_export]
    use super::workflow::inner_module as workflow_module;

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
