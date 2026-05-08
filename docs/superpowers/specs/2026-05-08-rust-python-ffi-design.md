# Expose Rust Entities to Python — PyO3 FFI Layer

**Date:** 2026-05-08
**Branch:** `rust-python-ffi`
**Crate:** `gaussian_job_shared`
**Companion spec:** [`2026-05-08-slurm-job-flow-structs-design.md`](./2026-05-08-slurm-job-flow-structs-design.md) (§10 lists pyo3 bindings as out-of-scope follow-up — this spec _is_ that follow-up)

## 1. Goal

Expose every public type in `crate::entities::*` (and the relevant error
type from `crate::error`) to Python through PyO3, accessible as
`gaussian_job_shared._core.<TypeName>`. Python users must be able to:

- Construct each value from Python (positional/keyword args mirroring the
  Rust struct fields)
- Read every field via attribute access
- Compare values for equality (and hash where the Rust type derives `Hash`)
- Round-trip through `str(x)` / `Type.parse(s)` for string-encoded Slurm
  surface types (`JobTimeLimit`, `SlurmDependency`, `SlurmArraySpec`,
  `ResourceSpec`, `Memory`)
- Move values back into Rust via `From` / `Into` so future Rust-side
  helpers can accept Python-built objects

Stub generation (`.pyi`) must remain functional; downstream Python
consumers rely on `pyo3-stub-gen` for IDE completion.

## 2. Non-Goals

- TOML round-trip helpers (file IO is owned by TaskManager)
- Validation logic (`UnknownParent`, cycle detection, …) — out of scope
  per the companion spec, applies here too
- Async submission / runtime mapping (TaskManager territory)
- Modifying `crate::entities` to derive `pyo3::PyClass` directly. The
  entity layer stays pyo3-free; all bindings live under `src/py_export/`.

## 3. Architecture

### 3.1 Layering

`entities/` (pure data) ← unchanged ↑
`py_export/entities/` (pyo3 wrapper) ← new
`py_export/mod.rs` (`#[pymodule]` registration) ← updated

Each Python-facing type is a **wrapper** struct that holds one inner
Rust value:

```rust
#[pyclass(name = "JobId", module = "gaussian_job_shared._core",
          eq, ord, hash, frozen)]
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PyJobId(pub crate::entities::slurm::JobId);

impl From<crate::entities::slurm::JobId> for PyJobId { … }
impl From<PyJobId> for crate::entities::slurm::JobId { … }
```

This pattern keeps `entities/` free of `pyo3` dependencies (matches the
existing `py_export/error.rs` convention) and gives full control over
the Python API shape (e.g. `parents` returns `list[JobEdge]` instead of
forcing a custom Rust collection wrapper).

### 3.2 Module layout (additive)

```
src/py_export/
├── mod.rs                     ← updated: registers every #[pyclass]
├── error.rs                   ← updated: add SLURMJOBError → PyErr
└── entities/                  ← NEW
    ├── mod.rs                 ← `pub use` re-exports
    ├── job_flow.rs            ← PyJobFlow, PyCalcType
    └── slurm/
        ├── mod.rs
        ├── job.rs             ← PyJob, PyJobSpec, PyJobEdge, PyJobId, PyProgram
        ├── status.rs          ← PyJobLifecycleStatus, PyStatusEntry
        ├── config.rs          ← PySlurmJobConfig, PyMailType, PyMailTypeInput
        ├── dependency.rs      ← PyDependencyType, PyDependencyJoin,
        │                        PyDependencyJobRef, PyDependencyClause,
        │                        PySlurmDependency
        ├── array_spec.rs      ← PyArrayIndex, PySlurmArraySpec
        ├── resource_spec.rs   ← PyMemoryUnit, PyMemory, PyResourceSpec,
        │                        PyResourceSpecCPU, PyResourceSpecGPU
        └── time_limit.rs      ← PyJobTimeLimit
```

## 4. Wrapping Strategy by Category

