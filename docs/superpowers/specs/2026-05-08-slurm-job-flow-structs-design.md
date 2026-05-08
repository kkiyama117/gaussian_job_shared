# Job and JobFlow Struct Design

> **Terminology in this spec:**
> - **Job** = a single bash file = a single `sbatch` submission unit. Carries its `program`, `config` (`SlurmJobConfig`), and `body` (bash text). Free of any flow-scoped reference (no `JobId`, no edge data).
> - **JobNode** = a position of a Job in a JobFlow. Wraps one `Job` plus its stable `id: JobId` and incoming `parents: Vec<JobEdge>`.
> - **JobFlow** = a DAG of `JobNode`s plus identity, lineage, `work_dir`, and tags. Program-agnostic; each Job declares its own program.

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
| Working directory (data root for the flow) | `JobFlow.work_dir` |
| Set of nodes (Jobs + their incoming edges) | `JobFlow.jobs: Vec<JobNode>` |
| Calculation type (overall flow purpose)  | `JobFlow.calc_type` |
| Stable ID of a node within the flow      | `JobNode.id: JobId` |
| Edges incident to a node (parents)       | `JobNode.parents` |
| Program executed in this stage           | `Job`      |
| Slurm submission directives              | `Job.config` |
| Bash script body                         | `Job.body` |
| Runtime mapping `JobId → SlurmJobId`     | **TaskManager** (out of scope here) |
| Status transitions / summary file        | **TaskManager** (out of scope here) |

**Why edges live in `JobNode.parents`, not on `Job`:** edges are a property of the graph position, not of the work definition. `Job` itself stays free of `JobId` references so it can be cloned/moved between flows; the wrapper `JobNode` owns the `id` and `parents` because those only make sense in the enclosing flow.

**Why `JobId` rather than positional `JobIdx`:** stable IDs survive reordering, are human-readable in TOML (`from = "g16"` vs `from = 0`), and double as bash-filename / log-prefix keys. This is the standard pattern in Airflow / GitHub Actions / Compose / k8s-style configs.

## 2. Non-Goals

The following are explicitly **out of scope** of this PR:

