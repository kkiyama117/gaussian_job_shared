# JobLifecycleStatus expansion: SLURM official state coverage + sub-state nesting

**Date:** 2026-05-08
**Branch:** `expand-status`
**Crate:** `gaussian_job_shared`
**Driving consumer:** `kkiyama117/slurm-async-runner` (S3 of the
`miyake-ken/slurm-async-runner@slurm-gaussian-migration` plan, see
`slurm-async-runner2/docs/superpowers/specs/2026-05-08-slurm-gaussian-migration-design.md`)

## 1. Goal

Expand `entities::workflow::status::JobLifecycleStatus` so that:

- **It is the only state type the wider workspace needs.** The migration
  spec previously planned to introduce a separate
  `SlurmJobState` enum in `slurm-async-runner` and re-export it here in
  S7+S8. With this expansion the separate enum is unnecessary —
  `slurm-async-runner` consumes `JobLifecycleStatus` directly.
- **It complies with the official Slurm job state codes** documented at
  <https://slurm.schedmd.com/squeue.html#JOB_STATE_CODES>. All 24
  official tokens parse to a defined variant; unknown tokens fall back
  to `Unknown`.
- **Symmetric sub-state nesting** lets workflow consumers either keep
  the coarse 5-category view (`Queued / Running / Done / Failed /
  Unknown`) or drill into the SLURM-specific kind via the inner enum.

## 2. Non-Goals

- Introducing a separate `SlurmJobState` type. The expansion makes one
  unnecessary.
- Changing `StatusEntry`. It continues to wrap `JobLifecycleStatus` +
  `transitioned_at`. (Only the inner enum gets richer.)
- Defining the `parse_squeue` / `parse_sacct` line parsers. Those stay
  in `slurm-async-runner::runner::query` because they handle row-level
  layout; this crate only provides the per-token state parser.
- Modeling preemption / requeue *transitions* as an FSM. We model the
  *current state observed by a SLURM query*, not the path between
  states.

## 3. Categorization (official SLURM state codes → variants)

Each variant carries the SLURM long-form token name verbatim (e.g.
`OutOfMemory` for `OUT_OF_MEMORY`). Compact codes (`PD`, `R`, `CG`, …)
are accepted at parse time but normalized to long form on serialization.

### 3.1 `Queued` — allocation not yet driving forward progress

| SLURM long form | Code | Meaning |
|-----------------|------|---------|
| `PENDING` | `PD` | Awaiting resource allocation. |
| `CONFIGURING` | `CF` | Allocated, resources not yet ready (booting). |
| `REQUEUED` | `RQ` | Completing job is being requeued. |
| `REQUEUE_FED` | `RF` | Being requeued by a federation. |
| `REQUEUE_HOLD` | `RH` | Held requeue. |
| `RESV_DEL_HOLD` | `RD` | Held because requested reservation was deleted. |
| `SUSPENDED` | `S` | Execution suspended; CPUs released to other jobs. |
| `STOPPED` | `ST` | Execution stopped via SIGSTOP; CPUs retained. |

`SUSPENDED` and `STOPPED` are paused-execution states. From a workflow
perspective they share the property "not making forward progress and
not yet terminal", so they live alongside `PENDING`. Consumers that
need to distinguish "have not started" from "started and paused" do so
via the inner `QueuedKind`.

### 3.2 `Running` — allocation alive and progressing (or transitioning while alive)

| SLURM long form | Code | Meaning |
|-----------------|------|---------|
| `RUNNING` | `R` | Has allocation, executing. |
| `COMPLETING` | `CG` | Finishing; some processes on some nodes still active (epilog, FS flush). |
| `RESIZING` | `RS` | About to change size. |
| `SIGNALING` | `SI` | Being signaled. |
| `STAGE_OUT` | `SO` | Burst-buffer stage-out in progress. |

`COMPLETING` is non-terminal because the allocation is still held (the
caller cannot reuse those nodes yet). It transitions to `COMPLETED` on
success or to a `Failed(_)` variant on epilog failure.

### 3.3 `Done` — terminal success (no sub-state)

| SLURM long form | Code | Meaning |
|-----------------|------|---------|
| `COMPLETED` | `CD` | All processes on all nodes exited 0. |

There is exactly one terminal-success token, so `Done` is a unit
variant.

