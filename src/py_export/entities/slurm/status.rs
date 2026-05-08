//! PyO3 wrappers for `entities::slurm::status::*`.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyclass_enum, gen_stub_pymethods};

use crate::entities::slurm::status as inner;

// ------------------------------------------------------- JobLifecycleStatus
#[gen_stub_pyclass_enum]
#[pyclass(
    name = "JobLifecycleStatus",
    module = "gaussian_job_shared._core.entities.slurm",
    from_py_object,
    eq,
    eq_int,
    hash,
    frozen
)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum PyJobLifecycleStatus {
    Queued,
    Running,
    Done,
    Failed,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyJobLifecycleStatus {
    fn __str__(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Done => "done",
            Self::Failed => "failed",
        }
    }

    fn __repr__(&self) -> String {
        format!("JobLifecycleStatus.{:?}", self)
    }
}

impl std::fmt::Debug for PyJobLifecycleStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Queued => f.write_str("Queued"),
            Self::Running => f.write_str("Running"),
            Self::Done => f.write_str("Done"),
            Self::Failed => f.write_str("Failed"),
        }
    }
}

impl From<inner::JobLifecycleStatus> for PyJobLifecycleStatus {
    fn from(v: inner::JobLifecycleStatus) -> Self {
        match v {
            inner::JobLifecycleStatus::Queued => Self::Queued,
            inner::JobLifecycleStatus::Running => Self::Running,
            inner::JobLifecycleStatus::Done => Self::Done,
            inner::JobLifecycleStatus::Failed => Self::Failed,
        }
    }
}

impl From<PyJobLifecycleStatus> for inner::JobLifecycleStatus {
    fn from(v: PyJobLifecycleStatus) -> Self {
        match v {
            PyJobLifecycleStatus::Queued => Self::Queued,
            PyJobLifecycleStatus::Running => Self::Running,
            PyJobLifecycleStatus::Done => Self::Done,
            PyJobLifecycleStatus::Failed => Self::Failed,
        }
    }
}

// ------------------------------------------------------------- StatusEntry
#[gen_stub_pyclass]
#[pyclass(
    name = "StatusEntry",
    module = "gaussian_job_shared._core.entities.slurm",
    from_py_object,
    eq
)]
#[derive(Clone, PartialEq, Eq)]
pub struct PyStatusEntry(pub inner::StatusEntry);

#[gen_stub_pymethods]
#[pymethods]
impl PyStatusEntry {
    #[new]
    #[pyo3(signature = (status, transitioned_at))]
    fn new(status: PyJobLifecycleStatus, transitioned_at: chrono::DateTime<chrono::Utc>) -> Self {
        Self(inner::StatusEntry {
            status: status.into(),
            transitioned_at,
        })
    }

    #[getter]
    fn status(&self) -> PyJobLifecycleStatus {
        self.0.status.into()
    }

    #[setter]
    fn set_status(&mut self, v: PyJobLifecycleStatus) {
        self.0.status = v.into();
    }

    #[getter]
    fn transitioned_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.0.transitioned_at
    }

    #[setter]
    fn set_transitioned_at(&mut self, v: chrono::DateTime<chrono::Utc>) {
        self.0.transitioned_at = v;
    }

    fn __repr__(&self) -> String {
        format!(
            "StatusEntry(status={:?}, transitioned_at={})",
            self.0.status, self.0.transitioned_at
        )
    }
}

impl From<inner::StatusEntry> for PyStatusEntry {
    fn from(v: inner::StatusEntry) -> Self {
        Self(v)
    }
}

impl From<PyStatusEntry> for inner::StatusEntry {
    fn from(v: PyStatusEntry) -> Self {
        v.0
    }
}
