# Job and JobFlow Struct Design

> **Terminology in this spec — two-tier Job design:**
> - **`JobSpec` (small / state-independent)** = the pure work definition: `program`, `config` (`SlurmJobConfig`), and `body` (bash text). Can be written before any flow exists; carries no flow-scoped or runtime state. Reusable across flows.
> - **`Job` (large / in-flow)** = a `JobSpec` placed in a flow. Adds the flow-scoped data: stable `id: JobId` and incoming `parents: Vec<JobEdge>`. Designed as the future home for runtime state (`slurm_jobid`, `status_history`, …) added by the TaskManager PR.
> - **`JobFlow`** = a DAG of `Job`s plus identity, lineage, `work_dir`, and tags. Program-agnostic; each `JobSpec` inside declares its own program.
>
> Pattern: `Job` *contains* `JobSpec` (`#[serde(flatten)]`). Same idiom as k8s `Pod { spec: PodSpec, status: PodStatus, … }`.

**Date:** 2026-05-08
**Branch:** `slurm-job-structs`
**Crate:** `gaussian_job_shared` (Rust portion of `gaussian-job-shared2`)

## 1. Goal

Add data types that represent **a single Slurm batch unit (split into `JobSpec` + `Job`)** and **a job flow (`JobFlow` — a DAG of `Job`s)** to the `gaussian_job_shared` Rust crate.

The data model must:

- **Two-tier Job design.** `JobSpec` (state-independent / pre-runtime — `program`, `config`, `body`) is reusable. `Job` (in-flow — adds `id`, `parents`, and is the designated extension point for future runtime state like `slurm_jobid`, `status_history`) wraps one `JobSpec`.
- Stay **program-agnostic at the JobFlow level**. `JobFlow` is purely orchestration; the `program` identifier lives on each `JobSpec` (different stages may run different programs, e.g. `g16` for the main step and `formchk` / analysis script for the post step).
- Reuse the existing `SlurmJobConfig` (and its components: `SlurmArraySpec`, `SlurmDependency`, `ResourceSpec`, `JobTimeLimit`) without modification.
- Express **intra-flow Job dependencies** as a DAG so that fork/join topologies are representable in the future.
- Be **send-side only** in this PR: no submission ID (`slurm_jobid`), no submit/tick logic, no override-merging logic. The `Job` type is *shaped* to host runtime state, but the actual fields are added in the TaskManager PR.

### 1.1 Responsibility Split

| Concern                                    | Owned by   |
|--------------------------------------------|------------|
| Identity / experiment grouping             | `JobFlow.uuid` / `JobFlow.experiment_id` |
| Tags                                       | `JobFlow.tags` |
| Working directory (data root for the flow) | `JobFlow.work_dir` |
| Set of `Job`s in the flow                  | `JobFlow.jobs: Vec<Job>` |
| Calculation type (overall flow purpose)    | `JobFlow.calc_type` |
| Stable ID of a Job within the flow         | `Job.id: JobId` |
| Edges incident to a Job (parents)          | `Job.parents` |
| Program executed in this stage             | `JobSpec.program` |
| Slurm submission directives                | `JobSpec.config` |
| Bash script body                           | `JobSpec.body` |
| Runtime state (slurm_jobid, status, ...)   | **`Job` (future fields)** — shape ready, fields out of scope here |
| Cross-flow lineage (`JobFlow → JobFlow`)   | **Out of scope** — see §11 (no `parent_uuids` field in this PR) |
| Runtime mapping `JobId → SlurmJobId`       | TaskManager (out of scope here) |
| Status transitions / summary file          | TaskManager (out of scope here) |

**Why edges live on `Job`, not on `JobSpec`:** edges are a property of the graph *position*, not of the pure work definition. `JobSpec` stays free of any flow-scoped reference (no `JobId`) so it can be cloned/moved between flows. `Job` wraps a `JobSpec` and adds the flow-scoped data (`id`, `parents`).

**Why `JobId` rather than positional `JobIdx`:** stable IDs survive reordering, are human-readable in TOML (`from = "g16"` vs `from = 0`), and double as bash-filename / log-prefix keys. This is the standard pattern in Airflow / GitHub Actions / Compose / k8s-style configs.

**Why two tiers (`JobSpec` + `Job`) instead of one flat type:** the small/large split puts a clean line between "data that exists before submission" and "data that becomes meaningful once placed in a flow / actually run". Future runtime state (`slurm_jobid`, `status_history`, `started_at`) extends `Job` only — `JobSpec` remains stable. This mirrors k8s `PodSpec` vs `Pod`, Airflow `BaseOperator` vs `TaskInstance`, etc.

