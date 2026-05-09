//! Python-facing wrappers for `crate::entities::workflow::*` (the workflow
//! tier — DAG-shaped flow that *uses* SLURM types but is not SLURM-internal).
//! Job lifecycle status (`PyJobStatus`, `PyJobState`, `PyJobReason`) lives
//! under `crate::py_export::entities::slurm::status` since it mirrors
//! SLURM's own `(state, reason)` pair.

pub mod job;

use std::collections::BTreeMap;
use std::path::PathBuf;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use uuid::Uuid;

use crate::entities::workflow as inner;

use self::job::{PyJob, PyJobId};

// ------------------------------------------------------------------ JobFlow
#[gen_stub_pyclass]
#[pyclass(
    name = "JobFlow",
    module = "gaussian_job_shared._gaussian_job_shared_core.entities.workflow",
    from_py_object
)]
#[derive(Clone)]
pub struct PyJobFlow(pub inner::JobFlow);

#[gen_stub_pymethods]
#[pymethods]
impl PyJobFlow {
    /// Build a `JobFlow`. `uuid` accepts the canonical hyphenated string form
    /// (e.g. `"01997cdc-…"`). To generate a fresh UUID v7, call
    /// `JobFlow.new_uuid()`.
    #[new]
    #[pyo3(signature = (
        uuid,
        created_at,
        work_dir,
        tags=BTreeMap::new(),
        jobs=BTreeMap::new(),
    ))]
    fn new(
        uuid: String,
        created_at: chrono::DateTime<chrono::Utc>,
        work_dir: PathBuf,
        tags: BTreeMap<String, String>,
        jobs: BTreeMap<String, PyJob>,
    ) -> PyResult<Self> {
        let parsed_uuid = Uuid::parse_str(&uuid)
            .map_err(|e| PyValueError::new_err(format!("invalid UUID {uuid:?}: {e}")))?;
        let jobs = jobs
            .into_iter()
            .map(|(k, v)| (inner::JobId(k), v.0))
            .collect();
        Ok(Self(inner::JobFlow {
            uuid: parsed_uuid,
            created_at,
            work_dir,
            tags,
            jobs,
        }))
    }

    /// Generate a fresh UUID v7 helper to feed into `JobFlow(...)`.
    #[staticmethod]
    fn new_uuid() -> String {
        Uuid::now_v7().to_string()
    }

    #[getter]
    fn uuid(&self) -> String {
        self.0.uuid.to_string()
    }

    #[setter]
    fn set_uuid(&mut self, v: String) -> PyResult<()> {
        self.0.uuid =
            Uuid::parse_str(&v).map_err(|e| PyValueError::new_err(format!("invalid UUID: {e}")))?;
        Ok(())
    }

    #[getter]
    fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.0.created_at
    }

    #[setter]
    fn set_created_at(&mut self, v: chrono::DateTime<chrono::Utc>) {
        self.0.created_at = v;
    }

    #[getter]
    fn work_dir(&self) -> PathBuf {
        self.0.work_dir.clone()
    }

    #[setter]
    fn set_work_dir(&mut self, v: PathBuf) {
        self.0.work_dir = v;
    }

    #[getter]
    fn tags(&self) -> BTreeMap<String, String> {
        self.0.tags.clone()
    }

    #[setter]
    fn set_tags(&mut self, v: BTreeMap<String, String>) {
        self.0.tags = v;
    }

    /// Returns the jobs map keyed by `JobId.value` (str). To mutate, build a
    /// fresh dict and re-assign — list/dict mutation on the returned dict
    /// does not write back to the underlying Rust value.
    #[getter]
    fn jobs(&self) -> BTreeMap<String, PyJob> {
        self.0
            .jobs
            .iter()
            .map(|(k, v)| (k.0.clone(), PyJob(v.clone())))
            .collect()
    }

    #[setter]
    fn set_jobs(&mut self, v: BTreeMap<String, PyJob>) {
        self.0.jobs = v.into_iter().map(|(k, v)| (inner::JobId(k), v.0)).collect();
    }

    /// Convenience: insert a single job under the given `JobId`.
    fn insert_job(&mut self, id: PyJobId, job: PyJob) {
        self.0.jobs.insert(id.0, job.0);
    }

    /// Convenience: get a job by id (returns `None` if missing).
    fn get_job(&self, id: PyJobId) -> Option<PyJob> {
        self.0.jobs.get(&id.0).cloned().map(PyJob)
    }

    fn __repr__(&self) -> String {
        format!(
            "JobFlow(uuid={:?}, jobs={})",
            self.0.uuid.to_string(),
            self.0.jobs.len()
        )
    }
}

impl From<inner::JobFlow> for PyJobFlow {
    fn from(v: inner::JobFlow) -> Self {
        Self(v)
    }
}

impl From<PyJobFlow> for inner::JobFlow {
    fn from(v: PyJobFlow) -> Self {
        v.0
    }
}

#[pymodule(name = "workflow")]
pub(crate) mod inner_module {
    use super::*;

    const PYTHON_MODULE_NAME: &str =
        "gaussian_job_shared._gaussian_job_shared_core.entities.workflow";

    #[pymodule_export]
    use super::PyJobFlow;

    #[pymodule_export]
    use super::job::{PyCalcType, PyJob, PyJobEdge, PyJobId, PyJobSpec, PyProgram};

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
