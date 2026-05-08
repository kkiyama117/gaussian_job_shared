# Changelog

All notable changes to `gaussian_job_shared` are recorded here. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Changed (BREAKING — module layout + status redesign)

- **`entities::workflow::status` → `entities::slurm::status`.** Job
  lifecycle status is a SLURM concept, so it now lives alongside the
  sbatch-options primitives under `entities::slurm`. Existing
  `crate::entities::slurm::JobStatus` re-exports keep top-level access
  ergonomic.
- **`entities::slurm` is now a parent module with two children:**
  - `entities::slurm::sbatch_options` — what was previously in
    `entities::slurm` (`SlurmJobConfig`, `SlurmArraySpec`,
    `SlurmDependency`, `ResourceSpec`, `JobTimeLimit`, `MailType`,
    `MailTypeInput`, …). Top-level `entities::slurm::*` re-exports
    preserve the old import paths.
  - `entities::slurm::status` — the new lifecycle types described
    below.
- **`JobLifecycleStatus` (4-variant enum) → `JobStatus` (struct).** The
  new shape mirrors SLURM's own `(state, reason)` pair surfaced by
  `squeue %T %r`:

  ```rust
  pub struct JobStatus {
      pub state: JobState,    // flat enum: 24 SLURM state codes + Unknown
      pub reason: JobReason,  // flat enum: ~80 reason codes + None + Other(String)
  }
  ```

  The previous nested variant model (`Queued(QueuedKind) |
  Running(RunningKind) | Done | Failed(FailureKind) | Unknown`) and its
  Python `kind` discriminant + `queued_kind()` / `running_kind()` /
  `failure_kind()` accessor pattern are **gone**. Pattern-matching now
  uses the flat `JobState` enum directly:

  ```rust
  match status.state {
      JobState::Pending | JobState::Configuring => { /* queued */ }
      JobState::Running | JobState::Completing => { /* alive */ }
      JobState::Completed => { /* terminal success */ }
      JobState::OutOfMemory | JobState::Failed | JobState::NodeFail => { /* failed */ }
      _ => {}
  }
  ```

  `JobReason` covers the canonical SLURM `slurm_reason_string` table
  (`Priority`, `Resources`, `Dependency`, `TimeLimit`, `OutOfMemory`,
  every `QOS*` / `Assoc*` limit, etc.) with an `Other(String)` escape
  hatch for forward-compat across SLURM versions.

- **`StatusEntry` removed.** It was unused in this crate (designated as
  a future-home type for `Job.status_history`). When status history is
  finally wired up, the field will use the new `JobStatus` directly.
- **Legacy 4-token TOML compatibility removed.** The `"queued"` /
  `"running"` / `"done"` / `"failed"` lowercase tokens are no longer
  recognized at deserialize time. New writes use SLURM's own UPPERCASE
  long-form for `state` and PascalCase for `reason`.

### Changed (BREAKING — Python surface)

- `gaussian_job_shared._core.entities.slurm` is now a sub-package with
  two child modules:
  - `gaussian_job_shared._core.entities.slurm.sbatch_options` (was
    `gaussian_job_shared._core.entities.slurm`).
  - `gaussian_job_shared._core.entities.slurm.status` — `JobStatus` /
    `JobState` / `JobReason`.
- `gaussian_job_shared._core.entities.workflow` no longer exports any
  status type. `JobLifecycleStatus`, `QueuedKind`, `RunningKind`,
  `FailureKind`, `StatusEntry` are **removed**. Migrate to:

  ```python
  from gaussian_job_shared._core.entities.slurm.sbatch_options import SlurmJobConfig
  from gaussian_job_shared._core.entities.slurm.status import JobStatus, JobState, JobReason

  s = JobStatus.parse("PD", "Priority")
  assert s.state == JobState.Pending
  assert s.reason == JobReason.priority()
  ```

### Changed (BREAKING — on-disk wire format)

- The same in-memory state now serializes to the SLURM long-form
  token (lower-cased), so the next time an existing TOML file is
  rewritten, two of the four legacy tokens change on disk:

  | Variant   | Before     | After         |
  |-----------|------------|---------------|
  | `Queued`  | `"queued"` | `"pending"`   |
  | `Running` | `"running"`| `"running"`   |
  | `Done`    | `"done"`   | `"completed"` |
  | `Failed`  | `"failed"` | `"failed"`    |

  Reads of legacy bytes are unaffected (covered by
  `toml_back_compat_legacy_lowercase_tokens`), so no data migration
  is required. But anything outside this crate that greps or diffs
  TOML by raw string (CI, dashboards, audits) will see churn the
  first time each affected file is rewritten.

  Python `str(status)` mirrors this: `str(JobLifecycleStatus.done())`
  is now `"completed"` (was `"done"`) and the canonical Queued status
  stringifies to `"pending"` (was `"queued"`). Downstream code that
  string-compares or logs `str(status)` should switch to `s.kind`
  (`"queued"` / `"running"` / `"done"` / `"failed"` / `"unknown"`)
  for category checks, or `s.token` for the SLURM long form.

### Added

- `QueuedKind`, `RunningKind`, `FailureKind` Python enums covering
  the 8 / 5 / 10 SLURM long-form sub-state tokens respectively.
- `JobLifecycleStatus.parse(raw)` accepting SLURM long forms,
  compact codes, trailing context, and the legacy 4-token form.
- `JobLifecycleStatus.token` returning the SLURM long form.