| Category | Types | Python-facing shape |
|---|---|---|
| **String newtypes** | `JobId`, `Program`, `CalcType` | `class JobId(s: str)` — frozen, eq+ord+hash, `value: str` getter, `__str__`, `__repr__` |
| **C-like enums** (closed, no payload) | `DependencyType`, `JobLifecycleStatus`, `MailType`, `MemoryUnit`, `DependencyJoin` | `pyclass enum` (variants visible as `JobLifecycleStatus.Queued` etc.), eq, hash, `__str__` returning the canonical lowercase form when one exists |
| **Stringly-encoded** | `JobTimeLimit`, `SlurmDependency`, `SlurmArraySpec`, `ResourceSpec`, `Memory` | `class T(s: str)` — `__new__` parses, `__str__` returns canonical form, key components exposed as read-only properties (e.g. `JobTimeLimit.total_seconds`, `SlurmDependency.clauses`, `ResourceSpec.cpu` / `.gpu`) |
| **Sum-type with payloads** | `ArrayIndex` (`Single`/`Range`/`Stepped`), `ResourceSpec` (`CPU`/`GPU`) | Constructor classmethods (`ArrayIndex.single(0)`, `ArrayIndex.range(0, 15)`, `ArrayIndex.stepped(0, 15, 4)`) + discriminant getters (`.kind` returning a `pyclass enum` discriminant; payload accessors return `Optional` of payload tuple/struct) |
| **Plain compound** | `JobEdge`, `StatusEntry`, `JobSpec`, `Job`, `SlurmJobConfig`, `JobFlow`, `DependencyClause`, `DependencyJobRef`, `ResourceSpecCPU`, `ResourceSpecGPU`, `MailTypeInput` | `#[pyclass]` with keyword `#[new]`, individual getters/setters per field, `__repr__`, `__eq__` |
| **Error** | `SLURMJOBError` | `From<SLURMJOBError> for PyErr` mapping to `PyRuntimeError` |

### 4.1 Property typing

Inside Python:

- `JobFlow.jobs: dict[JobId, Job]` (pyo3 maps `BTreeMap` → `dict`)
- `Job.parents: list[JobEdge]`
- `JobFlow.tags: dict[str, str]`
- `JobFlow.uuid: str` (UUID is exposed as its canonical hyphenated string
  to avoid pulling `uuid` into the Python type surface)
- `JobFlow.created_at: datetime` (pyo3 `chrono` feature converts
  automatically)
- `SlurmJobConfig.time_limit: JobTimeLimit | None`, etc.

### 4.2 Setter strategy

Compound types (`Job`, `JobFlow`, `SlurmJobConfig`, …) expose **mutable**
properties so Python users can build values incrementally:

```python
spec = JobSpec(program=Program("g16"), config=cfg, body="…")
job = Job(spec=spec, parents=[])
job.parents.append(JobEdge(from_=JobId("g16"), kind=DependencyType.AfterOk))
```

Caveat: Python attribute writes go through pyo3 setters that **replace**
the inner Rust value. List/dict mutation on a returned getter (e.g.
`job.parents.append(...)`) operates on a clone — to persist a change,
re-assign the property: `job.parents = job.parents + [edge]`. This is
documented in each setter docstring.

The string newtypes (`JobId`, `Program`, `CalcType`) and enums are
declared `frozen` because they are used as map keys.

## 5. Stub Generation

Every `#[pyclass]` and `#[pymethods]` block carries the matching
`#[gen_stub_pyclass]` / `#[gen_stub_pymethods]` / `#[gen_stub_pyclass_enum]`
attribute from `pyo3_stub_gen::derive`, so the existing `stub_gen` binary
keeps regenerating `python/gaussian_job_shared/_core/__init__.pyi`.

## 6. Module Registration

`src/py_export/mod.rs` adds one `#[pymodule_export] use crate::py_export::entities::…::PyXxx;`
line per type. The Python-visible name (set via `#[pyclass(name = "Xxx")]`)
omits the `Py` prefix.

## 7. Testing

- **Rust side**: keep existing entity unit tests untouched. Add
  conversion smoke tests in each `py_export/entities/...` file
  (round-trip `Rust → Py → Rust` equality).
- **Python side**: extend `python/tests/test_all.py` with constructor,
  equality, `str()` round-trip, and a small `JobFlow` build covering
  the g16 → post pair from the companion spec.
- Stub regeneration: run `cargo run --bin stub_gen --features stub_gen`
  and commit the regenerated `.pyi`.

## 8. Out of Scope (Future Work)

- TOML / JSON helpers (`Type.from_toml(s)` / `to_toml()`) — relies on a
  decision about whether Python should re-use Rust's serde or use
  `pythonize` to surface plain `dict`s. Defer until a real consumer
  requests one.
- Async APIs (sbatch/squeue) — these belong in a separate runtime crate.
- TaskManager-side runtime fields (`slurm_jobid`, `status_history`) —
  the wrappers will be extended once the upstream Rust types gain those
  fields.
- Validation methods on `JobFlow` — out of scope, same reason as the
  companion spec.

## 9. Risks / Notes

- **`pythonize` is enabled** (Cargo.toml line 67) but **not used in this
  PR**. Listed for future use only.
- **Stub-gen is sensitive to attribute order**: putting
  `#[gen_stub_pyclass]` *above* `#[pyclass]` is required.
- **`SlurmJobConfig` is large** (12 fields) — accept all-keyword `#[new]`
  with `Option<…>` for everything except `partition`.
- **`ResourceSpec` is a sum type**: Python class wraps a discriminant +
  payload union; safer to expose `ResourceSpec.cpu_spec()` and
  `ResourceSpec.gpu_spec()` returning `Optional` than to multiplex via a
  single field.
- **`Memory.unit` is an enum**; round-trip via `str(memory)` is the
  primary surface, struct fields are secondary.