## 2. Non-Goals

The following are explicitly **out of scope** of this PR:

- TaskManager / submit / tick logic
- Storing post-submission identifiers (`SlurmJobId` produced by `sbatch`); the `JobId → SlurmJobId` runtime map lives in TaskManager / a separate state file
- Filesystem operations: directory creation under `work_dir`, bash file rendering, summary-file writing
- Status transition logic (Python's `_decide_transition` equivalent)
- Program-specific params and compounds — owned by `gaussian16` module
- pyo3 bindings for the new types
- TOML serialization round-trip helpers (`read_metadata` / `write_metadata` equivalents)
- **Cross-flow lineage and DAG operations** (`parent_uuids`, sweep expansion, cross-flow parent resolution). No `parent_uuids` field is added to `JobFlow` in this PR; if cross-flow dependencies are needed later, the choice between an external graph store, a separate `JobFlowEdge` type, or revisiting `parent_uuids` is deferred to that follow-up.

## 3. Reference: Mapping to `gaussian-experiment-manager` (Python δ layer)

| Python (`gaussian-experiment-manager` + `gaussian-job-shared`) | This Rust design                                         |
|----------------------------------------------------------------|----------------------------------------------------------|
| `CalcBlock` (uuid, program, calc_type, ...)                    | Split: identity/calc_type → `JobFlow`; `program` → `JobSpec` |
| `CalcBlock.parent_uuids` (cross-flow lineage)                  | **Out of scope** — cross-flow DAG handled later (see §11) |
| `Compounds`                                                    | Out of scope — `gaussian16::Compounds` (program-side)    |
| `CalcParams` / `GaussianParams`                                | Out of scope — `gaussian16::JobParams` (program-side)    |
| `CalcBlock.slurm_jobid` / `post_jobid`                         | Out of scope — runtime fields will land on `Job` (the large tier) in TaskManager PR |
| `PlannedStep`                                                  | `JobFlow` + `Vec<Job>` (each with its `spec` and `parents`) collectively |
| Implicit `(g16, post)` pair                                    | `Job { id: "g16", spec: …, parents: [] }` + `Job { id: "post", spec: …, parents: [{ from: "g16", kind: Afterok }] }` |
| `_StepSubmitter.submit_step`                                   | Out of scope — TaskManager responsibility                |
| `Status` (queued/running/done/failed)                          | `JobLifecycleStatus` (independent type, not embedded yet) |
| `StatusEntry` (status + transitioned_at)                       | `StatusEntry` (matches Python's shape)                   |
| `SlurmJobState` (PENDING/RUNNING/...)                          | Out of scope — already exists in `slurm-async-runner`    |
| `TickResult`, `SubmitResult`, `SbatchError`                    | Out of scope — TaskManager responsibility                |

## 4. Architecture

```
[ JobFlow ]                  ── 1 logical job-flow unit (uuid + DAG)
   ├── identity:
   │     uuid / calc_type / created_at
   ├── grouping:
   │     experiment_id                    ── optional, for cross-flow grouping only
   ├── work_dir: PathBuf                  ── data root for this flow
   ├── tags:    BTreeMap<String, String>
   └── jobs:    Vec<Job>                  ── DAG (each Job = spec + flow-scoped state)

[ Job ]   (large tier — in-flow / runtime-aware)
   ├── id:      JobId                     ── stable ID, unique within flow
   ├── spec:    JobSpec                   ── work definition (flattened on serde)
   ├── parents: Vec<JobEdge>              ── incoming edges (empty = root)
   └── … future runtime fields (slurm_jobid, status_history) — out of scope here

[ JobSpec ]   (small tier — state-independent / pre-runtime)
   ├── program: Program                   ── what this stage runs (e.g. "g16", "formchk")
   ├── config:  SlurmJobConfig            ── existing type (TaskManager-merged)
   └── body:    String                    ── bash body text

[ JobEdge ]
   ├── from: JobId                        ── parent (stable ID of another Job in this flow)
   └── kind: DependencyType               ── existing enum (Afterok / ...)

[ JobId ] = newtype around String         ── stable, flow-scoped ID

[ JobLifecycleStatus ]                    ── independent enum (will populate Job.status_history later)
[ StatusEntry ] = (JobLifecycleStatus, DateTime<Utc>)
```

### 4.1 Runtime Workflow Contract (TaskManager scope, future PR)

How TaskManager will use the types defined here:

| Step | Action | Type role |
|------|--------|-----------|
| 1 | Create per-Job folders under `JobFlow.work_dir/<JobId>/` | `JobFlow.work_dir` + `Job.id` |
| 2 | Render `JobSpec.config` as `#SBATCH ...` directives + `JobSpec.body` → write `<work_dir>/<JobId>/<JobId>.bash` (plus program-specific extra inputs from `gaussian16` module) | `Job.spec.config`, `Job.spec.body`, `Job.id` |
| 3 | Construct in-memory DAG | `JobFlow.jobs` is the DAG (no extra construction needed) |
| 4 | Submit roots: `sbatch <work_dir>/<JobId>/<JobId>.bash` for each `Job` with `parents.is_empty()` | `Job.parents` |
| 5 | Capture `SlurmJobId` from sbatch stdout, store `BTreeMap<JobId, SlurmJobId>`. For each non-root child, build `--dependency=<kind>:<parent_slurm_id>:...` from `parents` + the map, then submit. (Future: write `slurm_jobid` back into the `Job` struct.) | `Job.parents`, `JobEdge.from`, `JobEdge.kind` |
| 6 | Persist the `JobId → SlurmJobId` map (and lifecycle entries) to `<work_dir>/summary.toml` for human inspection. (Future: `Job.status_history` is the in-memory mirror.) | `StatusEntry` (per-Job) |

Steps 1–6 are **not implemented in this PR**. They are listed here so the data model in §5 can be reviewed against actual usage; in particular, the choice to put `id` and `parents` on the `Job` (large) tier rather than `JobSpec` (small) follows directly from steps 1, 4, and 5 needing those fields together with the slurm_jobid that will land later.

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

    /// Optional experiment grouping ID. Multiple flows that belong to
    /// the same logical experiment share this value; otherwise `None`.
    pub experiment_id: Option<ExperimentId>,

    /// Working directory for this flow. TaskManager creates per-Job
    /// subfolders under this path (`<work_dir>/<JobId>/`) and writes
    /// the rendered `.bash`, log files, and per-Job state there.
    /// Path resolution (relative vs absolute) is TaskManager's concern.
    pub work_dir: PathBuf,

    /// Free-form metadata tags. BTreeMap for deterministic order.
    pub tags: BTreeMap<String, String>,

    /// The DAG: each entry is one Job in the flow. The Vec's order
    /// has no semantic meaning — references between Jobs use `JobId`,
    /// not Vec indices, so reordering is safe.
    pub jobs: Vec<Job>,
}

pub struct CalcType(pub String);
pub struct ExperimentId(pub String);
```

**Notes:**
- `JobFlow` is intentionally program-agnostic: it carries no `program` field. Each `Job` declares its own `program` because different stages of a flow may run different binaries (e.g. `g16` for the main step, `formchk` for the post step).
- `work_dir` is a flow-level concern (the *flow's* data lives there); per-Job folder layout under it is TaskManager's policy and is not modelled in `Job`.
- `CalcType` and `ExperimentId` are tuple-newtypes around `String`. `Display` and `From<String> / FromStr` are implemented.
- Validation rules (e.g., non-empty, unique `JobId` within `jobs`, edge `from` resolvable) are NOT enforced in the constructor in this PR — left to TaskManager.
- The `params` field present in Python's `Metadata` is **not** included; program-specific data lives in `gaussian16` module side-by-side.
- The `compounds` field present in Python's `Metadata` is **not** included; same reasoning.

### 5.2 `Job`, `JobSpec`, `JobEdge` (new — `src/entities/slurm/job.rs`)

```rust
/// LARGE tier: a `JobSpec` placed in a `JobFlow`.
/// Carries the flow-scoped state (`id`, `parents`) and is the designated
/// extension point for runtime state (`slurm_jobid`, `status_history`)
/// added by the TaskManager PR.
pub struct Job {
    /// Stable ID of this Job within the enclosing `JobFlow`.
    /// Required and unique within `JobFlow.jobs`. Used as:
    ///   - the lookup key for `JobEdge.from`
    ///   - the per-Job folder name under `JobFlow.work_dir`
    ///   - the `.bash` filename stem and log-file prefix
    pub id: JobId,

    /// The pure work definition. `#[serde(flatten)]` so that `program`,
    /// `config`, `body` appear as siblings of `id` and `parents` in
    /// the TOML representation rather than under a nested `[spec]` table.
    #[serde(flatten)]
    pub spec: JobSpec,

    /// Incoming dependency edges. Empty = root (this Job is submitted
    /// first, with no `--dependency` flag). Multiple entries = DAG join.
    pub parents: Vec<JobEdge>,

    // Future fields (TaskManager PR — out of scope here):
    //   pub slurm_jobid: Option<SlurmJobId>,
    //   pub status_history: Vec<StatusEntry>,
    //   pub started_at: Option<DateTime<Utc>>,
    //   pub finished_at: Option<DateTime<Utc>>,
}

/// SMALL tier: pure state-independent work definition.
/// Reusable across flows. Cloning / moving a `JobSpec` between
/// flows is a sound operation (no flow-scoped data inside).
pub struct JobSpec {
    /// Program identifier this stage runs (e.g. "g16", "formchk",
    /// "gaussview", program-specific analyzers). Newtype around String.
    pub program: Program,

    /// Slurm submission directives. TaskManager produces this by
    /// merging cluster-wide defaults with per-job overrides — by the
    /// time it lands in JobSpec it is already complete.
    pub config: SlurmJobConfig,

    /// Bash script body (the part of the .bash file *after* the
    /// `#SBATCH` directive block). May contain template placeholders
    /// pre-substituted by TaskManager.
    pub body: String,
}

pub struct JobEdge {
    /// Parent (predecessor) — `JobId` of another `Job` in the same flow.
    pub from: JobId,

    /// Dependency kind (Afterok / Afterany / After / ...).
    /// Reuses the existing enum from `entities::slurm::dependency`.
    pub kind: DependencyType,
}

pub struct JobId(pub String);

pub struct Program(pub String);
```

**Notes:**
- **Two-tier rationale (recap from §1.1):** `JobSpec` is the small/state-independent tier; `Job` is the large/in-flow tier. Future runtime state lands on `Job` only — `JobSpec` stays stable.
- **`#[serde(flatten)]`:** at the TOML level, a `[[jobs]]` entry is one flat block with `id`, `program`, `body`, `parents`, and `[jobs.config]` at the same level — no extra nesting layer for the spec wrapper. Future runtime fields (when added on `Job`) will sit beside these without disturbing the existing layout.
- **No `name` field:** `JobId` doubles as the human-readable label; an extra optional display name was deemed unnecessary (YAGNI).
- **Why `JobEdge` carries no `to`:** `to` is implicit — it is the enclosing `Job.id`. This eliminates a field and an integrity check.
- **`Program`** lives in this module (alongside `Job` / `JobSpec`) because it is a per-spec concern. `Display` and `From<String> / FromStr` are implemented.
- **No `delay_minutes`:** the existing `DependencyJobRef` supports it for `After`-typed clauses; this is left out of the intra-flow edge for simplicity and added later if needed.
- **Relationship to `SlurmJobConfig.dependency`:** `JobEdge` is the *logical* intra-flow dependency. At submission time, TaskManager looks up each parent's runtime `SlurmJobId`, builds a `SlurmDependency` clause, and merges it into the child's `spec.config.dependency` before calling `sbatch`. Cross-flow dependencies are not modelled in `JobFlow` (see §11) — if a child needs to wait on another flow's job, the user pre-populates that as a raw `SlurmDependency` clause in `spec.config.dependency`; intra-flow `JobEdge`s and any pre-set raw clauses simply concatenate.
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
- These types are NOT yet embedded in `JobFlow`, `Job`, or `JobSpec`. They are read/written separately by the future status-tracking / summary layer (cf. step 6 of §4.1). When TaskManager PR adds runtime state, `Vec<StatusEntry>` will likely become a field of `Job` (the large tier).

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
    ├── job.rs                   ← NEW: Job, JobSpec, JobEdge, JobId, Program
    └── status.rs                ← NEW: JobLifecycleStatus, StatusEntry
```

**Layering rationale:**
- `JobFlow` lives directly under `entities/` because it carries no Slurm-internal state — only an identity layer that *contains* a list of Slurm `Job`s.
- `Job` (large) and `JobSpec` (small) live under `entities/slurm/` because `JobSpec.config: SlurmJobConfig` is Slurm-specific.
- `Program` and `JobId` live alongside `Job` / `JobSpec` in `entities/slurm/job.rs` because they are per-Job concerns.
- `JobLifecycleStatus` lives under `entities/slurm/` because it tracks a Slurm job's lifecycle.

## 7. Re-exports

`src/entities/slurm.rs` adds:

```rust
pub mod job;
pub mod status;

pub use job::{Job, JobEdge, JobId, JobSpec, Program};
pub use status::{JobLifecycleStatus, StatusEntry};
```

`src/entities.rs` adds:

```rust
pub mod job_flow;

pub use job_flow::{CalcType, ExperimentId, JobFlow};
```

## 8. Validation

Validation is **not** enforced by these types in this PR. The following invariants are left to a future TaskManager layer:

- `Job.id` values are unique within `JobFlow.jobs`
- For every `JobEdge.from` in any `Job.parents`, there exists a `Job` in the same flow whose `id` equals it
- `JobEdge.from != enclosing Job.id` (no self-loop)
- The intra-flow graph (`Job`s × incoming edges) is acyclic
- `JobId` / `Program` / `CalcType` non-empty after trim, and `JobId` matches a usable filename charset (letters, digits, `-`, `_`)
- `JobFlow.work_dir` non-empty (path validation — existence, writability — is TaskManager's concern at submission time)

The types are pure data — they accept any well-typed value and validate at the higher layer.

## 9. Testing

Unit tests (in each new file) cover:

- `JobFlow` round-trip TOML serialize/deserialize with each field varying (including `work_dir`)
- `JobFlow` with empty `jobs`, multiple `jobs`, with and without `experiment_id`
- `Job` flatten behaviour: a `[[jobs]]` block in TOML has `id`, `program`, `body`, `parents`, and `[jobs.config]` at the same level (no `[jobs.spec]` nesting)
- `Job` with `parents = []` (root), one parent (linear chain), and `parents.len() > 1` (DAG join)
- `JobSpec` round-trip TOML in isolation (verifies it can be serialized standalone — important for the small/large split, since `JobSpec` should be reusable across flows)
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
- **Cross-flow lineage**: deliberately omitted in this PR — no `parent_uuids: Vec<Uuid>` on `JobFlow`. Open question for a future iteration: should cross-flow dependencies be modelled (a) as a separate `JobFlowEdge { from: Uuid, to: Uuid, kind }` graph stored alongside, (b) by reintroducing `parent_uuids`, or (c) by an external graph store (e.g., Python δ-layer)? Decide once a real cross-flow consumer exists.
- **Runtime fields on `Job`**: TaskManager PR adds `slurm_jobid: Option<SlurmJobId>`, `status_history: Vec<StatusEntry>`, `started_at` / `finished_at`. The two-tier split was chosen specifically to make this extension non-breaking: existing `JobSpec` consumers stay untouched.
- **`SlurmJobId` newtype + summary file**: shape will be `pub struct SlurmJobId(pub String);` plus a sibling `<work_dir>/summary.toml` keyed by `JobId` (per step 6 of §4.1).
- **Submission helpers on `JobFlow`**: `JobFlow::find(&JobId)`, `JobFlow::roots()`, `JobFlow::topological()` — likely live in TaskManager rather than directly on `JobFlow` to keep the data type pure.
- **Ergonomic access (`Job` → `JobSpec` fields)**: `job.spec.config` is verbose. Decision deferred — we can add `impl Deref<Target = JobSpec> for Job` (so `job.config` works) once usage shows it's worth the indirection.
- **`delay_minutes` on `JobEdge`**: defer until a real After-with-delay use-case appears.
- **TOML round-trip for `JobFlow`**: the canonical filename, schema, and `read_*` / `write_*` helpers are TaskManager / Metadata-layer concerns.
- **`calc_type` placement**: kept at `JobFlow` level for now (it describes the overall flow purpose). If we ever want stage-level calc kinds (e.g. main = "opt", post = "fchk-extract"), we can add a `Job.role` field as a follow-up rather than moving `calc_type`.
- **Generic-over-program JobFlow**: 5b (params/compounds outside JobFlow) is the chosen approach. If many programs each carry rich metadata, consider a side-by-side `JobFlowEnvelope<P>` wrapper at the metadata layer rather than introducing generics on `JobFlow` itself.
- **`Program` as enum**: currently a newtype String for openness. If a closed set ever becomes desirable, a follow-up PR can introduce `enum Program { Gaussian, Other(String) }` with a custom serde impl.
- **`JobLifecycleStatus` transition logic**: when adding the status-tracking layer, port Python's `_decide_transition`; this PR ships the data type only.
