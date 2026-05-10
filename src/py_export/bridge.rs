//! Duck-typed `FromPyObject` bridges for SAR's slurm vocab types.
//!
//! Architecture rule (Pyclass Single Owner — see SAR's spec doc):
//! shared2's cdylib MUST NOT link SAR's pyclass impls. Instead, where a
//! shared2 pyclass `__new__` or setter accepts a slurm vocab type as an
//! argument, we use a `#[repr(transparent)]` newtype that implements
//! `FromPyObject` by reading attributes off the Python object — i.e.
//! duck typing.
//!
//! Each bridge unwraps to its inner SAR Rust type via `.0`. They are NOT
//! `#[pyclass]` themselves — they only exist to teach pyo3 how to extract
//! a SAR-owned Python object into a SAR Rust struct without taking a
//! compile-time dependency on SAR's pyclass tree.
//!
//! Each bridge also implements `PyStubType` so `pyo3-stub-gen` knows
//! which Python type name to write into the generated `.pyi` files —
//! the names match SAR's exposed pyclasses
//! (`slurm_async_runner._slurm_async_runner_core.entities.slurm.sbatch_options.X`).
//!
//! Companion: for return paths (shared2 needs to hand a SAR slurm value
//! back to Python), the pyclass site uses `Py::import` to fetch SAR's
//! canonical Python class at runtime and instantiate it — see
//! `entities/workflow/job.rs`.

use std::num::NonZeroU32;
use std::str::FromStr;

use pyo3::Borrowed;
use pyo3::intern;
use pyo3::prelude::*;
use pyo3::types::PyAny;
use pyo3_stub_gen::{PyStubType, TypeInfo};
use slurm_async_runner::entities::slurm::{
    DependencyType, JobTimeLimit, Memory, MemoryUnit, ResourceSpec, SlurmJobConfig,
};

/// Python module path of SAR's slurm sbatch_options pyclasses.
///
/// Shared with call sites that use `Py::import` to fetch SAR's canonical
/// types at runtime (return paths) — keeping the path in one constant
/// avoids drift between the bridge stubs (which write this name into
/// generated `.pyi` files) and the Rust getters (which import it at
/// runtime).
pub const SAR_SBATCH_OPTIONS_MODULE: &str =
    "slurm_async_runner._slurm_async_runner_core.entities.slurm.sbatch_options";

/// Helper: build a `TypeInfo` pointing at one of SAR's pyclasses in the
/// `sbatch_options` submodule, so generated stubs reference the SAR-owned
/// type rather than a shadow class in shared2.
fn sar_sbatch_type(class: &str) -> TypeInfo {
    TypeInfo::with_module(
        &format!("{SAR_SBATCH_OPTIONS_MODULE}.{class}"),
        SAR_SBATCH_OPTIONS_MODULE.into(),
    )
}

// ---------------------------------------------------------------- DependencyType
/// Bridge over SAR's `DependencyType` enum (variants: After, AfterAny,
/// AfterBurstBuffer, AfterCorr, AfterNotOk, AfterOk, Singleton).
///
/// Extracts via `str(obj)` — SAR's `PyDependencyType.__str__` returns
/// the Slurm keyword (`"afterok"`, `"after"`, ...), which `DependencyType`'s
/// `FromStr` impl recognises.
#[repr(transparent)]
pub struct DependencyTypeBridge(pub DependencyType);

impl<'py> FromPyObject<'_, 'py> for DependencyTypeBridge {
    type Error = PyErr;

    fn extract(ob: Borrowed<'_, 'py, PyAny>) -> PyResult<Self> {
        let s: String = ob.str()?.extract()?;
        DependencyType::from_str(&s)
            .map(Self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }
}

impl PyStubType for DependencyTypeBridge {
    fn type_output() -> TypeInfo {
        sar_sbatch_type("DependencyType")
    }
}

// --------------------------------------------------------------------- Memory
/// Bridge over SAR's `Memory { value: NonZeroU32, unit: MemoryUnit }`.
///
/// Reads `value` as `u32` (rejecting 0) and `unit` via `str(obj.unit)` —
/// SAR's `PyMemoryUnit.__str__` returns the unit name; we also accept the
/// single-letter Slurm form for liberal input handling.
#[repr(transparent)]
pub struct MemoryBridge(pub Memory);

