//! Job, JobSpec, JobEdge, JobId, Program — the in-flow Slurm Job tier.
//! See `docs/superpowers/specs/2026-05-08-slurm-job-flow-structs-design.md`
//! §5.2.

use serde::{Deserialize, Serialize};

use super::dependency::DependencyType;

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

/// Intra-flow dependency edge — incoming to the enclosing `Job`.
/// `to` is implicit (= the map key of the enclosing `JobFlow.jobs` entry).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobEdge {
    /// Parent (predecessor) — key into the enclosing `JobFlow.jobs`.
    pub from: JobId,

    /// Slurm dependency kind (Afterok / Afterany / After / ...).
    /// Serialized as the Slurm-canonical lowercase keyword (`afterok`,
    /// `afterany`, …) via [`dep_kind_serde`] — same form `DependencyType`'s
    /// own [`std::str::FromStr`] / [`std::fmt::Display`] use, so cross-tool
    /// TOML stays self-consistent without touching `DependencyType`'s
    /// derive (which `SlurmDependency` parses through its own custom impl).
    #[serde(with = "dep_kind_serde")]
    pub kind: DependencyType,
}

/// Adapter so `JobEdge` can serde a `DependencyType` even though that enum
/// has no `derive(Serialize, Deserialize)` of its own. Round-trips through
/// `Display` / `FromStr` (defined in `super::dependency`).
mod dep_kind_serde {
    use std::str::FromStr;

    use serde::{Deserialize, Deserializer, Serializer};

    use super::DependencyType;

    pub fn serialize<S: Serializer>(kind: &DependencyType, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&kind.to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<DependencyType, D::Error> {
        let s = String::deserialize(de)?;
        DependencyType::from_str(&s).map_err(serde::de::Error::custom)
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
}
