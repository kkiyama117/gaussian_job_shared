#![cfg(feature = "pyo3-types")]

pub mod entities;
pub mod error;

// Stub-info gatherer (collects all #[gen_stub_*] annotations across
// the crate). Only useful when the wheel is being built with stub
// generation enabled.
#[cfg(feature = "stub_gen")]
pyo3_stub_gen::define_stub_info_gatherer!(stub_info);

// The outermost `_core` pymodule entry point. Compiled only when
// `pymodule-entry` is enabled — downstream library consumers
// (e.g. slurm-async-runner2 with `features = ["pyo3-types"]`)
// link the pyclass definitions but NOT `PyInit__core`, so they can
// expose their own pymodule without a duplicate-symbol collision.
#[cfg(feature = "pymodule-entry")]
mod pymodule_entry {
    use pyo3::prelude::*;

    /// A Python module implemented in Rust.
    #[pymodule]
    #[pyo3(name = "_core")]
    mod gaussian_job_shared {
        use super::*;
        // TODO: constcat const PYTHON_LIBRARY_NAME: &str = "gaussian_job_shared";
        const PYTHON_MODULE_NAME: &str = "gaussian_job_shared._core";

        #[pymodule_export]
        use crate::py_export::entities::inner_module;

        // ---- legacy demo function ----
        #[pymodule_export]
        use super::sum_as_string;

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
}