impl<'py> FromPyObject<'_, 'py> for MemoryBridge {
    type Error = PyErr;

    fn extract(ob: Borrowed<'_, 'py, PyAny>) -> PyResult<Self> {
        let py = ob.py();
        let raw_value: u32 = ob.getattr(intern!(py, "value"))?.extract()?;
        // SAR's `Memory.value: NonZeroU32` invariant should prevent 0 from
        // ever reaching here; this NonZeroU32::new check is defensive only,
        // for the case where a non-SAR Python object duck-types `.value`/`.unit`.
        let value = NonZeroU32::new(raw_value).ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("memory value must be positive (non-zero)")
        })?;
        let unit_str: String = ob.getattr(intern!(py, "unit"))?.str()?.extract()?;
        // Accept both single-letter (Display) and full-word (enum name)
        // forms so callers can be liberal in what they pass.
        let unit = match unit_str.as_str() {
            "K" | "Kilo" | "KiB" => MemoryUnit::Kilo,
            "M" | "Mega" | "MiB" => MemoryUnit::Mega,
            "G" | "Giga" | "GiB" => MemoryUnit::Giga,
            "T" | "Tera" | "TiB" => MemoryUnit::Tera,
            other => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "unrecognized memory unit {other}"
                )));
            }
        };
        Ok(Self(Memory { value, unit }))
    }
}

impl PyStubType for MemoryBridge {
    fn type_output() -> TypeInfo {
        sar_sbatch_type("Memory")
    }
}

// ---------------------------------------------------------------- JobTimeLimit
/// Bridge over SAR's `JobTimeLimit`. Extracts via `str(obj)` and routes
/// through `JobTimeLimit::from_str` (which accepts any of Slurm's six
/// surface forms).
#[repr(transparent)]
pub struct JobTimeLimitBridge(pub JobTimeLimit);

impl<'py> FromPyObject<'_, 'py> for JobTimeLimitBridge {
    type Error = PyErr;

    fn extract(ob: Borrowed<'_, 'py, PyAny>) -> PyResult<Self> {
        let s: String = ob.str()?.extract()?;
        JobTimeLimit::from_str(&s)
            .map(Self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }
}

impl PyStubType for JobTimeLimitBridge {
    fn type_output() -> TypeInfo {
        sar_sbatch_type("JobTimeLimit")
    }
}

// ---------------------------------------------------------------- ResourceSpec
/// Bridge over SAR's `ResourceSpec`. Reads the public Python-facing API
/// (`kind` plus `cpu_spec` / `gpu_spec`) and routes through
/// `ResourceSpec::from_parts` (the pure-Rust validator).
///
/// Shape of SAR's `PyResourceSpec`:
///   - `kind: str` — `"cpu"` or `"gpu"`.
///   - `cpu_spec: Optional[ResourceSpecCPU]` with fields `p`, `t`, `c`, `m`.
///   - `gpu_spec: Optional[ResourceSpecGPU]` with field `g`.
///
/// The `processes`/`threads`/`cores`/`memory`/`gpus` names in SAR's
/// `PyResourceSpec.__new__` are constructor keyword arguments only —
/// they are NOT exposed as attributes on instances. Reading them off an
/// instance raises `AttributeError`, which is the original bug this
/// bridge fixes.
#[repr(transparent)]
pub struct ResourceSpecBridge(pub ResourceSpec);

impl<'py> FromPyObject<'_, 'py> for ResourceSpecBridge {
    type Error = PyErr;

    fn extract(ob: Borrowed<'_, 'py, PyAny>) -> PyResult<Self> {
        let py = ob.py();
        let kind: String = ob.getattr(intern!(py, "kind"))?.extract()?;
        match kind.as_str() {
            "cpu" => {
                let cpu_any = ob.getattr(intern!(py, "cpu_spec"))?;
                if cpu_any.is_none() {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "ResourceSpec.kind=='cpu' but cpu_spec is None",
                    ));
                }
                let p: Option<u32> = cpu_any.getattr(intern!(py, "p"))?.extract()?;
                let t: Option<u32> = cpu_any.getattr(intern!(py, "t"))?.extract()?;
                let c: Option<u32> = cpu_any.getattr(intern!(py, "c"))?.extract()?;
                let m_any = cpu_any.getattr(intern!(py, "m"))?;
                let memory = if m_any.is_none() {
                    None
                } else {
                    // Bound::extract dispatches FromPyObject::extract via Borrowed.
                    Some(m_any.extract::<MemoryBridge>()?.0)
                };
                ResourceSpec::from_parts(p, t, c, memory, None)
                    .map(Self)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
            }
            "gpu" => {
                let gpu_any = ob.getattr(intern!(py, "gpu_spec"))?;
                if gpu_any.is_none() {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "ResourceSpec.kind=='gpu' but gpu_spec is None",
                    ));
                }
                let g: u32 = gpu_any.getattr(intern!(py, "g"))?.extract()?;
                ResourceSpec::from_parts(None, None, None, None, Some(g))
                    .map(Self)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
            }
            other => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "unknown ResourceSpec.kind: {other:?} (expected \"cpu\" or \"gpu\")"
            ))),
        }
    }
}

