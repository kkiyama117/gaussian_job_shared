//! Lifecycle status of a Job from a workflow perspective. Mirrors the
//! official Slurm job state codes — see
//! <https://slurm.schedmd.com/squeue.html#JOB_STATE_CODES>. The full
//! design rationale (which SLURM tokens map to which outer variant, why
//! `Unknown` exists, why parse is case-insensitive, etc.) lives in
//! `docs/superpowers/specs/2026-05-08-job-lifecycle-status-expand-design.md`.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Lifecycle of a Job from a workflow perspective, with SLURM-grade
/// sub-state where it carries actionable information for the caller.
///
/// Variants follow the official Slurm job state codes. Tokens not
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

/// SLURM "not-yet-progressing" sub-states (PENDING, CONFIGURING,
/// REQUEUED, REQUEUE_FED, REQUEUE_HOLD, RESV_DEL_HOLD, SUSPENDED,
/// STOPPED). See spec §3.1.
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

/// SLURM "alive" sub-states (RUNNING, COMPLETING, RESIZING, SIGNALING,
/// STAGE_OUT). See spec §3.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RunningKind {
    Running,
    Completing,
    Resizing,
    Signaling,
    StageOut,
}

/// SLURM terminal-failure sub-states (BOOT_FAIL, CANCELLED, DEADLINE,
/// FAILED, NODE_FAIL, OUT_OF_MEMORY, PREEMPTED, REVOKED, SPECIAL_EXIT,
/// TIMEOUT). See spec §3.4.
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

impl JobLifecycleStatus {
    /// Parse a raw squeue/sacct state token into a structured status.
    ///
    /// Accepts:
    /// - SLURM long forms (`"PENDING"`, `"OUT_OF_MEMORY"`, …)
    /// - SLURM compact codes (`"PD"`, `"OOM"`, `"NF"`, …)
    /// - Trailing context (`"CANCELLED by 1234"` → first whitespace-separated token wins)
    /// - The 4 legacy workflow tokens (`"queued"`, `"running"`, `"done"`, `"failed"`)
    ///   for back-compat with already-serialized data; they map to the
    ///   canonical first variant of each category.
    /// - Case-insensitive matching across the board.
    ///
    /// Falls back to `Unknown` for any other input (including empty string).
    pub fn parse(raw: &str) -> Self {
        let token = raw.split_whitespace().next().unwrap_or("");
        match token.to_ascii_uppercase().as_str() {
            // -------- Queued --------
            "PENDING" | "PD" | "QUEUED" => Self::Queued(QueuedKind::Pending),
            "CONFIGURING" | "CF" => Self::Queued(QueuedKind::Configuring),
            "REQUEUED" | "RQ" => Self::Queued(QueuedKind::Requeued),
            "REQUEUE_FED" | "RF" => Self::Queued(QueuedKind::RequeueFed),
            "REQUEUE_HOLD" | "RH" => Self::Queued(QueuedKind::RequeueHold),
            "RESV_DEL_HOLD" | "RD" => Self::Queued(QueuedKind::ResvDelHold),
            "SUSPENDED" | "S" => Self::Queued(QueuedKind::Suspended),
            "STOPPED" | "ST" => Self::Queued(QueuedKind::Stopped),

            // -------- Running --------
            "RUNNING" | "R" => Self::Running(RunningKind::Running),
            "COMPLETING" | "CG" => Self::Running(RunningKind::Completing),
            "RESIZING" | "RS" => Self::Running(RunningKind::Resizing),
            "SIGNALING" | "SI" => Self::Running(RunningKind::Signaling),
            "STAGE_OUT" | "SO" => Self::Running(RunningKind::StageOut),

            // -------- Done --------
            "COMPLETED" | "CD" | "DONE" => Self::Done,

            // -------- Failed --------
            "BOOT_FAIL" | "BF" => Self::Failed(FailureKind::BootFail),
            "CANCELLED" | "CA" => Self::Failed(FailureKind::Cancelled),
            "DEADLINE" | "DL" => Self::Failed(FailureKind::Deadline),
            "FAILED" | "F" => Self::Failed(FailureKind::Failed),
            "NODE_FAIL" | "NF" => Self::Failed(FailureKind::NodeFail),
            "OUT_OF_MEMORY" | "OOM" => Self::Failed(FailureKind::OutOfMemory),
            "PREEMPTED" | "PR" => Self::Failed(FailureKind::Preempted),
            "REVOKED" | "RV" => Self::Failed(FailureKind::Revoked),
            "SPECIAL_EXIT" | "SE" => Self::Failed(FailureKind::SpecialExit),
            "TIMEOUT" | "TO" => Self::Failed(FailureKind::Timeout),

            _ => Self::Unknown,
        }
    }

