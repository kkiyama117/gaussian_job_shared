# Job and JobFlow Struct Design

> **Terminology in this spec:**
> - **Job** = a single bash file = a single `sbatch` submission unit. Carries its own `program` (what it runs) and its own `SlurmJobConfig`.
> - **JobFlow** = a DAG of Jobs that together accomplish one logical calculation. Carries identity, lineage, and tags only — *no* `program` field, since each Job declares its own.

**Date:** 2026-05-08
**Branch:** `slurm-job-structs`
**Crate:** `gaussian_job_shared` (Rust portion of `gaussian-job-shared2`)

## 1. Goal

Add data types that represent **a single Slurm job (`Job` — one bash file = one sbatch unit)** and **a job flow (`JobFlow` — a DAG of Jobs that together accomplish a complex task)** to the `gaussian_job_shared` Rust crate.

The data model must:

- Stay **program-agnostic at the JobFlow level**. JobFlow is purely orchestration; the `program` identifier is a per-`Job` concern (different stages may run different programs, e.g. `g16` for the main step and `formchk` / analysis script for the post step).
- Reuse the existing `SlurmJobConfig` (and its components: `SlurmArraySpec`, `SlurmDependency`, `ResourceSpec`, `JobTimeLimit`) without modification.
- Express **intra-flow Job dependencies** as a DAG so that fork/join topologies are representable in the future.
- Be **send-side only**: no submission ID (`slurm_jobid`, `post_jobid`), no submit/tick logic, no override-merging logic. Those concerns belong to a future `TaskManager` layer.

### 1.1 Responsibility Split

