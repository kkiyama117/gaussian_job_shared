# Changelog

All notable changes to `gaussian_job_shared` are recorded here. The format
follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Changed (BREAKING — Python surface)

- `JobLifecycleStatus` is no longer a flat `enum.Enum` of four
  unit variants. It is now a wrapper class around a sum type covering
  every official SLURM job state code:

  ```
  Queued(QueuedKind) | Running(RunningKind) | Done | Failed(FailureKind) | Unknown
  ```

  Construct via `JobLifecycleStatus.queued(QueuedKind.Pending)`,
  `JobLifecycleStatus.parse("CANCELLED by 1234")`, etc. Inspect via
  `s.kind` (`"queued"` / `"running"` / `"done"` / `"failed"` /
  `"unknown"`) plus `s.queued_kind()` / `s.running_kind()` /
  `s.failure_kind()` (each returns `None` unless the outer variant
  matches).

  Migration: code comparing flat-enum equality (e.g.
  `JobLifecycleStatus.Queued == s`) becomes `s.kind == "queued"` or
  `s.queued_kind() is QueuedKind.Pending`.

  Already-persisted TOML using the four lowercase tokens (`"queued"`,
  `"running"`, `"done"`, `"failed"`) continues to deserialize cleanly
  to the canonical sub-variant — no data migration is required.

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