### 3.4 `Failed(FailureKind)` — terminal failure with cause

| SLURM long form | Code | Meaning |
|-----------------|------|---------|
| `BOOT_FAIL` | `BF` | Launch failed (typically hardware). |
| `CANCELLED` | `CA` | Explicit cancel by user / admin. May or may not have started. |
| `DEADLINE` | `DL` | Reached deadline before completion. |
| `FAILED` | `F` | Non-zero exit code or other failure. |
| `NODE_FAIL` | `NF` | Allocated node(s) failed. |
| `OUT_OF_MEMORY` | `OOM` | OOM killed. |
| `PREEMPTED` | `PR` | Preempted by a higher-priority job. |
| `REVOKED` | `RV` | Federation: sibling cluster started the job. |
| `SPECIAL_EXIT` | `SE` | Configured "special" requeue/exit (admin epilog). |
| `TIMEOUT` | `TO` | Wall-time limit reached. |

### 3.5 `Unknown` — sentinel

Used when:
- the caller queried `squeue` + `sacct` for a jobid neither command
  reported (the contract `slurm-async-runner` already relies on);
- the parsed token is none of the 24 official strings (forward-compat
  for SLURM versions adding new states).

## 4. Type definition

```rust
//! src/entities/workflow/status.rs

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Lifecycle of a Job from a workflow perspective, with SLURM-grade
/// sub-state where it carries actionable information for the caller.
///
/// Variants follow the official Slurm job state codes — see
/// <https://slurm.schedmd.com/squeue.html#JOB_STATE_CODES>. Tokens not
/// matching any official long-form name parse to `Unknown` so consumers
/// stay forward-compatible across SLURM versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobLifecycleStatus {
    Queued(QueuedKind),
    Running(RunningKind),
    Done,
    Failed(FailureKind),
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueuedKind {
    Pending,
    Configuring,
    Requeued,
    RequeueFed,
    RequeueHold,
    ResvDelHold,
    Suspended,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RunningKind {
    Running,
    Completing,
    Resizing,
    Signaling,
    StageOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FailureKind {
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

/// One entry in a Job's status history: a (status, timestamp) pair.
/// **Unchanged** from the previous design — only the embedded enum got
/// richer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusEntry {
    pub status: JobLifecycleStatus,
    pub transitioned_at: chrono::DateTime<chrono::Utc>,
}
```

## 5. Parsing