- TaskManager / submit / tick logic
- Storing post-submission identifiers (`SlurmJobId` produced by `sbatch`); the `JobId → SlurmJobId` runtime map lives in TaskManager / a separate state file
- Filesystem operations: directory creation under `work_dir`, bash file rendering, summary-file writing
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
| `PlannedStep`                                                  | `JobFlow` + `Vec<JobNode>` (each with its `parents`) collectively |
| Implicit `(g16, post)` pair                                    | `JobNode { id: "g16", … }` + `JobNode { id: "post", parents: [{ from: "g16", kind: Afterok }] }` |
| `_StepSubmitter.submit_step`                                   | Out of scope — TaskManager responsibility                |
| `Status` (queued/running/done/failed)                          | `JobLifecycleStatus` (independent type, not embedded)    |
| `StatusEntry` (status + transitioned_at)                       | `StatusEntry` (matches Python's shape)                   |
| `SlurmJobState` (PENDING/RUNNING/...)                          | Out of scope — already exists in `slurm-async-runner`    |
| `TickResult`, `SubmitResult`, `SbatchError`                    | Out of scope — TaskManager responsibility                |

## 4. Architecture

```
[ JobFlow ]                  ── 1 logical job-flow unit (uuid + DAG)
   ├── identity:
   │     uuid / calc_type / created_at
   ├── lineage (cross-flow):
   │     parent_uuids / experiment_id
   ├── work_dir: PathBuf                  ── data root for this flow
   ├── tags:    BTreeMap<String, String>
   └── jobs:    Vec<JobNode>              ── DAG (each node = work + incoming edges)

[ JobNode ]                  ── one DAG position
   ├── id:      JobId                     ── stable ID, unique within flow
   ├── job:     Job                       ── work definition (flatten on serde)
   └── parents: Vec<JobEdge>              ── incoming edges (empty = root)

[ Job ]                      ── 1 bash file = 1 sbatch unit (self-contained)
   ├── program: Program                   ── what this stage runs (e.g. "g16", "formchk")
   ├── config:  SlurmJobConfig            ── existing type (TaskManager-merged)
   └── body:    String                    ── bash body text

[ JobEdge ]
   ├── from: JobId                        ── parent (stable ID of another node in this flow)
   └── kind: DependencyType               ── existing enum (Afterok / ...)

[ JobId ] = newtype around String         ── stable, flow-scoped ID

[ JobLifecycleStatus ]                    ── independent enum
[ StatusEntry ] = (JobLifecycleStatus, DateTime<Utc>)
```

### 4.1 Runtime Workflow Contract (TaskManager scope, future PR)

How TaskManager will use the types defined here:

| Step | Action | Type role |
|------|--------|-----------|
| 1 | Create per-Job folders under `JobFlow.work_dir/<JobId>/` | `JobFlow.work_dir` + `JobNode.id` |
| 2 | Render `Job.config` as `#SBATCH ...` directives + `Job.body` → write `<work_dir>/<JobId>/<JobId>.bash` (plus program-specific extra inputs from `gaussian16` module) | `Job.config`, `Job.body`, `JobNode.id` |
| 3 | Construct in-memory DAG | `JobFlow.jobs` is the DAG (no extra construction needed) |
| 4 | Submit roots: `sbatch <work_dir>/<JobId>/<JobId>.bash` for each `JobNode` with `parents.is_empty()` | `JobNode.parents` |
| 5 | Capture `SlurmJobId` from sbatch stdout, store `BTreeMap<JobId, SlurmJobId>`. For each non-root child, build `--dependency=<kind>:<parent_slurm_id>:...` from `parents` + the map, then submit | `JobNode.parents`, `JobEdge.from`, `JobEdge.kind` |
| 6 | Persist the `JobId → SlurmJobId` map (and lifecycle entries) to `<work_dir>/summary.toml` for human inspection | `StatusEntry` (per-Job) |

Steps 1–6 are **not implemented in this PR**. They are listed here so the data model in §5 can be reviewed against actual usage.

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

    /// Working directory for this flow. TaskManager creates per-Job
    /// subfolders under this path (`<work_dir>/<JobId>/`) and writes
    /// the rendered `.bash`, log files, and per-Job state there.
    /// Path resolution (relative vs absolute) is TaskManager's concern.
    pub work_dir: PathBuf,

    /// Free-form metadata tags. BTreeMap for deterministic order.
    pub tags: BTreeMap<String, String>,

    /// The DAG: each entry is one position in the flow. The Vec's order
    /// has no semantic meaning — references between nodes use `JobId`,
    /// not Vec indices, so reordering is safe.
    pub jobs: Vec<JobNode>,
}

pub struct CalcType(pub String);
pub struct ExperimentId(pub String);
```

**Notes:**
- `JobFlow` is intentionally program-agnostic: it carries no `program` field. Each `Job` declares its own `program` because different stages of a flow may run different binaries (e.g. `g16` for the main step, `formchk` for the post step).
- `work_dir` is a flow-level concern (the *flow's* data lives there); per-Job folder layout under it is TaskManager's policy and is not modelled in `JobNode`.
- `CalcType` and `ExperimentId` are tuple-newtypes around `String`. `Display` and `From<String> / FromStr` are implemented.
- Validation rules (e.g., non-empty, unique `JobId` within `jobs`, edge `from` resolvable) are NOT enforced in the constructor in this PR — left to TaskManager.
- The `params` field present in Python's `Metadata` is **not** included; program-specific data lives in `gaussian16` module side-by-side.
- The `compounds` field present in Python's `Metadata` is **not** included; same reasoning.

### 5.2 `JobNode`, `Job`, `JobEdge` (new — `src/entities/slurm/job.rs`)

```rust
pub struct JobNode {
    /// Stable ID of this position in the flow. Required and unique
    /// within the enclosing `JobFlow.jobs`. Used as:
    ///   - the lookup key for `JobEdge.from`
    ///   - the per-Job folder name under `JobFlow.work_dir`
    ///   - the `.bash` filename stem and log-file prefix
    pub id: JobId,