| Concern                                  | Owned by   |
|------------------------------------------|------------|
| Identity / lineage / experiment grouping | `JobFlow`  |
| Tags                                     | `JobFlow`  |
| DAG topology of stages                   | `JobFlow.jobs` + `Job.parents` |
| Calculation type (overall flow purpose)  | `JobFlow.calc_type` |
| Program executed in this stage           | `Job`      |
| Slurm submission directives              | `Job.config` |
| Bash script body                         | `Job.body` |
| Per-Job parents (intra-flow edges)       | `Job.parents` |

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
| `CalcBlock` (uuid, program, calc_type, parent_uuids, ...)      | Split: identity/lineage/calc_type → `JobFlow`; `program` → `Job` |
| `Compounds`                                                    | Out of scope — `gaussian16::Compounds` (program-side)    |
| `CalcParams` / `GaussianParams`                                | Out of scope — `gaussian16::JobParams` (program-side)    |
| `CalcBlock.slurm_jobid` / `post_jobid`                         | Out of scope — runtime IDs not modelled here             |
| `PlannedStep`                                                  | `JobFlow` + `Vec<Job>` collectively                      |
| Implicit `(g16, post)` pair                                    | `Job` × 2 with one `JobEdge` (post → main, Afterok)      |
| `_StepSubmitter.submit_step`                                   | Out of scope — TaskManager responsibility                |
| `Status` (queued/running/done/failed)                          | `JobLifecycleStatus` (independent type, not embedded)    |
| `StatusEntry` (status + transitioned_at)                       | `StatusEntry` (matches Python's shape)                   |
| `SlurmJobState` (PENDING/RUNNING/...)                          | Out of scope — already exists in `slurm-async-runner`    |
| `TickResult`, `SubmitResult`, `SbatchError`                    | Out of scope — TaskManager responsibility                |

## 4. Architecture

```
[ JobFlow ]                  ── 1 logical job-flow unit (uuid + jobs DAG)
   ├── identity:
   │     uuid / calc_type / created_at
   ├── lineage (cross-flow):
   │     parent_uuids / experiment_id
   ├── shared metadata:
   │     tags
   └── jobs: Vec<Job>                    ── all Jobs in the flow

[ Job ]                      ── 1 bash file = 1 sbatch unit
   ├── name:    Option<String>           ── label like "g16", "post"
   ├── program: Program                  ── what this stage runs (e.g. "g16", "formchk")
   ├── parents: Vec<JobEdge>             ── intra-flow DAG (empty = root)
   ├── config:  SlurmJobConfig           ── existing type (TaskManager-merged)
   └── body:    String                    ── bash body text

[ JobEdge ]
   ├── parent: JobIdx                    ── index into JobFlow.jobs
   └── kind:   DependencyType            ── existing enum (Afterok / ...)

[ JobIdx ] = newtype around usize

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

    /// Calculation type ("opt", "freq", "opt+td", ...). Describes the
    /// overall purpose of the flow as a whole. Newtype for type safety.
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

    /// The Jobs in the flow (DAG nodes). Index into this Vec is the
    /// `JobIdx` referenced by `JobEdge.parent`.
    pub jobs: Vec<Job>,
}

pub struct CalcType(pub String);
pub struct ExperimentId(pub String);
```

**Notes:**
- `JobFlow` is intentionally program-agnostic: it carries no `program` field. Each `Job` declares its own `program` because different stages of a flow may run different binaries (e.g. `g16` for the main step, `formchk` for the post step).
- `CalcType` and `ExperimentId` are tuple-newtypes around `String`. `Display` and `From<String> / FromStr` are implemented.
- Validation rules (e.g., non-empty, no whitespace) are NOT enforced in the constructor in this PR — left to TaskManager.
- The `params` field present in Python's `Metadata` is **not** included; program-specific data lives in `gaussian16` module side-by-side.
- The `compounds` field present in Python's `Metadata` is **not** included; same reasoning.

### 5.2 `Job` (new — `src/entities/slurm/job.rs`)

```rust
pub struct Job {
    /// Optional human-readable label (e.g. "g16", "post"). Used for
    /// logging / debugging only — has no semantic effect.
    pub name: Option<String>,

    /// Program identifier this Job runs (e.g. "g16", "formchk",
    /// "gaussview", program-specific analyzers). Newtype around String.
    pub program: Program,

    /// Intra-flow dependency edges. Empty = root. Multiple entries =
    /// this Job depends on multiple parents (DAG join node).
    pub parents: Vec<JobEdge>,

    /// Slurm submission directives. TaskManager produces this by
    /// merging cluster-wide defaults with per-job overrides — by the
    /// time it lands in Job it is already complete.
    pub config: SlurmJobConfig,

    /// Bash script body (the part of the .bash file *after* the
    /// `#SBATCH` directive block). May contain template placeholders
    /// pre-substituted by TaskManager.
    pub body: String,
}

pub struct JobEdge {
    /// Index into the enclosing `JobFlow.jobs`.
    pub parent: JobIdx,

    /// Dependency type (Afterok / Afterany / After / ...).
    /// Reuses the existing enum from `entities::slurm::dependency`.
    pub kind: DependencyType,
}

pub struct JobIdx(pub usize);

pub struct Program(pub String);
```

**Notes:**
- `Program` lives in this module (alongside `Job`) because it is a per-Job concern. `Display` and `From<String> / FromStr` are implemented.
- `JobEdge` does **not** carry a `delay_minutes` field. The existing `DependencyJobRef` supports it for `After`-typed clauses; this is left out of the intra-flow edge for simplicity and added later if needed.
- The relationship to `SlurmJobConfig.dependency` (which references concrete jobids): `JobEdge` is the *logical* intra-flow dependency. After the parent is submitted, TaskManager resolves each `JobEdge` into a `SlurmDependency` clause and merges it into the child's `config.dependency`. Both can coexist (e.g., one cross-flow dependency on a parent JobFlow's last Job + one intra-flow dependency on a sibling Job).
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
- These types are NOT embedded in `JobFlow` or `Job`. They are read/written separately by the future status-tracking layer.

## 6. Module Layout

```
src/entities/
├── mod (entities.rs)            — re-exports
├── job_flow.rs                  ← NEW: JobFlow, CalcType, ExperimentId
├── slurm.rs                     — existing: SlurmJobConfig + re-exports (unchanged)
└── slurm/
    ├── array_spec.rs            — existing (unchanged)
    ├── dependency.rs            — existing (unchanged)
    ├── resource_spec.rs         — existing (unchanged)
    ├── time_limit.rs            — existing (unchanged)
    ├── job.rs                   ← NEW: Job, JobEdge, JobIdx, Program
    └── status.rs                ← NEW: JobLifecycleStatus, StatusEntry