```rust
impl JobLifecycleStatus {
    /// Parse a raw squeue/sacct state token into a structured status.
    ///
    /// Accepts:
    /// - SLURM long forms (`"PENDING"`, `"OUT_OF_MEMORY"`, `"NODE_FAIL"`, …)
    /// - SLURM compact codes (`"PD"`, `"OOM"`, `"NF"`, …)
    /// - Trailing context (`"CANCELLED by 1234"` → first whitespace-
    ///   separated token wins)
    /// - The 4 legacy workflow tokens (`"queued"`, `"running"`, `"done"`,
    ///   `"failed"`) for back-compat with already-serialized
    ///   `JobLifecycleStatus` data; these map to the canonical first
    ///   variant of each category (`Queued(Pending)`, `Running(Running)`,
    ///   `Done`, `Failed(Failed)`).
    /// - Case-insensitive matching across the board.
    ///
    /// Falls back to `Unknown` for any other input (including empty
    /// string) so the contract "every queried jobid maps to a defined
    /// variant" holds across SLURM upgrades.
    pub fn parse(raw: &str) -> Self {
        let token = raw.split_whitespace().next().unwrap_or("");
        match token.to_ascii_uppercase().as_str() {
            // -------- Queued --------
            "PENDING" | "PD" | "QUEUED"
                                 => Self::Queued(QueuedKind::Pending),
            "CONFIGURING" | "CF" => Self::Queued(QueuedKind::Configuring),
            "REQUEUED" | "RQ"    => Self::Queued(QueuedKind::Requeued),
            "REQUEUE_FED" | "RF" => Self::Queued(QueuedKind::RequeueFed),
            "REQUEUE_HOLD" | "RH" => Self::Queued(QueuedKind::RequeueHold),
            "RESV_DEL_HOLD" | "RD" => Self::Queued(QueuedKind::ResvDelHold),
            "SUSPENDED" | "S"    => Self::Queued(QueuedKind::Suspended),
            "STOPPED" | "ST"     => Self::Queued(QueuedKind::Stopped),

            // -------- Running --------
            "RUNNING" | "R"      => Self::Running(RunningKind::Running),
            "COMPLETING" | "CG"  => Self::Running(RunningKind::Completing),
            "RESIZING" | "RS"    => Self::Running(RunningKind::Resizing),
            "SIGNALING" | "SI"   => Self::Running(RunningKind::Signaling),
            "STAGE_OUT" | "SO"   => Self::Running(RunningKind::StageOut),

            // -------- Done --------
            "COMPLETED" | "CD" | "DONE" => Self::Done,

            // -------- Failed --------
            "BOOT_FAIL" | "BF"   => Self::Failed(FailureKind::BootFail),
            "CANCELLED" | "CA"   => Self::Failed(FailureKind::Cancelled),
            "DEADLINE" | "DL"    => Self::Failed(FailureKind::Deadline),
            "FAILED" | "F"       => Self::Failed(FailureKind::Failed),
            "NODE_FAIL" | "NF"   => Self::Failed(FailureKind::NodeFail),
            "OUT_OF_MEMORY" | "OOM" => Self::Failed(FailureKind::OutOfMemory),
            "PREEMPTED" | "PR"   => Self::Failed(FailureKind::Preempted),
            "REVOKED" | "RV"     => Self::Failed(FailureKind::Revoked),
            "SPECIAL_EXIT" | "SE" => Self::Failed(FailureKind::SpecialExit),
            "TIMEOUT" | "TO"     => Self::Failed(FailureKind::Timeout),

            _ => Self::Unknown,
        }
    }

    /// SLURM long-form token for this state. Round-trips `parse(...)`
    /// for every variant.
    pub fn as_token(&self) -> &'static str {
        match self {
            Self::Queued(QueuedKind::Pending) => "PENDING",
            Self::Queued(QueuedKind::Configuring) => "CONFIGURING",
            Self::Queued(QueuedKind::Requeued) => "REQUEUED",
            Self::Queued(QueuedKind::RequeueFed) => "REQUEUE_FED",
            Self::Queued(QueuedKind::RequeueHold) => "REQUEUE_HOLD",
            Self::Queued(QueuedKind::ResvDelHold) => "RESV_DEL_HOLD",
            Self::Queued(QueuedKind::Suspended) => "SUSPENDED",
            Self::Queued(QueuedKind::Stopped) => "STOPPED",

            Self::Running(RunningKind::Running) => "RUNNING",
            Self::Running(RunningKind::Completing) => "COMPLETING",
            Self::Running(RunningKind::Resizing) => "RESIZING",
            Self::Running(RunningKind::Signaling) => "SIGNALING",
            Self::Running(RunningKind::StageOut) => "STAGE_OUT",

            Self::Done => "COMPLETED",

            Self::Failed(FailureKind::BootFail) => "BOOT_FAIL",
            Self::Failed(FailureKind::Cancelled) => "CANCELLED",
            Self::Failed(FailureKind::Deadline) => "DEADLINE",
            Self::Failed(FailureKind::Failed) => "FAILED",
            Self::Failed(FailureKind::NodeFail) => "NODE_FAIL",
            Self::Failed(FailureKind::OutOfMemory) => "OUT_OF_MEMORY",
            Self::Failed(FailureKind::Preempted) => "PREEMPTED",
            Self::Failed(FailureKind::Revoked) => "REVOKED",
            Self::Failed(FailureKind::SpecialExit) => "SPECIAL_EXIT",
            Self::Failed(FailureKind::Timeout) => "TIMEOUT",

            Self::Unknown => "UNKNOWN",
        }
    }
}
```

## 6. Serde

The previous `#[serde(rename_all = "lowercase")]` attribute is removed
in favor of an explicit string round-trip. Stored data continues to
look like `status = "pending"` (lowercase TOML), and the parser is
case-insensitive so both lowercase and uppercase serialized forms read
back cleanly.

```rust
impl Serialize for JobLifecycleStatus {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.as_token().to_ascii_lowercase())
    }
}

impl<'de> Deserialize<'de> for JobLifecycleStatus {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Ok(Self::parse(&s))
    }
}
```