    /// SLURM long-form token for this state. `parse(self.as_token())`
    /// round-trips for every variant (excluding compact-code aliases).
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

/// One entry in a Job's status history: a (status, timestamp) pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusEntry {
    pub status: JobLifecycleStatus,
    pub transitioned_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every variant of the new enum, used by both the round-trip test
    /// and the `as_token` reverse-parse test.
    fn all_variants() -> Vec<JobLifecycleStatus> {
        vec![
            JobLifecycleStatus::Queued(QueuedKind::Pending),
            JobLifecycleStatus::Queued(QueuedKind::Configuring),
            JobLifecycleStatus::Queued(QueuedKind::Requeued),
            JobLifecycleStatus::Queued(QueuedKind::RequeueFed),
            JobLifecycleStatus::Queued(QueuedKind::RequeueHold),
            JobLifecycleStatus::Queued(QueuedKind::ResvDelHold),
            JobLifecycleStatus::Queued(QueuedKind::Suspended),
            JobLifecycleStatus::Queued(QueuedKind::Stopped),
            JobLifecycleStatus::Running(RunningKind::Running),
            JobLifecycleStatus::Running(RunningKind::Completing),
            JobLifecycleStatus::Running(RunningKind::Resizing),
            JobLifecycleStatus::Running(RunningKind::Signaling),
            JobLifecycleStatus::Running(RunningKind::StageOut),
            JobLifecycleStatus::Done,
            JobLifecycleStatus::Failed(FailureKind::BootFail),
            JobLifecycleStatus::Failed(FailureKind::Cancelled),
            JobLifecycleStatus::Failed(FailureKind::Deadline),
            JobLifecycleStatus::Failed(FailureKind::Failed),
            JobLifecycleStatus::Failed(FailureKind::NodeFail),
            JobLifecycleStatus::Failed(FailureKind::OutOfMemory),
            JobLifecycleStatus::Failed(FailureKind::Preempted),
            JobLifecycleStatus::Failed(FailureKind::Revoked),
            JobLifecycleStatus::Failed(FailureKind::SpecialExit),
            JobLifecycleStatus::Failed(FailureKind::Timeout),
            JobLifecycleStatus::Unknown,
        ]
    }

    // ---- parse: long forms ----
    #[test]
    fn parse_long_form_pending() {
        assert_eq!(
            JobLifecycleStatus::parse("PENDING"),
            JobLifecycleStatus::Queued(QueuedKind::Pending)
        );
    }
    #[test]
    fn parse_long_form_running() {
        assert_eq!(
            JobLifecycleStatus::parse("RUNNING"),
            JobLifecycleStatus::Running(RunningKind::Running)
        );
    }
    #[test]
    fn parse_long_form_completed_is_done() {
        assert_eq!(
            JobLifecycleStatus::parse("COMPLETED"),
            JobLifecycleStatus::Done
        );
    }
    #[test]
    fn parse_long_form_out_of_memory() {
        assert_eq!(
            JobLifecycleStatus::parse("OUT_OF_MEMORY"),
            JobLifecycleStatus::Failed(FailureKind::OutOfMemory)
        );
    }
    #[test]
    fn parse_long_form_special_exit() {
        assert_eq!(
            JobLifecycleStatus::parse("SPECIAL_EXIT"),
            JobLifecycleStatus::Failed(FailureKind::SpecialExit)
        );
    }

    // ---- parse: compact codes ----
    #[test]
    fn parse_compact_pd() {
        assert_eq!(
            JobLifecycleStatus::parse("PD"),
            JobLifecycleStatus::Queued(QueuedKind::Pending)
        );
    }
    #[test]
    fn parse_compact_oom() {
        assert_eq!(
            JobLifecycleStatus::parse("OOM"),
            JobLifecycleStatus::Failed(FailureKind::OutOfMemory)
        );
    }
    #[test]
    fn parse_compact_cd_is_done() {
        assert_eq!(JobLifecycleStatus::parse("CD"), JobLifecycleStatus::Done);
    }

