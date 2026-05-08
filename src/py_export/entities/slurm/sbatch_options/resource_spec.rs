//! PyO3 wrappers for `entities::slurm::resource_spec::*`.

use std::num::NonZeroU32;

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyclass_enum, gen_stub_pymethods};

use crate::entities::slurm as inner;

// --------------------------------------------------------------- MemoryUnit
#[gen_stub_pyclass_enum]
#[pyclass(
    name = "MemoryUnit",
    module = "gaussian_job_shared._core.entities.slurm.sbatch_options",
    from_py_object,
    eq,
    eq_int,
    hash,
    frozen
)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PyMemoryUnit {
    Kilo,
    Mega,
    Giga,
    Tera,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyMemoryUnit {
    fn __str__(&self) -> &'static str {
        match self {
            Self::Kilo => "K",
            Self::Mega => "M",
            Self::Giga => "G",
            Self::Tera => "T",
        }
    }

    fn __repr__(&self) -> String {
        format!("MemoryUnit.{:?}", self)
    }
}

impl From<inner::MemoryUnit> for PyMemoryUnit {
    fn from(v: inner::MemoryUnit) -> Self {
        match v {
            inner::MemoryUnit::Kilo => Self::Kilo,
            inner::MemoryUnit::Mega => Self::Mega,
            inner::MemoryUnit::Giga => Self::Giga,
            inner::MemoryUnit::Tera => Self::Tera,
        }
    }
}

impl From<PyMemoryUnit> for inner::MemoryUnit {
    fn from(v: PyMemoryUnit) -> Self {
        match v {
            PyMemoryUnit::Kilo => Self::Kilo,
            PyMemoryUnit::Mega => Self::Mega,
            PyMemoryUnit::Giga => Self::Giga,
            PyMemoryUnit::Tera => Self::Tera,
        }
    }
}

// ------------------------------------------------------------------- Memory
#[gen_stub_pyclass]
#[pyclass(
    name = "Memory",
    module = "gaussian_job_shared._core.entities.slurm.sbatch_options",
    from_py_object,
    eq,
    hash,
    frozen
)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PyMemory(pub inner::Memory);

#[gen_stub_pymethods]
#[pymethods]
impl PyMemory {
    /// Parse a Slurm memory token, e.g. `"8G"` (or unit-less `"1024"` =
    /// mebibytes per Slurm `--mem` default).
    #[new]
    fn new(s: &str) -> PyResult<Self> {
        s.parse::<inner::Memory>().map(Self).map_err(Into::into)
    }

    #[staticmethod]
    fn parse(s: &str) -> PyResult<Self> {
        Self::new(s)
    }

    #[staticmethod]
    fn from_value(value: u32, unit: PyMemoryUnit) -> PyResult<Self> {
        let value = NonZeroU32::new(value)
            .ok_or_else(|| PyValueError::new_err("memory value must be > 0"))?;
        Ok(Self(inner::Memory {
            value,
            unit: unit.into(),
        }))
    }

    #[getter]
    fn value(&self) -> u32 {
        self.0.value.get()
    }

    #[getter]
    fn unit(&self) -> PyMemoryUnit {
        self.0.unit.into()
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __repr__(&self) -> String {
        format!("Memory({:?})", self.0.to_string())
    }
}

impl From<inner::Memory> for PyMemory {
    fn from(v: inner::Memory) -> Self {
        Self(v)
    }
}

impl From<PyMemory> for inner::Memory {
    fn from(v: PyMemory) -> Self {
        v.0
    }
}

// ------------------------------------------------------------ ResourceSpecCPU
#[gen_stub_pyclass]
#[pyclass(
    name = "ResourceSpecCPU",
    module = "gaussian_job_shared._core.entities.slurm.sbatch_options",
    from_py_object,
    eq,
    frozen
)]
#[derive(Clone, PartialEq, Eq)]
pub struct PyResourceSpecCPU(pub inner::ResourceSpecCPU);

#[gen_stub_pymethods]
#[pymethods]
impl PyResourceSpecCPU {
    #[new]
    fn new(p: u32, t: u32, c: u32, m: PyMemory) -> PyResult<Self> {
        let p = NonZeroU32::new(p).ok_or_else(|| PyValueError::new_err("p must be > 0"))?;
        let t = NonZeroU32::new(t).ok_or_else(|| PyValueError::new_err("t must be > 0"))?;
        let c = NonZeroU32::new(c).ok_or_else(|| PyValueError::new_err("c must be > 0"))?;
        Ok(Self(inner::ResourceSpecCPU { p, t, c, m: m.0 }))
    }

    #[getter]
    fn p(&self) -> u32 {
        self.0.p.get()
    }

