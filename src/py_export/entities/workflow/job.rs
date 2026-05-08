//! PyO3 wrappers for `entities::workflow::{CalcType, JobId, Program, JobEdge, JobSpec, Job}`.
//! See `docs/superpowers/specs/2026-05-08-rust-python-ffi-design.md` §4.

use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::entities::workflow as inner;

use crate::py_export::entities::slurm::config::PySlurmJobConfig;
use crate::py_export::entities::slurm::dependency::PyDependencyType;

// ----------------------------------------------------------------- CalcType
#[gen_stub_pyclass]
#[pyclass(
    name = "CalcType",
    module = "gaussian_job_shared._core.entities.workflow",
    from_py_object,
    eq,
    ord,
    hash,
    frozen
)]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PyCalcType(pub inner::CalcType);

#[gen_stub_pymethods]
#[pymethods]
impl PyCalcType {
    #[new]
    fn new(value: String) -> Self {
        Self(inner::CalcType(value))
    }

    #[getter]
    fn value(&self) -> String {
        self.0.0.clone()
    }

    fn __str__(&self) -> String {
        self.0.0.clone()
    }

    fn __repr__(&self) -> String {
        format!("CalcType({:?})", self.0.0)
    }
}

impl From<inner::CalcType> for PyCalcType {
    fn from(v: inner::CalcType) -> Self {
        Self(v)
    }
}

impl From<PyCalcType> for inner::CalcType {
    fn from(v: PyCalcType) -> Self {
        v.0
    }
}

// -------------------------------------------------------------------- JobId
#[gen_stub_pyclass]
#[pyclass(
    name = "JobId",
    module = "gaussian_job_shared._core.entities.workflow",
    from_py_object,
    eq,
    ord,
    hash,
    frozen
)]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PyJobId(pub inner::JobId);

#[gen_stub_pymethods]
#[pymethods]
impl PyJobId {
    #[new]
    fn new(value: String) -> Self {
        Self(inner::JobId(value))
    }

    /// String value held by this `JobId`.
    #[getter]
    fn value(&self) -> String {
        self.0.0.clone()
    }

    fn __str__(&self) -> String {
        self.0.0.clone()
    }

    fn __repr__(&self) -> String {
        format!("JobId({:?})", self.0.0)
    }
}

impl From<inner::JobId> for PyJobId {
    fn from(v: inner::JobId) -> Self {
        Self(v)
    }
}

impl From<PyJobId> for inner::JobId {
    fn from(v: PyJobId) -> Self {
        v.0
    }
}

// ------------------------------------------------------------------ Program
#[gen_stub_pyclass]
#[pyclass(
    name = "Program",
    module = "gaussian_job_shared._core.entities.workflow",
    from_py_object,
    eq,
    ord,
    hash,
    frozen
)]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PyProgram(pub inner::Program);

#[gen_stub_pymethods]
#[pymethods]
impl PyProgram {
    #[new]
    fn new(value: String) -> Self {
        Self(inner::Program(value))
    }

    #[getter]
    fn value(&self) -> String {
        self.0.0.clone()
    }

    fn __str__(&self) -> String {
        self.0.0.clone()
    }

    fn __repr__(&self) -> String {
        format!("Program({:?})", self.0.0)
    }
}

impl From<inner::Program> for PyProgram {
    fn from(v: inner::Program) -> Self {
        Self(v)
    }
}

impl From<PyProgram> for inner::Program {
    fn from(v: PyProgram) -> Self {
        v.0
    }
}

// ----------------------------------------------------------------- JobEdge
#[gen_stub_pyclass]
#[pyclass(
    name = "JobEdge",
    module = "gaussian_job_shared._core.entities.workflow",
    from_py_object,
    eq
)]
#[derive(Clone, PartialEq, Eq)]
pub struct PyJobEdge(pub inner::JobEdge);

#[gen_stub_pymethods]
#[pymethods]
impl PyJobEdge {
    /// `from_` is spelled with a trailing underscore on the Python side
    /// because `from` is a reserved word.
    #[new]
    #[pyo3(signature = (from_, kind))]
    fn new(from_: PyJobId, kind: PyDependencyType) -> Self {
        Self(inner::JobEdge {
            from: from_.0,
            kind: kind.into(),
        })
    }

    #[getter(from_)]
    fn get_from(&self) -> PyJobId {
        PyJobId(self.0.from.clone())
    }