```

**Layering rationale:**
- `JobFlow` lives directly under `entities/` because it carries no Slurm-internal state — only an identity layer that *contains* a list of Slurm `Job`s.
- `Job` lives under `entities/slurm/` because its `config: SlurmJobConfig` is Slurm-specific.
- `Program` lives alongside `Job` (in `entities/slurm/job.rs`) because it is a per-Job concern, not a JobFlow-level concern.
- `JobLifecycleStatus` lives under `entities/slurm/` because it tracks a Slurm job's lifecycle.

## 7. Re-exports

`src/entities/slurm.rs` adds:

```rust
pub mod job;
pub mod status;

pub use job::{Job, JobEdge, JobIdx, Program};
pub use status::{JobLifecycleStatus, StatusEntry};
```

`src/entities.rs` adds:

```rust
pub mod job_flow;

pub use job_flow::{CalcType, ExperimentId, JobFlow};
```

## 8. Validation

Validation is **not** enforced by these types in this PR. The following invariants are left to a future TaskManager layer:

- `JobEdge.parent` indices are within `JobFlow.jobs.len()`
- The intra-flow Job graph is acyclic
- `parent_uuids` does not contain `uuid` (no self-reference in cross-flow)
- `Program` / `CalcType` non-empty after trim
- `name` non-empty after trim when present

The types are pure data — they accept any well-typed value and validate at the higher layer.

## 9. Testing

Unit tests (in each new file) cover:

- `JobFlow` round-trip TOML serialize/deserialize with each field varying
- `JobFlow` with empty `jobs`, multiple `jobs`, multiple `parent_uuids`
- `Job` with `parents = []` (root), `parents.len() > 1` (DAG join), and varying `program`
- `JobEdge` round-trip TOML for each `DependencyType` variant
- `JobLifecycleStatus` round-trip TOML lowercase mapping
- `StatusEntry` round-trip TOML with timezone-aware `DateTime<Utc>`
- `JobIdx` newtype: `Eq` / `Ord` / serde behave as bare `usize`

No integration tests for submit / tick — out of scope.

## 10. Backwards Compatibility

- Existing `SlurmJobConfig`, `SlurmArraySpec`, `SlurmDependency`, `ResourceSpec`, `JobTimeLimit` and their re-exports are **unchanged**.
- New types are additive. No public API removed.
- pyo3 bindings: this PR does not export the new types to Python; that is a follow-up PR.

## 11. Open Questions / Future Work

- **DAG validation**: when adding TaskManager, define a `JobFlowError::CycleDetected` and a graph validator.
- **`delay_minutes` on `JobEdge`**: defer until a real After-with-delay use-case appears.
- **TOML round-trip for `JobFlow`**: the canonical filename, schema, and `read_*` / `write_*` helpers are TaskManager / Metadata-layer concerns.
- **`calc_type` placement**: kept at `JobFlow` level for now (it describes the overall flow purpose). If we ever want stage-level calc kinds (e.g. main = "opt", post = "fchk-extract"), we can add a `Job.role` field as a follow-up rather than moving `calc_type`.
- **Generic-over-program JobFlow**: 5b (params/compounds outside JobFlow) is the chosen approach. If many programs each carry rich metadata, consider a side-by-side `JobFlowEnvelope<P>` wrapper at the metadata layer rather than introducing generics on `JobFlow` itself.
- **`Program` as enum**: currently a newtype String for openness. If a closed set ever becomes desirable, a follow-up PR can introduce `enum Program { Gaussian, Other(String) }` with a custom serde impl.
- **`JobLifecycleStatus` transition logic**: when adding the status-tracking layer, port Python's `_decide_transition`; this PR ships the data type only.