    #[getter]
    fn t(&self) -> u32 {
        self.0.t.get()
    }

    #[getter]
    fn c(&self) -> u32 {
        self.0.c.get()
    }

    #[getter]
    fn m(&self) -> PyMemory {
        PyMemory(self.0.m)
    }

    fn __repr__(&self) -> String {
        format!(
            "ResourceSpecCPU(p={}, t={}, c={}, m={:?})",
            self.0.p.get(),
            self.0.t.get(),
            self.0.c.get(),
            self.0.m.to_string()
        )
    }
}

impl From<inner::ResourceSpecCPU> for PyResourceSpecCPU {
    fn from(v: inner::ResourceSpecCPU) -> Self {
        Self(v)
    }
}

impl From<PyResourceSpecCPU> for inner::ResourceSpecCPU {
    fn from(v: PyResourceSpecCPU) -> Self {
        v.0
    }
}

// ------------------------------------------------------------ ResourceSpecGPU
#[gen_stub_pyclass]
#[pyclass(
    name = "ResourceSpecGPU",
    module = "gaussian_job_shared._core.entities.slurm.sbatch_options",
    from_py_object,
    eq,
    frozen
)]
#[derive(Clone, PartialEq, Eq)]
pub struct PyResourceSpecGPU(pub inner::ResourceSpecGPU);

#[gen_stub_pymethods]
#[pymethods]
impl PyResourceSpecGPU {
    #[new]
    fn new(g: u32) -> PyResult<Self> {
        let g = NonZeroU32::new(g).ok_or_else(|| PyValueError::new_err("g must be > 0"))?;
        Ok(Self(inner::ResourceSpecGPU { g }))
    }

    #[getter]
    fn g(&self) -> u32 {
        self.0.g.get()
    }

    fn __repr__(&self) -> String {
        format!("ResourceSpecGPU(g={})", self.0.g.get())
    }
}

impl From<inner::ResourceSpecGPU> for PyResourceSpecGPU {
    fn from(v: inner::ResourceSpecGPU) -> Self {
        Self(v)
    }
}

impl From<PyResourceSpecGPU> for inner::ResourceSpecGPU {
    fn from(v: PyResourceSpecGPU) -> Self {
        v.0
    }
}

// --------------------------------------------------------------- ResourceSpec
#[gen_stub_pyclass]
#[pyclass(
    name = "ResourceSpec",
    module = "gaussian_job_shared._core.entities.slurm.sbatch_options",
    from_py_object,
    eq,
    frozen
)]
#[derive(Clone, PartialEq, Eq)]
pub struct PyResourceSpec(pub inner::ResourceSpec);

#[gen_stub_pymethods]
#[pymethods]
impl PyResourceSpec {
    /// Parse a Slurm `--rsc` spec, e.g. `"p=4:t=8:c=8:m=8G"` or `"g=1"`.
    #[new]
    fn new(s: &str) -> PyResult<Self> {
        s.parse::<inner::ResourceSpec>()
            .map(Self)
            .map_err(Into::into)
    }

    #[staticmethod]
    fn parse(s: &str) -> PyResult<Self> {
        Self::new(s)
    }

    #[staticmethod]
    fn cpu(spec: PyResourceSpecCPU) -> Self {
        Self(inner::ResourceSpec::CPU(spec.0))
    }

    #[staticmethod]
    fn gpu(spec: PyResourceSpecGPU) -> Self {
        Self(inner::ResourceSpec::GPU(spec.0))
    }

    /// `"cpu"` or `"gpu"`.
    #[getter]
    fn kind(&self) -> &'static str {
        match self.0 {
            inner::ResourceSpec::CPU(_) => "cpu",
            inner::ResourceSpec::GPU(_) => "gpu",
        }
    }

    #[getter]
    fn cpu_spec(&self) -> Option<PyResourceSpecCPU> {
        match &self.0 {
            inner::ResourceSpec::CPU(c) => Some(PyResourceSpecCPU(c.clone())),
            _ => None,
        }
    }

    #[getter]
    fn gpu_spec(&self) -> Option<PyResourceSpecGPU> {
        match &self.0 {
            inner::ResourceSpec::GPU(g) => Some(PyResourceSpecGPU(g.clone())),
            _ => None,
        }
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __repr__(&self) -> String {
        format!("ResourceSpec({:?})", self.0.to_string())
    }
}

impl From<inner::ResourceSpec> for PyResourceSpec {
    fn from(v: inner::ResourceSpec) -> Self {
        Self(v)
    }
}

impl From<PyResourceSpec> for inner::ResourceSpec {
    fn from(v: PyResourceSpec) -> Self {
        v.0
    }
}
