# Job and JobFlow Struct Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the Rust data types that represent a Slurm batch job (`JobSpec` / `Job`) and a job flow (`JobFlow`) as specified in `docs/superpowers/specs/2026-05-08-slurm-job-flow-structs-design.md`.

**Architecture:** Two-tier Job design — `JobSpec` (small / state-independent: `program`, `config`, `body`) and `Job` (large / in-flow: wraps `JobSpec` via `#[serde(flatten)]`, adds `parents: Vec<JobEdge>`). `JobFlow` stores its DAG as `BTreeMap<JobId, Job>` for `O(log N)` lookup and structural ID uniqueness. All types are pure data — no validation, no submit/tick logic, no pyo3 bindings.

**Tech Stack:** Rust 2024 edition, `serde` (derive + flatten), `chrono` (DateTime<Utc>), `toml` 1.x for round-trip tests, `uuid` v7. Tests live as `#[cfg(test)] mod tests { … }` at the end of each file (matches existing pattern in `src/entities/slurm/array_spec.rs`, `dependency.rs`, etc.).

**Reference spec:** `docs/superpowers/specs/2026-05-08-slurm-job-flow-structs-design.md` (commit `eb5d487`). Read §5 (type specifications) before starting Task 1.

---

## Repository Background

- Crate root: `src/lib.rs` (this is the `gaussian_job_shared` crate)
- Existing module layout: `src/entities.rs` → `src/entities/slurm.rs` → `src/entities/slurm/{array_spec,dependency,resource_spec,time_limit}.rs`
- Existing test pattern (mimic this): wrap the type under test in a `Holder` struct in `#[cfg(test)] mod tests`, then `toml::to_string` / `toml::from_str` round-trip. See bottom of `src/entities/slurm/array_spec.rs` for the canonical example.
- Test command throughout this plan: `cargo test --lib --no-default-features <pattern>` — `--no-default-features` skips the pyo3 / stub_gen features so we don't need Python in the loop.

## Pre-existing Build Issue (fixed in Task 0)

`Cargo.toml` declares `[[bin]] name = "stub_gen"` but `src/bin/stub_gen.rs` is missing, so even `cargo check --lib` fails with a manifest parse error. Task 0 creates a minimal stub_gen entry point (one-liner that calls the existing `gaussian_job_shared::stub_info()`).

## Files Touched

| Path | Action | Owner |
|------|--------|-------|
| `src/bin/stub_gen.rs` | **create** | Task 0 |
| `src/entities/slurm/job.rs` | **create** | Tasks 1–5 |
| `src/entities/slurm.rs` | modify (add `pub mod job;` + re-exports) | Task 6 |
| `src/entities/slurm/status.rs` | **create** | Tasks 7–8 |
| `src/entities/slurm.rs` | modify (add `pub mod status;` + re-exports) | Task 9 |
| `src/entities/job_flow.rs` | **create** | Tasks 10–11 |
| `src/entities.rs` | modify (add `pub mod job_flow;` + re-exports) | Task 12 |

---

## Task 0: Unblock the build with a minimal `stub_gen` entry point

**Files:**
- Create: `src/bin/stub_gen.rs`

- [ ] **Step 1: Verify the failure mode**

Run: `cargo check --lib --no-default-features`
Expected: error `can't find 'stub_gen' bin at 'src/bin/stub_gen.rs'`. If you instead get "no errors", skip Task 0 entirely.

- [ ] **Step 2: Create the bin file**

Create `src/bin/stub_gen.rs` with exactly:

```rust
//! pyo3-stub-gen entry point. Re-exposes `gaussian_job_shared::stub_info`
//! (defined in `src/py_export/mod.rs` via `define_stub_info_gatherer!`).
//! Only built when the `stub_gen` feature is on.

fn main() -> pyo3_stub_gen::Result<()> {
    let stub = gaussian_job_shared::stub_info()?;
    stub.generate()?;
    Ok(())
}
```

- [ ] **Step 3: Verify the build is unblocked**

Run: `cargo check --lib --no-default-features`
Expected: success (no errors, possibly some warnings).

Run: `cargo test --lib --no-default-features` (no tests yet, just verifies the harness works)
Expected: 0 tests pass / 0 fail / build succeeds.

- [ ] **Step 4: Commit**

```bash
git add src/bin/stub_gen.rs
git commit -m "build: add minimal stub_gen bin entry point so cargo check works

The [[bin]] declaration in Cargo.toml referenced src/bin/stub_gen.rs
which did not exist, causing 'cargo check --lib' to fail at the
manifest level. This adds a one-line main() that delegates to the
existing gaussian_job_shared::stub_info() defined under the pyo3
feature, matching the pyo3-stub-gen convention."
```

---

## Task 1: `JobId` newtype

**Files:**
- Create: `src/entities/slurm/job.rs`
- Modify: `src/entities/slurm.rs` (declare `pub mod job;`)

