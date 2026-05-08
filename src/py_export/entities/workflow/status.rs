//! PyO3 wrappers for `entities::workflow::status::*`.
//!
//! `PyJobLifecycleStatus` follows the newtype + static-factory pattern
//! used by `PyArrayIndex` (see `src/py_export/entities/slurm/array_spec.rs`).
//! Three flat sub-enums (`PyQueuedKind`, `PyRunningKind`, `PyFailureKind`)
//! mirror the SLURM-token enums on the Rust side and are exposed
//! alongside.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyclass_enum, gen_stub_pymethods};

use crate::entities::workflow::status as inner;

// ============================================================== Sub-enums

#[gen_stub_pyclass_enum]
#[pyclass(
    name = "QueuedKind",
    module = "gaussian_job_shared._core.entities.workflow",
    from_py_object,
    eq,
    eq_int,
    hash,
    frozen
)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PyQueuedKind {
    Pending,
    Configuring,
    Requeued,
    RequeueFed,
    RequeueHold,
    ResvDelHold,
    Suspended,
    Stopped,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyQueuedKind {
    fn __str__(&self) -> &'static str {
        inner::QueuedKind::from(*self).as_token()
    }

    fn __repr__(&self) -> String {
        format!("QueuedKind.{:?}", self)
    }
}

impl From<inner::QueuedKind> for PyQueuedKind {
    fn from(v: inner::QueuedKind) -> Self {
        match v {
            inner::QueuedKind::Pending => Self::Pending,
            inner::QueuedKind::Configuring => Self::Configuring,
            inner::QueuedKind::Requeued => Self::Requeued,
            inner::QueuedKind::RequeueFed => Self::RequeueFed,
            inner::QueuedKind::RequeueHold => Self::RequeueHold,
            inner::QueuedKind::ResvDelHold => Self::ResvDelHold,
            inner::QueuedKind::Suspended => Self::Suspended,
            inner::QueuedKind::Stopped => Self::Stopped,
        }
    }
}

impl From<PyQueuedKind> for inner::QueuedKind {
    fn from(v: PyQueuedKind) -> Self {
        match v {
            PyQueuedKind::Pending => Self::Pending,
            PyQueuedKind::Configuring => Self::Configuring,
            PyQueuedKind::Requeued => Self::Requeued,
            PyQueuedKind::RequeueFed => Self::RequeueFed,
            PyQueuedKind::RequeueHold => Self::RequeueHold,
            PyQueuedKind::ResvDelHold => Self::ResvDelHold,
            PyQueuedKind::Suspended => Self::Suspended,
            PyQueuedKind::Stopped => Self::Stopped,
        }
    }
}

#[gen_stub_pyclass_enum]
#[pyclass(
    name = "RunningKind",
    module = "gaussian_job_shared._core.entities.workflow",
    from_py_object,
    eq,
    eq_int,
    hash,
    frozen
)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PyRunningKind {
    Running,
    Completing,
    Resizing,
    Signaling,
    StageOut,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyRunningKind {
    fn __str__(&self) -> &'static str {
        inner::RunningKind::from(*self).as_token()
    }

    fn __repr__(&self) -> String {
        format!("RunningKind.{:?}", self)
    }
}

impl From<inner::RunningKind> for PyRunningKind {
    fn from(v: inner::RunningKind) -> Self {
        match v {
            inner::RunningKind::Running => Self::Running,
            inner::RunningKind::Completing => Self::Completing,
            inner::RunningKind::Resizing => Self::Resizing,
            inner::RunningKind::Signaling => Self::Signaling,
            inner::RunningKind::StageOut => Self::StageOut,
        }
    }
}

impl From<PyRunningKind> for inner::RunningKind {
    fn from(v: PyRunningKind) -> Self {
        match v {
            PyRunningKind::Running => Self::Running,
            PyRunningKind::Completing => Self::Completing,
            PyRunningKind::Resizing => Self::Resizing,
            PyRunningKind::Signaling => Self::Signaling,
            PyRunningKind::StageOut => Self::StageOut,
        }
    }
}

#[gen_stub_pyclass_enum]
#[pyclass(
    name = "FailureKind",
    module = "gaussian_job_shared._core.entities.workflow",
    from_py_object,
    eq,
    eq_int,
    hash,
    frozen
)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PyFailureKind {
    BootFail,
    Cancelled,
    Deadline,
    Failed,
    NodeFail,
    OutOfMemory,
    Preempted,
    Revoked,
    SpecialExit,
    Timeout,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyFailureKind {
    fn __str__(&self) -> &'static str {
        inner::FailureKind::from(*self).as_token()
    }

    fn __repr__(&self) -> String {
        format!("FailureKind.{:?}", self)
    }
}

impl From<inner::FailureKind> for PyFailureKind {
    fn from(v: inner::FailureKind) -> Self {
        match v {
            inner::FailureKind::BootFail => Self::BootFail,
            inner::FailureKind::Cancelled => Self::Cancelled,
            inner::FailureKind::Deadline => Self::Deadline,
            inner::FailureKind::Failed => Self::Failed,
            inner::FailureKind::NodeFail => Self::NodeFail,
            inner::FailureKind::OutOfMemory => Self::OutOfMemory,
            inner::FailureKind::Preempted => Self::Preempted,
            inner::FailureKind::Revoked => Self::Revoked,
            inner::FailureKind::SpecialExit => Self::SpecialExit,
            inner::FailureKind::Timeout => Self::Timeout,
        }
    }
}