impl PyStubType for ResourceSpecBridge {
    fn type_output() -> TypeInfo {
        sar_sbatch_type("ResourceSpec")
    }
}

// --------------------------------------------------------------- SlurmJobConfig
/// Bridge over SAR's `SlurmJobConfig`. Pulls each field by attribute name
/// off the Python object, recursing through nested bridges
/// (`JobTimeLimitBridge`, `ResourceSpecBridge`) where needed.
///
/// `SlurmJobConfig` has no `new(...)` constructor — we build the struct
/// literal directly and must populate every field. Where a nested field
/// type is a SAR pyclass (e.g. `array_spec: Option<SlurmArraySpec>`) and
/// shared2 cannot link it, we reject non-`None` values with a clear
/// `NotImplementedError` so callers see the limitation.
#[repr(transparent)]
pub struct SlurmJobConfigBridge(pub SlurmJobConfig);

impl<'py> FromPyObject<'_, 'py> for SlurmJobConfigBridge {
    type Error = PyErr;

    fn extract(ob: Borrowed<'_, 'py, PyAny>) -> PyResult<Self> {
        let py = ob.py();

        // partition is a JobPartition = String alias — extract directly.
        let partition: String = ob.getattr(intern!(py, "partition"))?.extract()?;

        let time_limit_any = ob.getattr(intern!(py, "time_limit"))?;
        let time_limit = if time_limit_any.is_none() {
            None
        } else {
            Some(time_limit_any.extract::<JobTimeLimitBridge>()?.0)
        };

        let resource_spec_any = ob.getattr(intern!(py, "resource_spec"))?;
        let resource_spec = if resource_spec_any.is_none() {
            None
        } else {
            Some(resource_spec_any.extract::<ResourceSpecBridge>()?.0)
        };

        // Plain-string / PathBuf fields can pyo3-extract directly.
        let log_stdout: Option<std::path::PathBuf> =
            ob.getattr(intern!(py, "log_stdout"))?.extract()?;
        let log_stderr: Option<std::path::PathBuf> =
            ob.getattr(intern!(py, "log_stderr"))?.extract()?;
        let comment: Option<String> = ob.getattr(intern!(py, "comment"))?.extract()?;
        let job_name: Option<String> = ob.getattr(intern!(py, "job_name"))?.extract()?;
        let mail_user: Option<String> = ob.getattr(intern!(py, "mail_user"))?.extract()?;

        // The remaining fields (array_spec, dependency, mail_types) are
        // SAR pyclasses with no plain-data equivalent shared2 can extract
        // without linking SAR's pyclass tree. shared2's workflow pyclasses
        // do not currently pass these through, so we reject any non-None
        // value with a clear error.
        let array_spec_any = ob.getattr(intern!(py, "array_spec"))?;
        if !array_spec_any.is_none() {
            return Err(pyo3::exceptions::PyNotImplementedError::new_err(
                "SlurmJobConfig.array_spec passthrough is not yet implemented in the shared2 \
                 bridge — set it via SAR-side construction instead",
            ));
        }
        let dependency_any = ob.getattr(intern!(py, "dependency"))?;
        if !dependency_any.is_none() {
            return Err(pyo3::exceptions::PyNotImplementedError::new_err(
                "SlurmJobConfig.dependency passthrough is not yet implemented in the shared2 \
                 bridge — set it via SAR-side construction instead",
            ));
        }
        let mail_types_any = ob.getattr(intern!(py, "mail_types"))?;
        if !mail_types_any.is_none() {
            return Err(pyo3::exceptions::PyNotImplementedError::new_err(
                "SlurmJobConfig.mail_types passthrough is not yet implemented in the shared2 \
                 bridge — set it via SAR-side construction instead",
            ));
        }

        Ok(Self(SlurmJobConfig {
            partition,
            time_limit,
            log_stdout,
            log_stderr,
            comment,
            job_name,
            array_spec: None,
            dependency: None,
            mail_user,
            mail_types: None,
            resource_spec,
        }))
    }
}

impl PyStubType for SlurmJobConfigBridge {
    fn type_output() -> TypeInfo {
        sar_sbatch_type("SlurmJobConfig")
    }
}
