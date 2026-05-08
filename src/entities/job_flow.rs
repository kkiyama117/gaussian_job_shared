//! JobFlow + CalcType — top-level flow type. See spec §5.1.

use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::slurm::{Job, JobId};

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

/// Top-level job-flow unit. See spec §5.1.
///
/// Storage: `jobs: BTreeMap<JobId, Job>` — the map structure is the single
/// source of truth for the stable `JobId` (no separate `id` field on `Job`)
/// and structurally enforces ID uniqueness. ID lookup is `O(log N)` via
/// `flow.jobs.get(&id)`.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct CalcTypeHolder {
        calc_type: CalcType,
    }

    #[test]
    fn calc_type_toml_roundtrip() {
        let h = CalcTypeHolder {
            calc_type: CalcType::from("opt"),
        };
        let s = toml::to_string(&h).unwrap();
        assert!(s.contains(r#"calc_type = "opt""#), "actual TOML: {s}");
        let back: CalcTypeHolder = toml::from_str(&s).unwrap();
        assert_eq!(back, h);
    }

    use chrono::TimeZone;

    use crate::entities::slurm::{DependencyType, JobEdge, JobSpec, Program, SlurmJobConfig};

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
        assert_eq!(back.uuid, flow.uuid);
        assert_eq!(back.work_dir, flow.work_dir);
    }

    #[test]
    fn job_flow_g16_post_pair_roundtrip() {
        let mut flow = empty_flow();
        flow.jobs
            .insert(JobId::from("g16"), make_job("g16", vec![]));
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
        flow.jobs
            .insert(JobId::from("post"), make_job("formchk", vec![]));
        flow.jobs
            .insert(JobId::from("g16"), make_job("g16", vec![]));
        let keys: Vec<_> = flow.jobs.keys().cloned().collect();
        assert_eq!(keys, vec![JobId::from("g16"), JobId::from("post")]);
    }

    #[test]
    fn job_flow_id_lookup_is_constant_form() {
        let mut flow = empty_flow();
        flow.jobs
            .insert(JobId::from("g16"), make_job("g16", vec![]));
        // The point of BTreeMap storage: lookup is O(log N) and built-in.
        // No need for a JobFlow::find helper.
        assert!(flow.jobs.contains_key(&JobId::from("g16")));
        assert!(!flow.jobs.contains_key(&JobId::from("nope")));
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
}