impl From<PyFailureKind> for inner::FailureKind {
    fn from(v: PyFailureKind) -> Self {
        match v {
            PyFailureKind::BootFail => Self::BootFail,
            PyFailureKind::Cancelled => Self::Cancelled,
            PyFailureKind::Deadline => Self::Deadline,
            PyFailureKind::Failed => Self::Failed,
            PyFailureKind::NodeFail => Self::NodeFail,
            PyFailureKind::OutOfMemory => Self::OutOfMemory,
            PyFailureKind::Preempted => Self::Preempted,
            PyFailureKind::Revoked => Self::Revoked,
            PyFailureKind::SpecialExit => Self::SpecialExit,
            PyFailureKind::Timeout => Self::Timeout,
        }
    }
}

// ====================================================== JobLifecycleStatus

/// Wraps the [`JobLifecycleStatus`] sum type. Construct one of the five
/// variants via the `queued`/`running`/`done`/`failed`/`unknown`
/// classmethods, or parse a raw SLURM token via `parse`. Inspect via
/// `kind` plus the matching `queued_kind()`/`running_kind()`/
/// `failure_kind()` accessor.
#[gen_stub_pyclass]
#[pyclass(
    name = "JobLifecycleStatus",
    module = "gaussian_job_shared._core.entities.workflow",
    from_py_object,
    eq,
    hash,
    frozen
)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PyJobLifecycleStatus(pub inner::JobLifecycleStatus);

#[gen_stub_pymethods]
#[pymethods]
impl PyJobLifecycleStatus {
    /// Build a `Queued(kind)` status.
    #[staticmethod]
    fn queued(kind: PyQueuedKind) -> Self {
        Self(inner::JobLifecycleStatus::Queued(kind.into()))
    }

    /// Build a `Running(kind)` status.
    #[staticmethod]
    fn running(kind: PyRunningKind) -> Self {
        Self(inner::JobLifecycleStatus::Running(kind.into()))
    }

    /// Build the `Done` status (terminal success).
    #[staticmethod]
    fn done() -> Self {
        Self(inner::JobLifecycleStatus::Done)
    }

    /// Build a `Failed(kind)` status (terminal failure).
    #[staticmethod]
    fn failed(kind: PyFailureKind) -> Self {
        Self(inner::JobLifecycleStatus::Failed(kind.into()))
    }

    /// Build the `Unknown` sentinel status.
    #[staticmethod]
    fn unknown() -> Self {
        Self(inner::JobLifecycleStatus::Unknown)
    }

    /// Parse a raw SLURM state token (`"PENDING"`, `"OUT_OF_MEMORY"`,
    /// `"CANCELLED by 1234"`, lowercase / compact codes / legacy
    /// workflow tokens, …). Falls back to `Unknown`.
    #[staticmethod]
    fn parse(raw: &str) -> Self {
        Self(inner::JobLifecycleStatus::parse(raw))
    }

    /// Outer discriminant: `"queued"`, `"running"`, `"done"`,
    /// `"failed"`, or `"unknown"`.
    #[getter]
    fn kind(&self) -> &'static str {
        match self.0 {
            inner::JobLifecycleStatus::Queued(_) => "queued",
            inner::JobLifecycleStatus::Running(_) => "running",
            inner::JobLifecycleStatus::Done => "done",
            inner::JobLifecycleStatus::Failed(_) => "failed",
            inner::JobLifecycleStatus::Unknown => "unknown",
        }
    }

    /// SLURM long-form token (e.g. `"PENDING"`, `"OUT_OF_MEMORY"`,
    /// `"COMPLETED"`, `"UNKNOWN"`).
    #[getter]
    fn token(&self) -> &'static str {
        self.0.as_token()
    }

    /// `Some(kind)` for `Queued(_)`, else `None`.
    fn queued_kind(&self) -> Option<PyQueuedKind> {
        match self.0 {
            inner::JobLifecycleStatus::Queued(k) => Some(k.into()),
            _ => None,
        }
    }

    /// `Some(kind)` for `Running(_)`, else `None`.
    fn running_kind(&self) -> Option<PyRunningKind> {
        match self.0 {
            inner::JobLifecycleStatus::Running(k) => Some(k.into()),
            _ => None,
        }
    }

    /// `Some(kind)` for `Failed(_)`, else `None`.
    fn failure_kind(&self) -> Option<PyFailureKind> {
        match self.0 {
            inner::JobLifecycleStatus::Failed(k) => Some(k.into()),
            _ => None,
        }
    }

    fn __str__(&self) -> String {
        self.0.as_token().to_ascii_lowercase()
    }

    fn __repr__(&self) -> String {
        format!("JobLifecycleStatus({:?})", self.0.as_token())
    }
}

impl From<inner::JobLifecycleStatus> for PyJobLifecycleStatus {
    fn from(v: inner::JobLifecycleStatus) -> Self {
        Self(v)
    }
}

impl From<PyJobLifecycleStatus> for inner::JobLifecycleStatus {
    fn from(v: PyJobLifecycleStatus) -> Self {
        v.0
    }
}

// ============================================================ StatusEntry

// StatusEntry intentionally omits `frozen`/`hash`: it has setters
// (set_status, set_transitioned_at) so it cannot be hashable.
#[gen_stub_pyclass]
#[pyclass(
    name = "StatusEntry",
    module = "gaussian_job_shared._core.entities.workflow",
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
            "StatusEntry(status={}, transitioned_at={})",
            PyJobLifecycleStatus(self.0.status).__repr__(),
            self.0.transitioned_at,
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
