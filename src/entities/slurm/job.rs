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

/// SMALL tier: state-independent / pre-runtime work definition.
/// Reusable across flows — carries no flow-scoped or runtime state.
/// See `docs/superpowers/specs/2026-05-08-slurm-job-flow-structs-design.md` §5.2.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// LARGE tier: a `JobSpec` placed in a `JobFlow`.
/// Identified positionally by its key in `JobFlow.jobs: BTreeMap<JobId, Job>`
/// — there is *no* `id` field on `Job` itself.
///
/// Designed as the future home for runtime state added by the TaskManager PR
/// (`slurm_jobid: Option<SlurmJobId>`, `status_history: Vec<StatusEntry>`,
/// `started_at` / `finished_at`).
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    #[derive(Debug, Serialize, Deserialize)]
    struct SpecHolder {
        spec: JobSpec,
    }

    #[test]
    fn job_spec_toml_roundtrip_field_by_field() {
        // SlurmJobConfig does not derive PartialEq, so we compare each
        // field individually rather than the whole struct.
        let h = SpecHolder {
            spec: JobSpec {
                program: Program::from("g16"),
                config: sample_config(),
                body: "g16 < input.gjf > output.log\n".to_string(),
            },
        };
        let s = toml::to_string(&h).unwrap();
        let back: SpecHolder = toml::from_str(&s).unwrap();
        assert_eq!(back.spec.program, h.spec.program);
        assert_eq!(back.spec.config.partition, h.spec.config.partition);
        assert_eq!(back.spec.body, h.spec.body);
    }

    #[test]
    fn job_spec_is_state_independent_can_be_cloned() {
        // Sanity: JobSpec carries no flow-scoped reference and can be
        // freely cloned. Compile-time check via .clone().
        let original = JobSpec {
            program: Program::from("g16"),
            config: sample_config(),
            body: "echo hi\n".to_string(),
        };
        let copy = original.clone();
        assert_eq!(copy.program, original.program);
        assert_eq!(copy.body, original.body);
    }

    #[derive(Debug, Serialize, Deserialize)]
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
        assert!(
            !s.contains("[job.spec]"),
            "spec wrapper leaked into TOML: {s}"
        );
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
}