    #[setter(from_)]
    fn set_from(&mut self, v: PyJobId) {
        self.0.from = v.0;
    }

    #[getter]
    fn kind(&self) -> PyDependencyType {
        self.0.kind.into()
    }

    #[setter]
    fn set_kind(&mut self, v: PyDependencyType) {
        self.0.kind = v.into();
    }

    fn __repr__(&self) -> String {
        format!("JobEdge(from_={:?}, kind={:?})", self.0.from.0, self.0.kind)
    }
}

impl From<inner::JobEdge> for PyJobEdge {
    fn from(v: inner::JobEdge) -> Self {
        Self(v)
    }
}

impl From<PyJobEdge> for inner::JobEdge {
    fn from(v: PyJobEdge) -> Self {
        v.0
    }
}

// ----------------------------------------------------------------- JobSpec
#[gen_stub_pyclass]
#[pyclass(
    name = "JobSpec",
    module = "gaussian_job_shared._core.entities.workflow",
    from_py_object
)]
#[derive(Clone)]
pub struct PyJobSpec(pub inner::JobSpec);

#[gen_stub_pymethods]
#[pymethods]
impl PyJobSpec {
    #[new]
    #[pyo3(signature = (program, config, body))]
    fn new(program: PyProgram, config: PySlurmJobConfig, body: String) -> Self {
        Self(inner::JobSpec {
            program: program.0,
            config: config.0,
            body,
        })
    }

    #[getter]
    fn program(&self) -> PyProgram {
        PyProgram(self.0.program.clone())
    }

    #[setter]
    fn set_program(&mut self, v: PyProgram) {
        self.0.program = v.0;
    }

    #[getter]
    fn config(&self) -> PySlurmJobConfig {
        PySlurmJobConfig(self.0.config.clone())
    }

    #[setter]
    fn set_config(&mut self, v: PySlurmJobConfig) {
        self.0.config = v.0;
    }

    #[getter]
    fn body(&self) -> String {
        self.0.body.clone()
    }

    #[setter]
    fn set_body(&mut self, v: String) {
        self.0.body = v;
    }

    fn __repr__(&self) -> String {
        format!(
            "JobSpec(program={:?}, body={:?})",
            self.0.program.0, self.0.body
        )
    }
}

impl From<inner::JobSpec> for PyJobSpec {
    fn from(v: inner::JobSpec) -> Self {
        Self(v)
    }
}

impl From<PyJobSpec> for inner::JobSpec {
    fn from(v: PyJobSpec) -> Self {
        v.0
    }
}

// --------------------------------------------------------------------- Job
#[gen_stub_pyclass]
#[pyclass(
    name = "Job",
    module = "gaussian_job_shared._core.entities.workflow",
    from_py_object
)]
#[derive(Clone)]
pub struct PyJob(pub inner::Job);

#[gen_stub_pymethods]
#[pymethods]
impl PyJob {
    #[new]
    #[pyo3(signature = (spec, parents=Vec::new()))]
    fn new(spec: PyJobSpec, parents: Vec<PyJobEdge>) -> Self {
        Self(inner::Job {
            spec: spec.0,
            parents: parents.into_iter().map(|e| e.0).collect(),
        })
    }

    #[getter]
    fn spec(&self) -> PyJobSpec {
        PyJobSpec(self.0.spec.clone())
    }

    #[setter]
    fn set_spec(&mut self, v: PyJobSpec) {
        self.0.spec = v.0;
    }

    /// Returns a *copy* of the parent edges. To mutate, re-assign the
    /// whole list: `job.parents = job.parents + [edge]`.
    #[getter]
    fn parents(&self) -> Vec<PyJobEdge> {
        self.0.parents.iter().cloned().map(PyJobEdge).collect()
    }

    #[setter]
    fn set_parents(&mut self, v: Vec<PyJobEdge>) {
        self.0.parents = v.into_iter().map(|e| e.0).collect();
    }

    fn __repr__(&self) -> String {
        format!(
            "Job(program={:?}, parents={})",
            self.0.spec.program.0,
            self.0.parents.len()
        )
    }
}

impl From<inner::Job> for PyJob {
    fn from(v: inner::Job) -> Self {
        Self(v)
    }
}

impl From<PyJob> for inner::Job {
    fn from(v: PyJob) -> Self {
        v.0
    }
}