### 6.1 Backward compatibility for existing serialized data

The four existing workflow tokens `"queued"`, `"running"`, `"done"`,
`"failed"` continue to deserialize cleanly:

- `"queued"` → `Queued(Pending)` (canonical first variant)
- `"running"` → `Running(Running)`
- `"done"` → `Done`
- `"failed"` → `Failed(Failed)`

This means already-persisted `StatusEntry` values keep working with no
migration step. New writes use the SLURM-specific token (`"pending"` /
`"completing"` / `"cancelled"` / …) so the persisted form gradually
becomes more precise without a hard cutover.

Example `StatusEntry` TOML before/after expansion:

```toml
# Before — only 4 variants available
[entry]
status = "running"
transitioned_at = 2026-05-08T12:00:00Z

# After — SLURM-precise tokens supported
[entry]
status = "completing"
transitioned_at = 2026-05-08T12:00:30Z
```

## 7. PyO3 surface

The current `PyJobLifecycleStatus` is a flat `#[pyclass(eq_int, hash,
frozen)]` enum with 4 unit variants. The richer Rust enum cannot be
expressed as a flat pyo3 enum (tuple-variants are unsupported in that
form), so the wrapper migrates to the **`ArrayIndex` pattern** already
used elsewhere in this crate:

- `PyJobLifecycleStatus`: newtype `pub struct PyJobLifecycleStatus(pub
  inner::JobLifecycleStatus)` carrying the inner Rust enum.
  - Static factories: `queued(kind)`, `running(kind)`, `done()`,
    `failed(kind)`, `unknown()`.
  - `kind()` getter returns the discriminant string (`"queued"`,
    `"running"`, `"done"`, `"failed"`, `"unknown"`) — matches the
    `ArrayIndex.kind` pattern.
  - `queued_kind() -> Option<PyQueuedKind>`,
    `running_kind() -> Option<PyRunningKind>`,
    `failure_kind() -> Option<PyFailureKind>` getters return `Some(_)`
    only for the relevant outer variant.
  - `token()` getter returns the SLURM long-form string (delegates to
    `inner::as_token`).
  - `parse(raw: &str)` static method delegates to
    `inner::JobLifecycleStatus::parse`.
  - Implements `__str__` (lowercase token) and `__repr__`.

- `PyQueuedKind` / `PyRunningKind` / `PyFailureKind`: flat
  `#[pyclass(eq, eq_int, hash, frozen)]` enums identical in shape to
  the old `PyJobLifecycleStatus`. Each carries `__str__` / `__repr__`
  returning the SLURM token (lowercased for `__str__`, PascalCase for
  `__repr__`).

Module path stays `gaussian_job_shared._core.entities.workflow`. The
new sub-enums register alongside `PyJobLifecycleStatus` /
`PyStatusEntry` in `py_export::entities::workflow::mod.rs`.

### 7.1 Python ergonomics

```python
from gaussian_job_shared._core.entities.workflow import (
    JobLifecycleStatus,
    QueuedKind,
    RunningKind,
    FailureKind,
)

s = JobLifecycleStatus.parse("CANCELLED by 1234")
assert s.kind == "failed"
assert s.failure_kind() == FailureKind.Cancelled
assert s.token == "CANCELLED"
```

Pattern-matching uses the `kind` discriminant + sub-getter combination
because Python lacks tagged-union pattern matching. This matches how
`ArrayIndex` is consumed today.

### 7.2 Breaking change notice

Existing consumers that compared `PyJobLifecycleStatus.Queued ==
my_status` (flat enum equality) must migrate to
`my_status.kind == "queued"` or
`my_status.queued_kind() is QueuedKind.Pending`. Listed up front in
the CHANGELOG entry produced by this branch.

## 8. Tests

### 8.1 New `cargo test` cases (in `entities/workflow/status.rs`)