    // ---- parse: trailing context, whitespace, case ----
    #[test]
    fn parse_trailing_context_cancelled_by() {
        assert_eq!(
            JobLifecycleStatus::parse("CANCELLED by 1234"),
            JobLifecycleStatus::Failed(FailureKind::Cancelled)
        );
    }
    #[test]
    fn parse_padded_running() {
        assert_eq!(
            JobLifecycleStatus::parse("  RUNNING  "),
            JobLifecycleStatus::Running(RunningKind::Running)
        );
    }
    #[test]
    fn parse_lowercase_pending() {
        assert_eq!(
            JobLifecycleStatus::parse("pending"),
            JobLifecycleStatus::Queued(QueuedKind::Pending)
        );
    }

    // ---- parse: legacy 4-token compatibility ----
    #[test]
    fn parse_legacy_queued() {
        assert_eq!(
            JobLifecycleStatus::parse("queued"),
            JobLifecycleStatus::Queued(QueuedKind::Pending)
        );
    }
    #[test]
    fn parse_legacy_running() {
        assert_eq!(
            JobLifecycleStatus::parse("running"),
            JobLifecycleStatus::Running(RunningKind::Running)
        );
    }
    #[test]
    fn parse_legacy_done() {
        assert_eq!(JobLifecycleStatus::parse("done"), JobLifecycleStatus::Done);
    }
    #[test]
    fn parse_legacy_failed() {
        assert_eq!(
            JobLifecycleStatus::parse("failed"),
            JobLifecycleStatus::Failed(FailureKind::Failed)
        );
    }

    // ---- parse: unknown / empty ----
    #[test]
    fn parse_empty_is_unknown() {
        assert_eq!(JobLifecycleStatus::parse(""), JobLifecycleStatus::Unknown);
    }
    #[test]
    fn parse_blanks_only_is_unknown() {
        assert_eq!(
            JobLifecycleStatus::parse("   "),
            JobLifecycleStatus::Unknown
        );
    }
    #[test]
    fn parse_garbage_is_unknown() {
        assert_eq!(
            JobLifecycleStatus::parse("FOO_BAR_BAZ"),
            JobLifecycleStatus::Unknown
        );
    }

    // ---- as_token round-trip ----
    #[test]
    fn as_token_round_trips_every_variant() {
        for s in all_variants() {
            assert_eq!(
                JobLifecycleStatus::parse(s.as_token()),
                s,
                "as_token round-trip failed for {:?}",
                s,
            );
        }
    }

    // ---- TOML round-trip ----
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Holder {
        status: JobLifecycleStatus,
    }

    #[test]
    fn toml_roundtrip_every_variant() {
        for s in all_variants() {
            let h = Holder { status: s };
            let serialized = toml::to_string(&h).unwrap();
            let back: Holder = toml::from_str(&serialized).unwrap();
            assert_eq!(back, h, "TOML round-trip failed for {:?}", s);
        }
    }

    #[test]
    fn toml_serializes_lowercase_long_form() {
        let h = Holder {
            status: JobLifecycleStatus::Failed(FailureKind::OutOfMemory),
        };
        let serialized = toml::to_string(&h).unwrap();
        assert!(
            serialized.contains(r#"status = "out_of_memory""#),
            "actual TOML: {serialized}"
        );
    }

    #[test]
    fn toml_back_compat_legacy_lowercase_tokens() {
        // Already-persisted data using the 4 legacy lowercase tokens
        // continues to deserialize cleanly to the canonical sub-variant.
        for (text, expected) in [
            (
                r#"status = "queued""#,
                JobLifecycleStatus::Queued(QueuedKind::Pending),
            ),
            (
                r#"status = "running""#,
                JobLifecycleStatus::Running(RunningKind::Running),
            ),
            (r#"status = "done""#, JobLifecycleStatus::Done),
            (
                r#"status = "failed""#,
                JobLifecycleStatus::Failed(FailureKind::Failed),
            ),
        ] {
            let h: Holder = toml::from_str(text).unwrap();
            assert_eq!(h.status, expected, "legacy back-compat failed for {text}");
        }
    }

    // ---- StatusEntry round-trip (richer status now) ----
    use chrono::{TimeZone, Utc};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct EntryHolder {
        entry: StatusEntry,
    }

    #[test]
    fn status_entry_toml_roundtrip() {
        let h = EntryHolder {
            entry: StatusEntry {
                status: JobLifecycleStatus::Failed(FailureKind::Timeout),
                transitioned_at: Utc.with_ymd_and_hms(2026, 5, 8, 12, 0, 0).unwrap(),
            },
        };
        let serialized = toml::to_string(&h).unwrap();
        let back: EntryHolder = toml::from_str(&serialized).unwrap();
        assert_eq!(back, h);
    }
}
