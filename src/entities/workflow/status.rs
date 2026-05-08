use serde::{Deserialize, Serialize};

/// Lifecycle of a Job from a workflow perspective. Distinct from
/// `SlurmJobState` (PENDING/RUNNING/...) which lives in the
/// slurm-async-runner crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobLifecycleStatus {
    Queued,
    Running,
    Done,
    Failed,
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

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Holder {
        status: JobLifecycleStatus,
    }

    #[test]
    fn job_lifecycle_status_serializes_lowercase() {
        let h = Holder {
            status: JobLifecycleStatus::Queued,
        };
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
}