    /// The work definition. `#[serde(flatten)]` so that `program`,
    /// `config`, `body` appear as siblings of `id` and `parents` in
    /// the TOML representation rather than under a nested `[job]` table.
    #[serde(flatten)]
    pub job: Job,

    /// Incoming dependency edges. Empty = root (this Job is submitted
    /// first, with no `--dependency` flag). Multiple entries = DAG join.
    pub parents: Vec<JobEdge>,
}

pub struct Job {
    /// Program identifier this Job runs (e.g. "g16", "formchk",
    /// "gaussview", program-specific analyzers). Newtype around String.
    pub program: Program,

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
    /// Parent (predecessor) — `JobId` of another node in the same flow.
    pub from: JobId,

    /// Dependency kind (Afterok / Afterany / After / ...).
    /// Reuses the existing enum from `entities::slurm::dependency`.
    pub kind: DependencyType,
}

pub struct JobId(pub String);

pub struct Program(pub String);
```

**Notes:**
- **Layering:** `JobNode` is the position-in-flow wrapper that carries `id` and `parents`; `Job` is the pure work definition. This split keeps `Job` free of any flow-specific reference (no `JobId` in `Job`), so a `Job` value can be cloned or moved between flows by wrapping it in a new `JobNode`.
- **`#[serde(flatten)]`:** at the TOML level, a `[[jobs]]` entry is one flat block with `id`, `program`, `body`, `parents`, and `config` (subtable) at the same level — no extra nesting layer for the wrapper.
- **No `name` field:** `JobId` doubles as the human-readable label; an extra optional display name was deemed unnecessary (YAGNI).
- **Why `JobEdge` carries no `to`:** `to` is implicit — it is the enclosing `JobNode.id`. This eliminates a field and an integrity check.
- **`Program`** lives in this module (alongside `Job`) because it is a per-Job concern. `Display` and `From<String> / FromStr` are implemented.
- **No `delay_minutes`:** the existing `DependencyJobRef` supports it for `After`-typed clauses; this is left out of the intra-flow edge for simplicity and added later if needed.
- **Relationship to `SlurmJobConfig.dependency`:** `JobEdge` is the *logical* intra-flow dependency. At submission time, TaskManager looks up each parent's runtime `SlurmJobId`, builds a `SlurmDependency` clause, and merges it into the child's `config.dependency` before calling `sbatch`. Cross-flow dependencies (against a different JobFlow's last `JobId`) and intra-flow dependencies coexist on the same child.
- **`body` as plain `String`:** most env-setup boilerplate (shebang, `set -euo pipefail`, conda cleanup, module restore, conda activate) is a fixed string template applied by TaskManager. The program-specific main command is owned by the program-specific module (e.g., `gaussian16`).

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
- These types are NOT embedded in `JobFlow`, `JobNode`, or `Job`. They are read/written separately by the future status-tracking / summary layer (cf. step 6 of §4.1).

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
    ├── job.rs                   ← NEW: JobNode, Job, JobEdge, JobId, Program
    └── status.rs                ← NEW: JobLifecycleStatus, StatusEntry
```

**Layering rationale:**
- `JobFlow` lives directly under `entities/` because it carries no Slurm-internal state — only an identity layer that *contains* a list of Slurm `JobNode`s.
- `JobNode` and `Job` live under `entities/slurm/` because `Job.config: SlurmJobConfig` is Slurm-specific.
- `Program` and `JobId` live alongside `JobNode` / `Job` in `entities/slurm/job.rs` because they are per-Job concerns.
- `JobLifecycleStatus` lives under `entities/slurm/` because it tracks a Slurm job's lifecycle.

## 7. Re-exports

`src/entities/slurm.rs` adds:

```rust
pub mod job;
pub mod status;

pub use job::{Job, JobEdge, JobId, JobNode, Program};
pub use status::{JobLifecycleStatus, StatusEntry};
```

`src/entities.rs` adds:

```rust
pub mod job_flow;

pub use job_flow::{CalcType, ExperimentId, JobFlow};
```

## 8. Validation

Validation is **not** enforced by these types in this PR. The following invariants are left to a future TaskManager layer:

- `JobNode.id` values are unique within `JobFlow.jobs`
- For every `JobEdge.from` in any `parents`, there exists a `JobNode` in the same flow whose `id` equals it
- `JobEdge.from != enclosing JobNode.id` (no self-loop)
- The intra-flow graph (nodes × incoming edges) is acyclic
- `parent_uuids` does not contain `uuid` (no self-reference in cross-flow)
- `JobId` / `Program` / `CalcType` non-empty after trim, and `JobId` matches a usable filename charset (letters, digits, `-`, `_`)
- `JobFlow.work_dir` non-empty (path validation — existence, writability — is TaskManager's concern at submission time)

The types are pure data — they accept any well-typed value and validate at the higher layer.

## 9. Testing

Unit tests (in each new file) cover:

- `JobFlow` round-trip TOML serialize/deserialize with each field varying (including `work_dir`)
- `JobFlow` with empty `jobs`, multiple `jobs`, multiple `parent_uuids`
- `JobNode` flatten behaviour: a `[[jobs]]` block in TOML has `id`, `program`, `body`, `parents`, and `[jobs.config]` at the same level (no `[jobs.job]` nesting)
- `JobNode` with `parents = []` (root), one parent (linear chain), and `parents.len() > 1` (DAG join)
- `JobEdge` round-trip TOML for each `DependencyType` variant
- `JobId` round-trip and equality (e.g. `"g16"` and `"post"` distinct)
- `JobLifecycleStatus` round-trip TOML lowercase mapping
- `StatusEntry` round-trip TOML with timezone-aware `DateTime<Utc>`

No integration tests for submit / tick — out of scope.

## 10. Backwards Compatibility

- Existing `SlurmJobConfig`, `SlurmArraySpec`, `SlurmDependency`, `ResourceSpec`, `JobTimeLimit` and their re-exports are **unchanged**.
- New types are additive. No public API removed.
- pyo3 bindings: this PR does not export the new types to Python; that is a follow-up PR.

## 11. Open Questions / Future Work

- **DAG validation**: when adding TaskManager, define a `JobFlowError::{DuplicateJobId, UnknownParent, CycleDetected, SelfLoop}` and a graph validator.
- **`SlurmJobId` newtype + runtime state file**: defined in the TaskManager PR. Likely shape: `pub struct SlurmJobId(pub String);` plus a sibling `summary.toml` keyed by `JobId` (per step 6 of §4.1).
- **Submission helpers on `JobFlow`**: `JobFlow::find(&JobId)`, `JobFlow::roots()`, `JobFlow::topological()` — likely live in TaskManager rather than directly on `JobFlow` to keep the data type pure.
- **`delay_minutes` on `JobEdge`**: defer until a real After-with-delay use-case appears.
- **TOML round-trip for `JobFlow`**: the canonical filename, schema, and `read_*` / `write_*` helpers are TaskManager / Metadata-layer concerns.
- **`calc_type` placement**: kept at `JobFlow` level for now (it describes the overall flow purpose). If we ever want stage-level calc kinds (e.g. main = "opt", post = "fchk-extract"), we can add a `Job.role` field as a follow-up rather than moving `calc_type`.
- **Generic-over-program JobFlow**: 5b (params/compounds outside JobFlow) is the chosen approach. If many programs each carry rich metadata, consider a side-by-side `JobFlowEnvelope<P>` wrapper at the metadata layer rather than introducing generics on `JobFlow` itself.
- **`Program` as enum**: currently a newtype String for openness. If a closed set ever becomes desirable, a follow-up PR can introduce `enum Program { Gaussian, Other(String) }` with a custom serde impl.
- **`JobLifecycleStatus` transition logic**: when adding the status-tracking layer, port Python's `_decide_transition`; this PR ships the data type only.