| Group | Cases |
|-------|-------|
| `parse` long forms | One assert per of the 24 SLURM tokens → expected variant. |
| `parse` compact codes | One assert per of the 24 compact codes → same variant as the long form. |
| `parse` trailing context | `"CANCELLED by 1234"` → `Failed(Cancelled)`; `"  RUNNING  "` → `Running(Running)`. |
| `parse` legacy workflow tokens | `"queued"` → `Queued(Pending)`; `"done"` → `Done`. |
| `parse` unknown / empty | `""`, `"FOO"`, `"  "` → `Unknown`. |
| `as_token` round-trip | For each variant, `parse(s.as_token())` returns the same `s`. |
| TOML round-trip | Replace the old `job_lifecycle_status_roundtrip_all_variants` with one that iterates over **all** sub-variants (24 + Unknown) and verifies serde round-trip via TOML. |
| TOML lowercase invariant | `Holder { status: Failed(OutOfMemory) }` serializes to `status = "out_of_memory"`. |
| StatusEntry round-trip | Existing test stays — just retarget to a richer status (`Failed(Timeout)` instead of `Running`). |

### 8.2 New PyO3 tests (under `python/tests/`, if any)

If the repo already has Python tests for `PyJobLifecycleStatus`, port
them to the new wrapper-struct shape and add coverage for `parse` /
`failure_kind()` / `kind`. If not, defer Python-level coverage to the
downstream `slurm-async-runner` package whose pytest suite is the main
consumer.

## 9. Files Touched

| Path | Change |
|------|--------|
| `src/entities/workflow/status.rs` | Replace inner enum, add `QueuedKind` / `RunningKind` / `FailureKind`, add `impl` block (`parse`, `as_token`), replace serde derive with manual `Serialize`/`Deserialize`, update tests. |
| `src/py_export/entities/workflow/status.rs` | Replace `PyJobLifecycleStatus` flat enum with newtype wrapper + factories + getters; add `PyQueuedKind` / `PyRunningKind` / `PyFailureKind`. |
| `src/py_export/entities/workflow/mod.rs` | Register the three new sub-enums alongside `PyJobLifecycleStatus`. |
| `python/gaussian_job_shared/_core/entities/workflow/__init__.pyi` | Regenerate via `cargo run --bin stub_gen --features stub_gen` and commit. |
| `CHANGELOG.md` | Add an entry noting the breaking change to `PyJobLifecycleStatus`. |

## 10. Acceptance Criteria

- `cargo test` ✓ including all parse / round-trip cases above.
- `cargo clippy --all-targets -- -D warnings` ✓.
- `maturin develop` ✓ in `gaussian-job-shared2`.
- The downstream `slurm-async-runner` repo can `cargo update -p
  gaussian_job_shared` and successfully replace its planned local
  `SlurmJobState` enum with `pub use
  gaussian_job_shared::entities::workflow::status::JobLifecycleStatus;`
  (verified by a follow-up dry-run from `slurm-async-runner2`).

## 11. Migration Path for `slurm-async-runner`

After this branch lands and is merged into `main`, the downstream
`slurm-async-runner` migration spec collapses S3, S7, and S8 into a
single S3':

> **S3' (revised)** — *port query result type to
> `gaussian_job_shared::entities::workflow::status::JobLifecycleStatus`.*
>
> - bump `gaussian_job_shared` Cargo dep to a `rev` covering this
>   branch's merge commit;
> - drop the planned local `SlurmJobState` enum and its parser
>   (`slurm_async_runner::entities::slurm_job_state` is never created);
> - `runner::query::parse_squeue` / `parse_sacct` use
>   `JobLifecycleStatus::parse(state_token)` directly;
> - the Python surface re-exports `JobLifecycleStatus` and friends
>   from this crate's pyo3 module, keeping
>   `from slurm_async_runner import JobLifecycleStatus` available;
> - the existing migration spec's S7 (move enum to gaussian_job_shared)
>   and S8 (re-export from slurm-async-runner) are obsolete — already
>   covered here.

## 12. Open Questions

- **Naming of the legacy `Failed` workflow token's deserialized
  variant.** Currently mapped to `Failed(Failed)` (the SLURM `FAILED`
  generic). Alternative: `Failed(Cancelled)` if existing data semantically
  meant "canceled by user". Consensus needed if real persisted data
  exists; default is `Failed(Failed)`.
- **Compact-code parsing scope.** The spec accepts compact codes (`PD`,
  `OOM`, …) for completeness, but `slurm-async-runner` currently runs
  `squeue -o "%T"` which always emits long forms. If we end up never
  needing compact-code parsing in production, the parser can shed those
  branches in a later cleanup.
