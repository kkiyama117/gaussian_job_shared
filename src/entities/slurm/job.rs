//! Job, JobSpec, JobEdge, JobId, Program — the in-flow Slurm Job tier.
//! See `docs/superpowers/specs/2026-05-08-slurm-job-flow-structs-design.md`
//! §5.2.

use serde::{Deserialize, Serialize};

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
        assert_eq!(
            keys,
            vec![JobId("g16".to_string()), JobId("post".to_string())]
        );
    }
}
