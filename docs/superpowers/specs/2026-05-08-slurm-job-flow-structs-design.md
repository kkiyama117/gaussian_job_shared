# Slurm Job and JobFlow Struct Design

**Date:** 2026-05-08
**Branch:** `slurm-job-structs`
**Crate:** `gaussian_job_shared` (Rust portion of `gaussian-job-shared2`)

## 1. Goal

Add data types that represent **a single Slurm job** and **a single-flow set of Slurm jobs** (the chain of batches that constitute one logical calculation unit) to the `gaussian_job_shared` Rust crate.

The data model must:

- Stay **program-agnostic** at the top level (Gaussian-specific data lives in a separate `gaussian16` module).
- Reuse the existing `SlurmJobConfig` (and its components: `SlurmArraySpec`, `SlurmDependency`, `ResourceSpec`, `JobTimeLimit`) without modification.
- Express **intra-flow batch dependencies** as a DAG so that fork/join topologies are representable in the future.
- Be **send-side only**: no submission ID (`slurm_jobid`, `post_jobid`), no submit/tick logic, no override-merging logic. Those concerns belong to a future `TaskManager` layer.

## 2. Non-Goals

The following are explicitly **out of scope** of this PR:

- TaskManager / submit / tick logic
- Storing post-submission identifiers (`slurm_jobid`, `post_jobid`)
- Status transition logic (Python's `_decide_transition` equivalent)
- Program-specific params and compounds — owned by `gaussian16` module
- pyo3 bindings for the new types
- TOML serialization round-trip helpers (`read_metadata` / `write_metadata` equivalents)
- Cross-flow DAG operations (sweep expansion, parent resolution) — Python δ-layer concerns

## 3. Reference: Mapping to `gaussian-experiment-manager` (Python δ layer)

| Python (`gaussian-experiment-manager` + `gaussian-job-shared`) | This Rust design                                         |
|----------------------------------------------------------------|----------------------------------------------------------|
| `CalcBlock` (uuid, program, calc_type, parent_uuids, ...)      | `JobFlow` (identity + lineage + tags + batches)          |
| `Compounds`                                                    | Out of scope — `gaussian16::Compounds` (program-side)    |
| `CalcParams` / `GaussianParams`                                | Out of scope — `gaussian16::JobParams` (program-side)    |
| `CalcBlock.slurm_jobid` / `post_jobid`                         | Out of scope — runtime IDs not modelled here             |
| `PlannedStep`                                                  | `JobFlow` + `Vec<BatchJob>` collectively                 |
| Implicit `(g16, post)` pair                                    | `BatchJob` × 2 with one `BatchEdge` (post → main, Afterok) |
| `_StepSubmitter.submit_step`                                   | Out of scope — TaskManager responsibility                |
| `Status` (queued/running/done/failed)                          | `JobLifecycleStatus` (independent type, not embedded)    |
| `StatusEntry` (status + transitioned_at)                       | `StatusEntry` (matches Python's shape)                   |
| `SlurmJobState` (PENDING/RUNNING/...)                          | Out of scope — already exists in `slurm-async-runner`    |
| `TickResult`, `SubmitResult`, `SbatchError`                    | Out of scope — TaskManager responsibility                |

## 4. Architecture

```
[ JobFlow ]                  ── 1 logical job-flow unit (uuid + batches DAG)
   ├── identity:
   │     uuid / program / calc_type / created_at
   ├── lineage (cross-flow):
   │     parent_uuids / experiment_id
   ├── shared metadata:
   │     tags
   └── batches: Vec<BatchJob>           ── all batches in the flow

[ BatchJob ]                 ── 1 .bash = 1 sbatch unit
   ├── name:    Option<String>           ── label like "g16", "post"
   ├── parents: Vec<BatchEdge>           ── intra-flow DAG (empty = root)
   ├── config:  SlurmJobConfig           ── existing type (TaskManager-merged)
   └── body:    String                    ── bash body text

[ BatchEdge ]
   ├── parent: BatchIdx                  ── index into JobFlow.batches
   └── kind:   DependencyType            ── existing enum (Afterok / ...)

[ BatchIdx ] = newtype around usize

[ JobLifecycleStatus ]                   ── independent enum
[ StatusEntry ] = (JobLifecycleStatus, DateTime<Utc>)
```

## 5. Type Specifications

All structs derive at minimum: `Debug`, `Clone`, `PartialEq`, `Eq`, `serde::Serialize`, `serde::Deserialize`. Hash where it can be implemented without breaking semantics. `serde(deny_unknown_fields)` for top-level types.

### 5.1 `JobFlow` (new — `src/entities/job_flow.rs`)

```rust
pub struct JobFlow {
    /// UUID v7 — identifier of this logical job-flow unit.
    pub uuid: Uuid,

    /// Program identifier ("gaussian" など). Newtype for type safety.
    pub program: Program,

    /// Calculation type ("opt", "freq", ...). Newtype for type safety.
    pub calc_type: CalcType,

    /// Creation timestamp (UTC).
    pub created_at: DateTime<Utc>,

    /// Cross-flow parents — UUIDs of other JobFlows whose results
    /// this flow consumes. Empty for root flows.
    pub parent_uuids: Vec<Uuid>,

    /// Optional experiment grouping ID.
    pub experiment_id: Option<ExperimentId>,

    /// Free-form metadata tags. BTreeMap for deterministic order.
    pub tags: BTreeMap<String, String>,

    /// The intra-flow batches (DAG nodes). Index into this Vec is the
    /// `BatchIdx` referenced by `BatchEdge.parent`.
    pub batches: Vec<BatchJob>,
}

pub struct Program(pub String);
pub struct CalcType(pub String);
pub struct ExperimentId(pub String);
```

**Notes:**
- `Program`, `CalcType`, `ExperimentId` are tuple-newtypes around `String`. `Display` and `From<String> / FromStr` are implemented.
- Validation rules (e.g., non-empty, no whitespace) are NOT enforced in the constructor in this PR — left to TaskManager.
- The `params` field present in Python's `Metadata` is **not** included; program-specific data lives in `gaussian16` module side-by-side.
- The `compounds` field present in Python's `Metadata` is **not** included; same reasoning.

### 5.2 `BatchJob` (new — `src/entities/slurm/batch.rs`)

```rust
pub struct BatchJob {
    /// Optional human-readable label (e.g. "g16", "post"). Used for
    /// logging / debugging only — has no semantic effect.
    pub name: Option<String>,

    /// Intra-flow dependency edges. Empty = root. Multiple entries =
    /// this batch depends on multiple parents (DAG join node).
    pub parents: Vec<BatchEdge>,

    /// Slurm submission directives. TaskManager produces this by
    /// merging cluster-wide defaults with per-job overrides — by the
    /// time it lands in BatchJob it is already complete.
    pub config: SlurmJobConfig,

    /// Bash script body (the part of the .bash file *after* the
    /// `#SBATCH` directive block). May contain template placeholders
    /// pre-substituted by TaskManager.
    pub body: String,
}

pub struct BatchEdge {
    /// Index into the enclosing `JobFlow.batches`.
    pub parent: BatchIdx,

    /// Dependency type (Afterok / Afterany / After / ...).
    /// Reuses the existing enum from `entities::slurm::dependency`.
    pub kind: DependencyType,
}

pub struct BatchIdx(pub usize);
```

**Notes:**
- `BatchEdge` does **not** carry a `delay_minutes` field. The existing `DependencyJobRef` supports it for `After`-typed clauses; this is left out of the intra-flow edge for simplicity and added later if needed.
- The relationship to `SlurmJobConfig.dependency` (which references concrete jobids): `BatchEdge` is the *logical* intra-flow dependency. After the parent is submitted, TaskManager resolves each `BatchEdge` into a `SlurmDependency` clause and merges it into the child's `config.dependency`. Both can coexist (e.g., one cross-flow dependency on a parent JobFlow's last batch + one intra-flow dependency on a sibling batch).
- `body` as plain `String` is intentional — most env-setup boilerplate (shebang, `set -euo pipefail`, conda cleanup, module restore, conda activate) is treated as a fixed string template applied by TaskManager. The program-specific main command portion is owned by the program-specific module (e.g., `gaussian16`).

### 5.3 `JobLifecycleStatus` / `StatusEntry` (new — `src/entities/slurm/status.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobLifecycleStatus {
    Queued,
    Running,
    Done,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusEntry {
    pub status: JobLifecycleStatus,
    pub transitioned_at: DateTime<Utc>,
}
```

**Notes:**
- Mirrors the Python `Status` (StrEnum) and `StatusEntry` dataclass exactly.
- TOML-side / file-side serialization (Python writes `"<status> <ISO8601-UTC>"` to a one-line file) is **not** implemented in this PR — types only.
- These types are NOT embedded in `JobFlow` or `BatchJob`. They are read/written separately by the future status-tracking layer.

## 6. Module Layout

```
src/entities/
├── mod (entities.rs)            — re-exports
├── job_flow.rs                  ← NEW: JobFlow, Program, CalcType, ExperimentId
├── slurm.rs                     — existing: SlurmJobConfig + re-exports (unchanged)
└── slurm/
    ├── array_spec.rs            — existing (unchanged)
    ├── dependency.rs            — existing (unchanged)
    ├── resource_spec.rs         — existing (unchanged)
    ├── time_limit.rs            — existing (unchanged)
    ├── batch.rs                 ← NEW: BatchJob, BatchEdge, BatchIdx
    └── status.rs                ← NEW: JobLifecycleStatus, StatusEntry
```

**Layering rationale:**
- `JobFlow` lives directly under `entities/` because it carries no Slurm-internal state — only an identity layer that *contains* a list of Slurm batches.
- `BatchJob` lives under `entities/slurm/` because its `config: SlurmJobConfig` is Slurm-specific.
- `JobLifecycleStatus` lives under `entities/slurm/` because it tracks a Slurm job's lifecycle.

## 7. Re-exports

`src/entities/slurm.rs` adds:

```rust
pub mod batch;
pub mod status;

pub use batch::{BatchEdge, BatchIdx, BatchJob};
pub use status::{JobLifecycleStatus, StatusEntry};
```

`src/entities.rs` adds:

```rust
pub mod job_flow;

pub use job_flow::{CalcType, ExperimentId, JobFlow, Program};
```

## 8. Validation

Validation is **not** enforced by these types in this PR. The following invariants are left to a future TaskManager layer:

- `BatchEdge.parent` indices are within `JobFlow.batches.len()`
- The intra-flow batch graph is acyclic
- `parent_uuids` does not contain `uuid` (no self-reference in cross-flow)
- `Program` / `CalcType` non-empty after trim
- `name` non-empty after trim when present

The types are pure data — they accept any well-typed value and validate at the higher layer.

## 9. Testing

Unit tests (in each new file) cover:

- `JobFlow` round-trip TOML serialize/deserialize with each field varying
- `JobFlow` with empty `batches`, multiple `batches`, multiple `parent_uuids`
- `BatchJob` with `parents = []` (root) and `parents.len() > 1` (DAG join)
- `BatchEdge` round-trip TOML for each `DependencyType` variant
- `JobLifecycleStatus` round-trip TOML lowercase mapping
- `StatusEntry` round-trip TOML with timezone-aware `DateTime<Utc>`
- `BatchIdx` newtype: `Eq` / `Ord` / serde behave as bare `usize`

No integration tests for submit / tick — out of scope.

## 10. Backwards Compatibility

- Existing `SlurmJobConfig`, `SlurmArraySpec`, `SlurmDependency`, `ResourceSpec`, `JobTimeLimit` and their re-exports are **unchanged**.
- New types are additive. No public API removed.
- pyo3 bindings: this PR does not export the new types to Python; that is a follow-up PR.

## 11. Open Questions / Future Work

- **DAG validation**: when adding TaskManager, define a `JobFlowError::CycleDetected` and a graph validator.
- **`delay_minutes` on `BatchEdge`**: defer until a real After-with-delay use-case appears.
- **TOML round-trip for `JobFlow`**: the canonical filename, schema, and `read_*` / `write_*` helpers are TaskManager / Metadata-layer concerns.
- **Generic-over-program JobFlow**: 5b (params/compounds outside JobFlow) is the chosen approach. If many programs each carry rich metadata, consider a side-by-side `JobFlowEnvelope<P>` wrapper at the metadata layer rather than introducing generics on `JobFlow` itself.
- **`Program` as enum**: currently a newtype String for openness. If a closed set ever becomes desirable, a follow-up PR can introduce `enum Program { Gaussian, Other(String) }` with a custom serde impl.
- **`JobLifecycleStatus` transition logic**: when adding the status-tracking layer, port Python's `_decide_transition`; this PR ships the data type only.