- [ ] **Step 1: Write the failing test**

Create `src/entities/slurm/job.rs` with this initial content (test only — no impl yet, intentional fail):

```rust
//! Job, JobSpec, JobEdge, JobId, Program — the in-flow Slurm Job tier.
//! See `docs/superpowers/specs/2026-05-08-slurm-job-flow-structs-design.md`
//! §5.2.

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Holder {
        id: JobId,
    }

    #[test]
    fn job_id_toml_roundtrip() {
        let original = Holder {
            id: JobId("g16".to_string()),
        };
        let s = toml::to_string(&original).unwrap();
        assert!(s.contains(r#"id = "g16""#), "actual TOML: {s}");
        let back: Holder = toml::from_str(&s).unwrap();
        assert_eq!(back, original);
    }

    #[test]
    fn job_id_ord_supports_btreemap_key() {
        let mut m = std::collections::BTreeMap::new();
        m.insert(JobId("post".to_string()), 2);
        m.insert(JobId("g16".to_string()), 1);
        let keys: Vec<_> = m.keys().cloned().collect();
        // BTreeMap iterates in key order: "g16" < "post"
        assert_eq!(keys, vec![JobId("g16".to_string()), JobId("post".to_string())]);
    }
}
```

Then wire the new module into the parent so `cargo test` sees it. Modify `src/entities/slurm.rs` to add right above the existing `pub mod array_spec;` line:

```rust
pub mod job;
```

