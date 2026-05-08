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

### Added

- `QueuedKind`, `RunningKind`, `FailureKind` Python enums covering
  the 8 / 5 / 10 SLURM long-form sub-state tokens respectively.
- `JobLifecycleStatus.parse(raw)` accepting SLURM long forms,
  compact codes, trailing context, and the legacy 4-token form.
- `JobLifecycleStatus.token` returning the SLURM long form.