(No re-exports yet — that's Task 6.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib --no-default-features job_id`
Expected: compile error `cannot find type 'JobId' in this scope` (test mod references `JobId` which is not yet defined).

- [ ] **Step 3: Write minimal implementation**

In `src/entities/slurm/job.rs`, immediately after the `use serde::{Deserialize, Serialize};` line (before `#[cfg(test)]`), add:

```rust
/// Stable ID of a `Job` within a `JobFlow`. Used as the map key in
/// `JobFlow.jobs: BTreeMap<JobId, Job>` and as bash-filename / log-prefix
/// stem. Derives `Ord` because it is a `BTreeMap` key.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JobId(pub String);

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for JobId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for JobId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib --no-default-features job_id`
Expected: 2 tests pass (`job_id_toml_roundtrip`, `job_id_ord_supports_btreemap_key`).

- [ ] **Step 5: Commit**

```bash
git add src/entities/slurm/job.rs src/entities/slurm.rs
git commit -m "feat: add JobId newtype (entities/slurm/job.rs)

Stable, flow-scoped identifier used as the BTreeMap key in
JobFlow.jobs. Derives Ord (required for BTreeMap), uses
serde(transparent) so the TOML/JSON form is the bare string.
Display + From<String> + From<&str> for ergonomic construction."
```

---

## Task 2: `Program` newtype

**Files:**
- Modify: `src/entities/slurm/job.rs`

- [ ] **Step 1: Write the failing test**

Append to the `#[cfg(test)] mod tests` block in `src/entities/slurm/job.rs`:

```rust
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct ProgramHolder {
        program: Program,
    }

    #[test]
    fn program_toml_roundtrip() {
        let h = ProgramHolder {
            program: Program("g16".to_string()),
        };
        let s = toml::to_string(&h).unwrap();
        assert!(s.contains(r#"program = "g16""#), "actual TOML: {s}");
        let back: ProgramHolder = toml::from_str(&s).unwrap();
        assert_eq!(back, h);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib --no-default-features program_toml_roundtrip`
Expected: compile error `cannot find type 'Program' in this scope`.

- [ ] **Step 3: Write minimal implementation**

Add to `src/entities/slurm/job.rs`, right after the `JobId` impls:

```rust
/// Program identifier a `JobSpec` runs (e.g. "g16", "formchk",
/// "gaussview", program-specific analyzers).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Program(pub String);

impl std::fmt::Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for Program {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Program {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib --no-default-features program_toml_roundtrip`
Expected: 1 test passes.

- [ ] **Step 5: Commit**

```bash
git add src/entities/slurm/job.rs
git commit -m "feat: add Program newtype (entities/slurm/job.rs)

String-newtype identifier for the program a JobSpec runs.
Same shape as JobId (transparent serde, Display, From conversions)."
```

---

## Task 3: `JobEdge` struct

**Files:**
- Modify: `src/entities/slurm/job.rs`

- [ ] **Step 1: Verify the existing `DependencyType` variants**

Run: `grep -nE 'pub enum DependencyType|^\s+[A-Z][a-zA-Z]+\s*,' src/entities/slurm/dependency.rs | head -15`

Note the exact variant names (e.g. `AfterOk` vs `Afterok`). Use whatever spelling exists in the file in the test code below — the spec uses `Afterok` style but the codebase may use `AfterOk`. Substitute as needed in **all** subsequent tasks too.

- [ ] **Step 2: Write the failing test**

Append to the `tests` mod (replace `DependencyType::AfterOk` / `DependencyType::After` with the actual variant names from Step 1):

```rust
    use super::super::dependency::DependencyType;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct EdgeHolder {
        edge: JobEdge,
    }

    #[test]
    fn job_edge_toml_roundtrip_afterok() {
        let h = EdgeHolder {
            edge: JobEdge {
                from: JobId::from("g16"),
                kind: DependencyType::AfterOk,
            },
        };
        let s = toml::to_string(&h).unwrap();
        let back: EdgeHolder = toml::from_str(&s).unwrap();
        assert_eq!(back, h);
    }

    #[test]
    fn job_edge_toml_roundtrip_after() {
        let h = EdgeHolder {
            edge: JobEdge {
                from: JobId::from("upstream"),
                kind: DependencyType::After,
            },
        };
        let s = toml::to_string(&h).unwrap();
        let back: EdgeHolder = toml::from_str(&s).unwrap();
        assert_eq!(back, h);
    }
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test --lib --no-default-features job_edge_toml_roundtrip`
Expected: compile error `cannot find struct 'JobEdge'`.

- [ ] **Step 4: Write minimal implementation**

At the top of `src/entities/slurm/job.rs`, replace `use serde::{Deserialize, Serialize};` with:

```rust
use serde::{Deserialize, Serialize};

use super::dependency::DependencyType;
```

After the `Program` block, add:

```rust
/// Intra-flow dependency edge — incoming to the enclosing `Job`.
/// `to` is implicit (= the map key of the enclosing `JobFlow.jobs` entry).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobEdge {
    /// Parent (predecessor) — key into the enclosing `JobFlow.jobs`.
    pub from: JobId,

    /// Slurm dependency kind (Afterok / Afterany / After / ...).
    pub kind: DependencyType,
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test --lib --no-default-features job_edge_toml_roundtrip`
Expected: 2 tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/entities/slurm/job.rs
git commit -m "feat: add JobEdge struct (entities/slurm/job.rs)

Intra-flow dependency edge: { from: JobId, kind: DependencyType }.
'to' is implicit (= the enclosing Job's map key in JobFlow.jobs).
deny_unknown_fields to catch typos at deserialize time."
```

---

## Task 4: `JobSpec` struct

**Files:**
- Modify: `src/entities/slurm/job.rs`
- Possibly modify: `src/entities/slurm.rs` (only if `SlurmJobConfig` does not already derive `PartialEq`)

- [ ] **Step 1: Check if `SlurmJobConfig` derives `PartialEq`**

Run: `grep -B1 'pub struct SlurmJobConfig' src/entities/slurm.rs`

If the derive line includes `PartialEq`, skip Step 4a below. Otherwise plan to add `PartialEq` to that derive line.

- [ ] **Step 2: Write the failing test**

Append to the `tests` mod:

```rust
    use super::super::SlurmJobConfig;

    fn sample_config() -> SlurmJobConfig {
        SlurmJobConfig {
            partition: "long".to_string(),
            time_limit: None,
            log_stdout: None,
            log_stderr: None,
            comment: None,
            job_name: None,
            array_spec: None,
            dependency: None,
            mail_user: None,
            mail_types: None,
            resource_spec: None,
        }
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct SpecHolder {
        spec: JobSpec,
    }

    #[test]
    fn job_spec_toml_roundtrip() {
        let h = SpecHolder {
            spec: JobSpec {
                program: Program::from("g16"),
                config: sample_config(),
                body: "g16 < input.gjf > output.log\n".to_string(),
            },
        };
        let s = toml::to_string(&h).unwrap();
        let back: SpecHolder = toml::from_str(&s).unwrap();
        assert_eq!(back, h);
    }

    #[test]
    fn job_spec_is_state_independent_can_be_cloned() {
        let original = JobSpec {
            program: Program::from("g16"),
            config: sample_config(),
            body: "echo hi\n".to_string(),
        };
        let copy = original.clone();
        assert_eq!(original, copy);
    }
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test --lib --no-default-features job_spec_`
Expected: compile error `cannot find struct 'JobSpec'`. (Or, if `SlurmJobConfig` lacks `PartialEq`, an error about that — addressed in Step 4a.)

- [ ] **Step 4a: (only if Step 1 found `SlurmJobConfig` lacks `PartialEq`)**

In `src/entities/slurm.rs`, find the line `#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]` that sits directly above `pub struct SlurmJobConfig` and append `, PartialEq`:

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
```

If the inner Slurm types it uses (`JobTimeLimit`, `SlurmArraySpec`, `SlurmDependency`, `MailTypeInput`, `ResourceSpec`) lack `PartialEq` and the build fails, add `PartialEq` to their derive lines too. Each is a one-token addition; do **not** rewrite the structs.

- [ ] **Step 4b: Write `JobSpec`**

Add to `src/entities/slurm/job.rs`, after the `JobEdge` block:

```rust
/// SMALL tier: state-independent / pre-runtime work definition.
/// Reusable across flows — carries no flow-scoped or runtime state.
/// See `docs/superpowers/specs/2026-05-08-slurm-job-flow-structs-design.md` §5.2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobSpec {
    /// Program identifier this stage runs.
    pub program: Program,

    /// Slurm submission directives. TaskManager produces this by merging
    /// cluster-wide defaults with per-job overrides — by the time it
    /// lands in `JobSpec` it is already complete.
    pub config: super::SlurmJobConfig,

    /// Bash script body (text *after* the `#SBATCH` directive block).
    pub body: String,
}
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test --lib --no-default-features job_spec_`
Expected: 2 tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/entities/slurm/job.rs src/entities/slurm.rs
git commit -m "feat: add JobSpec struct (entities/slurm/job.rs)

JobSpec is the SMALL tier of the two-tier Job design (spec §5.2):
state-independent work definition with program, config, body —
no flow-scoped or runtime fields. Reusable across flows.

Adds PartialEq derive to SlurmJobConfig (and any of its inner types
that lacked it) to support assert_eq! in JobSpec round-trip tests."
```

---

## Task 5: `Job` struct (with `#[serde(flatten)]`)

**Files:**
- Modify: `src/entities/slurm/job.rs`

- [ ] **Step 1: Write the failing test**

Append to the `tests` mod:

```rust
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct JobHolder {
        job: Job,
    }

    #[test]
    fn job_no_id_field_flatten_produces_flat_toml() {
        // Verifies: there is NO `[job.spec]` nesting — flatten makes
        // program/config/body siblings of parents.
        let h = JobHolder {
            job: Job {
                spec: JobSpec {
                    program: Program::from("g16"),
                    config: sample_config(),
                    body: "echo hi\n".to_string(),
                },
                parents: vec![],
            },
        };
        let s = toml::to_string(&h).unwrap();
        assert!(s.contains("program = \"g16\""), "actual TOML: {s}");
        assert!(!s.contains("[job.spec]"), "spec wrapper leaked into TOML: {s}");
    }

    #[test]
    fn job_root_has_empty_parents() {
        let h = JobHolder {
            job: Job {
                spec: JobSpec {
                    program: Program::from("g16"),
                    config: sample_config(),
                    body: String::new(),
                },
                parents: vec![],
            },
        };
        let s = toml::to_string(&h).unwrap();
        let back: JobHolder = toml::from_str(&s).unwrap();
        assert!(back.job.parents.is_empty());
    }

    #[test]
    fn job_with_one_parent() {
        let h = JobHolder {
            job: Job {
                spec: JobSpec {
                    program: Program::from("formchk"),
                    config: sample_config(),
                    body: String::new(),
                },
                parents: vec![JobEdge {
                    from: JobId::from("g16"),
                    kind: DependencyType::AfterOk,
                }],
            },
        };
        let s = toml::to_string(&h).unwrap();
        let back: JobHolder = toml::from_str(&s).unwrap();
        assert_eq!(back.job.parents.len(), 1);
        assert_eq!(back.job.parents[0].from, JobId::from("g16"));
    }

    #[test]
    fn job_with_dag_join_two_parents() {
        let h = JobHolder {
            job: Job {
                spec: JobSpec {
                    program: Program::from("merge"),
                    config: sample_config(),
                    body: String::new(),
                },
                parents: vec![
                    JobEdge {
                        from: JobId::from("branch_a"),
                        kind: DependencyType::AfterOk,
                    },
                    JobEdge {
                        from: JobId::from("branch_b"),
                        kind: DependencyType::AfterOk,
                    },
                ],
            },
        };
        let s = toml::to_string(&h).unwrap();
        let back: JobHolder = toml::from_str(&s).unwrap();
        assert_eq!(back.job.parents.len(), 2);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib --no-default-features job_no_id_field_flatten`
Expected: compile error `cannot find struct 'Job' in this scope`.

- [ ] **Step 3: Write minimal implementation**

Add to `src/entities/slurm/job.rs`, after the `JobSpec` block:

```rust
/// LARGE tier: a `JobSpec` placed in a `JobFlow`.
/// Identified positionally by its key in `JobFlow.jobs: BTreeMap<JobId, Job>`
/// — there is *no* `id` field on `Job` itself.
///
/// Designed as the future home for runtime state added by the TaskManager PR
/// (`slurm_jobid: Option<SlurmJobId>`, `status_history: Vec<StatusEntry>`,
/// `started_at` / `finished_at`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Job {
    /// Pure work definition. `#[serde(flatten)]` so program/config/body
    /// appear as siblings of `parents` in TOML — no `[spec]` nesting.
    #[serde(flatten)]
    pub spec: JobSpec,

    /// Incoming dependency edges. Empty = root.
    #[serde(default)]
    pub parents: Vec<JobEdge>,
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib --no-default-features job_no_id_field_flatten job_root_has_empty_parents job_with_one_parent job_with_dag_join_two_parents`
Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/entities/slurm/job.rs
git commit -m "feat: add Job struct (entities/slurm/job.rs)

LARGE tier of the two-tier Job design (spec §5.2): JobSpec wrapper
that adds parents: Vec<JobEdge>. NO id field — JobId is the map
key in JobFlow.jobs.

#[serde(flatten)] so TOML stays flat (program/config/body siblings
of parents — no [spec] sub-table). Future runtime fields will sit
beside parents without disturbing the existing layout."
```

---

## Task 6: Wire `slurm/job.rs` re-exports into `src/entities/slurm.rs`

**Files:**
- Modify: `src/entities/slurm.rs`

- [ ] **Step 1: Write the failing test**

Append to `src/entities/slurm.rs` (this creates a new test mod at the very end of the file):

```rust
#[cfg(test)]
mod reexport_tests {
    // Verifies the top-level slurm module re-exports the new types so
    // downstream code can `use crate::entities::slurm::{Job, JobSpec, ...}`
    // without reaching into the inner `job` module.
    #[test]
    fn job_module_types_are_reexported() {
        fn _assert<T>() {}
        _assert::<super::Job>();
        _assert::<super::JobSpec>();
        _assert::<super::JobEdge>();
        _assert::<super::JobId>();
        _assert::<super::Program>();
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib --no-default-features job_module_types_are_reexported`
Expected: compile error `cannot find type 'Job' in scope`.

- [ ] **Step 3: Write minimal implementation**

In `src/entities/slurm.rs`, find the line `pub mod job;` (added in Task 1) and add immediately after it:

```rust
pub use job::{Job, JobEdge, JobId, JobSpec, Program};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib --no-default-features job_module_types_are_reexported`
Expected: 1 test passes.

- [ ] **Step 5: Commit**

```bash
git add src/entities/slurm.rs
git commit -m "feat: re-export Job/JobSpec/JobEdge/JobId/Program from entities::slurm

Lets downstream code import the new types via
crate::entities::slurm::{Job, JobSpec, …} without needing to know
they live in the inner `job` submodule. Same pattern as the existing
re-exports for SlurmDependency / ResourceSpec / JobTimeLimit."
```

---

## Task 7: `JobLifecycleStatus` enum

**Files:**
- Create: `src/entities/slurm/status.rs`
- Modify: `src/entities/slurm.rs` (add `pub mod status;`)

- [ ] **Step 1: Write the failing test**

Create `src/entities/slurm/status.rs` with test only:

```rust
//! JobLifecycleStatus + StatusEntry — Python `Status` / `StatusEntry` mirror.
//! See `docs/superpowers/specs/2026-05-08-slurm-job-flow-structs-design.md` §5.3.

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Holder {
        status: JobLifecycleStatus,
    }

    #[test]
    fn job_lifecycle_status_serializes_lowercase() {
        let h = Holder { status: JobLifecycleStatus::Queued };
        let s = toml::to_string(&h).unwrap();
        assert!(s.contains(r#"status = "queued""#), "actual TOML: {s}");
    }

    #[test]
    fn job_lifecycle_status_roundtrip_all_variants() {
        for status in [
            JobLifecycleStatus::Queued,
            JobLifecycleStatus::Running,
            JobLifecycleStatus::Done,
            JobLifecycleStatus::Failed,
        ] {
            let h = Holder { status };
            let s = toml::to_string(&h).unwrap();
            let back: Holder = toml::from_str(&s).unwrap();
            assert_eq!(back, h, "round-trip failed for {:?}", status);
        }
    }
}
```

Modify `src/entities/slurm.rs` to add right after the `pub mod job;` line:

```rust
pub mod status;
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib --no-default-features job_lifecycle_status`
Expected: compile error `cannot find type 'JobLifecycleStatus'`.

- [ ] **Step 3: Write minimal implementation**

In `src/entities/slurm/status.rs`, add right after the `use serde…` line:

```rust
/// Lifecycle of a Job from a workflow perspective. Mirrors Python's
/// `gaussian_job_shared.fs.status.Status`. Distinct from `SlurmJobState`
/// (PENDING/RUNNING/...) which lives in the slurm-async-runner crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobLifecycleStatus {
    Queued,
    Running,
    Done,
    Failed,
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib --no-default-features job_lifecycle_status`
Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/entities/slurm/status.rs src/entities/slurm.rs
git commit -m "feat: add JobLifecycleStatus enum (entities/slurm/status.rs)

Mirrors Python's Status (queued/running/done/failed) with
#[serde(rename_all = 'lowercase')] for matching TOML form.
Distinct from SlurmJobState (slurm-async-runner crate, out of scope)."
```

---

## Task 8: `StatusEntry` struct

**Files:**
- Modify: `src/entities/slurm/status.rs`

- [ ] **Step 1: Write the failing test**

Append to the `tests` mod in `src/entities/slurm/status.rs`:

```rust
    use chrono::{TimeZone, Utc};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct EntryHolder {
        entry: StatusEntry,
    }

    #[test]
    fn status_entry_toml_roundtrip() {
        let h = EntryHolder {
            entry: StatusEntry {
                status: JobLifecycleStatus::Running,
                transitioned_at: Utc.with_ymd_and_hms(2026, 5, 8, 12, 0, 0).unwrap(),
            },
        };
        let s = toml::to_string(&h).unwrap();
        let back: EntryHolder = toml::from_str(&s).unwrap();
        assert_eq!(back, h);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib --no-default-features status_entry_toml_roundtrip`
Expected: compile error `cannot find struct 'StatusEntry'`.

- [ ] **Step 3: Write minimal implementation**

Add to `src/entities/slurm/status.rs`, after the `JobLifecycleStatus` enum:

```rust
/// One entry in a Job's status history: a (status, timestamp) pair.
/// Mirrors Python's `StatusEntry` dataclass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusEntry {
    pub status: JobLifecycleStatus,
    pub transitioned_at: chrono::DateTime<chrono::Utc>,
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib --no-default-features status_entry_toml_roundtrip`
Expected: 1 test passes.

- [ ] **Step 5: Commit**

```bash
git add src/entities/slurm/status.rs
git commit -m "feat: add StatusEntry struct (entities/slurm/status.rs)

Pairs JobLifecycleStatus with a UTC timestamp. Used by the future
status-history layer (TaskManager PR)."
```

---

## Task 9: Wire `slurm/status.rs` re-exports

**Files:**
- Modify: `src/entities/slurm.rs`

- [ ] **Step 1: Write the failing test**

Append to the `reexport_tests` mod added in Task 6 (in `src/entities/slurm.rs`):

```rust
    #[test]
    fn status_module_types_are_reexported() {
        fn _assert<T>() {}
        _assert::<super::JobLifecycleStatus>();
        _assert::<super::StatusEntry>();
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib --no-default-features status_module_types_are_reexported`
Expected: compile error `cannot find type 'JobLifecycleStatus'`.

- [ ] **Step 3: Write minimal implementation**

In `src/entities/slurm.rs`, add right after the `pub use job::{...}` line:

```rust
pub use status::{JobLifecycleStatus, StatusEntry};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib --no-default-features status_module_types_are_reexported`
Expected: 1 test passes.

- [ ] **Step 5: Commit**

```bash
git add src/entities/slurm.rs
git commit -m "feat: re-export JobLifecycleStatus / StatusEntry from entities::slurm"
```

---

## Task 10: `CalcType` newtype (in new `entities/job_flow.rs`)

**Files:**
- Create: `src/entities/job_flow.rs`
- Modify: `src/entities.rs` (add `pub mod job_flow;`)

- [ ] **Step 1: Write the failing test**

Create `src/entities/job_flow.rs` with test only:

```rust
//! JobFlow + CalcType — top-level flow type. See spec §5.1.

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Holder {
        calc_type: CalcType,
    }

    #[test]
    fn calc_type_toml_roundtrip() {
        let h = Holder {
            calc_type: CalcType::from("opt"),
        };
        let s = toml::to_string(&h).unwrap();
        assert!(s.contains(r#"calc_type = "opt""#), "actual TOML: {s}");
        let back: Holder = toml::from_str(&s).unwrap();
        assert_eq!(back, h);
    }
}
```

Modify `src/entities.rs` to add right after the existing `pub mod slurm;`:

```rust
pub mod job_flow;
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib --no-default-features calc_type_toml_roundtrip`
Expected: compile error `cannot find type 'CalcType'`.

- [ ] **Step 3: Write minimal implementation**

Add to `src/entities/job_flow.rs`, right after the `use serde…` line:

```rust
/// Calculation type — describes the overall purpose of a `JobFlow`
/// (e.g. "opt", "freq", "opt+td"). Stage-level kinds are intentionally
/// not modelled here (see spec §11 for the deferred decision).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CalcType(pub String);

impl std::fmt::Display for CalcType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for CalcType {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for CalcType {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib --no-default-features calc_type_toml_roundtrip`
Expected: 1 test passes.

- [ ] **Step 5: Commit**

```bash
git add src/entities/job_flow.rs src/entities.rs
git commit -m "feat: add CalcType newtype (entities/job_flow.rs)

Same shape as JobId/Program — transparent-serde String wrapper with
Display + From conversions. Lives in job_flow.rs because it is a
JobFlow-level concern (overall flow purpose), not per-Job."
```

---

## Task 11: `JobFlow` struct

**Files:**
- Modify: `src/entities/job_flow.rs`

- [ ] **Step 1: Write the failing test**

Append to the `tests` mod in `src/entities/job_flow.rs`:

```rust
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    use crate::entities::slurm::{
        DependencyType, Job, JobEdge, JobId, JobSpec, Program, SlurmJobConfig,
    };

    fn sample_config() -> SlurmJobConfig {
        SlurmJobConfig {
            partition: "long".to_string(),
            time_limit: None,
            log_stdout: None,
            log_stderr: None,
            comment: None,
            job_name: None,
            array_spec: None,
            dependency: None,
            mail_user: None,
            mail_types: None,
            resource_spec: None,
        }
    }

    fn make_job(program: &str, parents: Vec<JobEdge>) -> Job {
        Job {
            spec: JobSpec {
                program: Program::from(program),
                config: sample_config(),
                body: String::new(),
            },
            parents,
        }
    }

    fn empty_flow() -> JobFlow {
        JobFlow {
            uuid: Uuid::nil(),
            calc_type: CalcType::from("opt"),
            created_at: Utc.with_ymd_and_hms(2026, 5, 8, 0, 0, 0).unwrap(),
            work_dir: PathBuf::from("/tmp/flow"),
            tags: BTreeMap::new(),
            jobs: BTreeMap::new(),
        }
    }

    #[test]
    fn job_flow_empty_jobs_roundtrip() {
        let flow = empty_flow();
        let s = toml::to_string(&flow).unwrap();
        let back: JobFlow = toml::from_str(&s).unwrap();
        assert_eq!(back.jobs.len(), 0);
        assert_eq!(back.calc_type, flow.calc_type);
    }

    #[test]
    fn job_flow_g16_post_pair_roundtrip() {
        let mut flow = empty_flow();
        flow.jobs.insert(JobId::from("g16"), make_job("g16", vec![]));
        flow.jobs.insert(
            JobId::from("post"),
            make_job(
                "formchk",
                vec![JobEdge {
                    from: JobId::from("g16"),
                    kind: DependencyType::AfterOk,
                }],
            ),
        );
        let s = toml::to_string(&flow).unwrap();
        // Verify named-section TOML form: `[jobs.g16]` and `[jobs.post]`.
        assert!(s.contains("[jobs.g16]"), "actual TOML: {s}");
        assert!(s.contains("[jobs.post]"), "actual TOML: {s}");
        let back: JobFlow = toml::from_str(&s).unwrap();
        assert_eq!(back.jobs.len(), 2);
        assert_eq!(back.jobs[&JobId::from("post")].parents.len(), 1);
        assert_eq!(
            back.jobs[&JobId::from("post")].parents[0].from,
            JobId::from("g16")
        );
    }

    #[test]
    fn job_flow_iteration_order_is_alphabetical() {
        let mut flow = empty_flow();
        flow.jobs.insert(JobId::from("post"), make_job("formchk", vec![]));
        flow.jobs.insert(JobId::from("g16"), make_job("g16", vec![]));
        let keys: Vec<_> = flow.jobs.keys().cloned().collect();
        assert_eq!(keys, vec![JobId::from("g16"), JobId::from("post")]);
    }

    #[test]
    fn job_flow_id_lookup_is_constant_form() {
        let mut flow = empty_flow();
        flow.jobs.insert(JobId::from("g16"), make_job("g16", vec![]));
        // The point of BTreeMap storage: get(&id) is O(log N) and built-in.
        // No need for a JobFlow::find helper.
        assert!(flow.jobs.get(&JobId::from("g16")).is_some());
        assert!(flow.jobs.get(&JobId::from("nope")).is_none());
    }

    #[test]
    fn job_flow_duplicate_jobid_rejected_at_deserialize() {
        // Two [jobs.g16] sections is a TOML duplicate-key error.
        let bad = r#"
uuid = "00000000-0000-0000-0000-000000000000"
calc_type = "opt"
created_at = 2026-05-08T00:00:00Z
work_dir = "/tmp/flow"
tags = {}

[jobs.g16]
program = "g16"
body = ""
parents = []
[jobs.g16.config]
partition = "long"

[jobs.g16]
program = "g16"
body = ""
parents = []
[jobs.g16.config]
partition = "long"
"#;
        assert!(toml::from_str::<JobFlow>(bad).is_err());
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib --no-default-features job_flow_`
Expected: compile error `cannot find struct 'JobFlow'`.

- [ ] **Step 3: Write minimal implementation**

Replace the file head of `src/entities/job_flow.rs` (currently `use serde::{Deserialize, Serialize};`) with:

```rust
use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::slurm::{Job, JobId};
```

Add to `src/entities/job_flow.rs`, after the `CalcType` block:

```rust
/// Top-level job-flow unit. See spec §5.1.
///
/// Storage: `jobs: BTreeMap<JobId, Job>` — the map structure is the single
/// source of truth for the stable `JobId` (no separate `id` field on `Job`)
/// and structurally enforces ID uniqueness. ID lookup is `O(log N)` via
/// `flow.jobs.get(&id)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobFlow {
    /// UUID v7 — identifier of this logical job-flow unit.
    pub uuid: Uuid,

    /// Calculation type ("opt", "freq", "opt+td", ...).
    pub calc_type: CalcType,

    /// Creation timestamp (UTC).
    pub created_at: DateTime<Utc>,

    /// Working directory: `<work_dir>/<JobId>/` is each Job's folder.
    /// TaskManager creates these and writes the rendered `.bash` etc.
    pub work_dir: PathBuf,

    /// Free-form metadata. BTreeMap for deterministic order. (Until a
    /// typed `experiment_id` field is added — see spec §11 — projects
    /// can stash an `"experiment"` key here.)
    #[serde(default)]
    pub tags: BTreeMap<String, String>,

    /// The DAG. Map key = stable `JobId`. Iteration order is alphabetical
    /// by key; execution order is determined by the DAG (`Job.parents`).
    #[serde(default)]
    pub jobs: BTreeMap<JobId, Job>,
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib --no-default-features job_flow_`
Expected: 5 tests pass (`job_flow_empty_jobs_roundtrip`, `job_flow_g16_post_pair_roundtrip`, `job_flow_iteration_order_is_alphabetical`, `job_flow_id_lookup_is_constant_form`, `job_flow_duplicate_jobid_rejected_at_deserialize`).

- [ ] **Step 5: Commit**

```bash
git add src/entities/job_flow.rs
git commit -m "feat: add JobFlow struct (entities/job_flow.rs)

Top-level flow type per spec §5.1: identity (uuid, calc_type,
created_at) + work_dir + tags + jobs: BTreeMap<JobId, Job>.

The BTreeMap storage gives O(log N) ID lookup, structural ID
uniqueness (no validation rule needed — TOML duplicate keys are a
parse error), and a TOML form with named [jobs.<id>] sections."
```

---

## Task 12: Wire `entities/job_flow.rs` re-exports

**Files:**
- Modify: `src/entities.rs`

- [ ] **Step 1: Write the failing test**

Append to `src/entities.rs`:

```rust
#[cfg(test)]
mod reexport_tests {
    #[test]
    fn job_flow_module_types_are_reexported() {
        fn _assert<T>() {}
        _assert::<super::JobFlow>();
        _assert::<super::CalcType>();
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib --no-default-features job_flow_module_types_are_reexported`
Expected: compile error `cannot find type 'JobFlow'`.

- [ ] **Step 3: Write minimal implementation**

In `src/entities.rs`, add right after the `pub mod job_flow;` line:

```rust
pub use job_flow::{CalcType, JobFlow};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib --no-default-features job_flow_module_types_are_reexported`
Expected: 1 test passes.

- [ ] **Step 5: Commit**

```bash
git add src/entities.rs
git commit -m "feat: re-export JobFlow / CalcType from entities

Final re-export so callers can write
`use crate::entities::{JobFlow, CalcType, slurm::*};`."
```

---

## Final Verification

Once all tasks pass:

- [ ] **Step 1: Run the full test suite**

Run: `cargo test --lib --no-default-features`
Expected: all tests pass (existing + the new ones from this plan, ~20 new tests).

- [ ] **Step 2: Build with default features (sanity check)**

Run: `cargo build --lib`
Expected: success. (This exercises the `pyo3` and `stub_gen` features even though we did not write tests for them.)

- [ ] **Step 3: Inspect the public API surface**

Run: `cargo doc --lib --no-default-features --no-deps`
Expected: docs build with no errors.

If the public API looks right and all tests are green, the plan is complete.

---

## Out of Scope (do **not** add in this PR)

The following are explicitly deferred per spec §2 / §11. Do not let them creep in:

- `SlurmJobId` newtype (TaskManager PR)
- Runtime fields on `Job`: `slurm_jobid`, `status_history`, `started_at`, `finished_at`
- `parent_uuids` on `JobFlow` (cross-flow lineage)
- `experiment_id` / `ExperimentId` newtype
- Validation logic (`UnknownParent`, `CycleDetected`, `SelfLoop`, …)
- Traversal helpers (`roots()`, `children_of()`, `topological()`)
- `pyo3` / `gen_stub_pyclass` annotations on the new types
- TOML read/write helpers (`read_metadata` / `write_metadata` equivalents)
- Filesystem operations (folder creation under `work_dir`, bash rendering, summary file)

If during implementation you spot a strong need for one of these, leave a TODO comment with a `// TODO(taskmanager-pr): ...` prefix and stop. Discuss with the user before scope-creeping.
