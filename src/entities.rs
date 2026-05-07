/// Strict-schema entities for `common.toml` (cluster/account-level settings
/// that rarely change). Mirrors the `[slurm]`, `[slurm.resource_spec]`,
/// `[env]`, and `[gaussian_cmd]` tables documented in README.md.
pub mod common_config_entities {
    use std::path::PathBuf;

    use serde::{Deserialize, Serialize};

    use super::slurm_config_entities::MailType;

    /// Top-level structure of `common.toml`.
    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    pub struct CommonConfig {
        pub slurm: Slurm,
        pub env: Env,
        pub gaussian_cmd: GaussianCmd,
    }

    /// `[slurm]` table — SLURM submission defaults.
    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    pub struct Slurm {
        pub partition: String,
        #[serde(default)]
        pub job_name: Option<String>,
        /// Wall-clock limit as `"HH:MM:SS"`. Convert to
        /// [`chrono::TimeDelta`] via [`super::slurm_config_entities::JobTimeLimit`].
        #[serde(default)]
        pub time_limit: Option<String>,
        #[serde(default)]
        pub log_stdout: Option<PathBuf>,
        #[serde(default)]
        pub log_stderr: Option<PathBuf>,
        #[serde(default)]
        pub mail_user: Option<String>,
        #[serde(default)]
        pub mail_types: Vec<MailType>,
        pub resource_spec: ResourceSpec,
        /// Optional per-field overrides for the post-processing batch.
        /// When omitted, the post batch inherits the surrounding `[slurm]`
        /// values verbatim (per-field fallback per the α design).
        #[serde(default)]
        pub post: Option<SlurmPost>,
    }

    /// `[slurm.post]` table — per-field overrides for the post-processing
    /// batch. Every field is `Option`al; `None` means "inherit from
    /// `[slurm]`". `mail_types` is `Option<Vec<_>>` so that an explicit
    /// empty list (suppress all mails) can be distinguished from "inherit".
    #[derive(Debug, Clone, Default, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    pub struct SlurmPost {
        #[serde(default)]
        pub partition: Option<String>,
        #[serde(default)]
        pub job_name: Option<String>,
        #[serde(default)]
        pub time_limit: Option<String>,
        #[serde(default)]
        pub log_stdout: Option<PathBuf>,
        #[serde(default)]
        pub log_stderr: Option<PathBuf>,
        #[serde(default)]
        pub mail_user: Option<String>,
        #[serde(default)]
        pub mail_types: Option<Vec<MailType>>,
        #[serde(default)]
        pub resource_spec: Option<ResourceSpec>,
    }

    /// `[slurm.resource_spec]` — `p` procs, `t` threads, `c` cores, `m` memory,
    /// optional `g` GPUs.
    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    pub struct ResourceSpec {
        pub p: u32,
        pub t: u32,
        pub c: u32,
        /// Memory string such as `"56G"`.
        pub m: String,
        #[serde(default)]
        pub g: Option<u32>,
    }

    /// `[env]` — root paths and the per-task base name used by
    /// `PathResolver`.
    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    pub struct Env {
        pub root: PathBuf,
        pub tmp_root: PathBuf,
        pub task_basename: String,
    }

    /// `[gaussian_cmd]` — Gaussian executable command (e.g. `g16`).
    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    pub struct GaussianCmd {
        pub command: String,
    }
}

/// Strict-schema entities for `experiment.toml` (per-experiment plan: which
/// compounds run through which `(program, calc_type)` step). Mirrors the
/// `[experiment]` table and `[[step]]` array documented in README.md.
pub mod experiment_config_entities {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Serialize};

    /// Top-level structure of `experiment.toml`.
    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    pub struct ExperimentDoc {
        pub experiment: Experiment,
        /// `[[step]]` array. Renamed via serde so the public Rust field is
        /// pluralised while the TOML key remains `step` (idiomatic for an
        /// array-of-tables).
        #[serde(rename = "step", default)]
        pub steps: Vec<Step>,
    }

    /// `[experiment]` table.
    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    pub struct Experiment {
        pub id: String,
        #[serde(default)]
        pub tags: BTreeMap<String, String>,
    }

    /// One entry in the `[[step]]` array.
    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    pub struct Step {
        pub program: String,
        pub calc_type: String,
        #[serde(default)]
        pub compounds: Vec<String>,
        #[serde(default)]
        pub parent_uuids: Vec<String>,
        #[serde(default)]
        pub tags: BTreeMap<String, String>,
        pub params: StepParams,
    }

    /// `[step.params]` — kept as a raw TOML table so that the
    /// `(program, calc_type)` registry described in README.md can later
    /// re-parse it into a typed `CalcParams` value (e.g. `GaussianParams`).
    /// Using `toml::Table` preserves ordering and value typing without
    /// committing the strict-schema layer to a specific program.
    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(transparent)]
    pub struct StepParams(pub toml::Table);
}

pub mod slurm_config_entities {
    use std::path::PathBuf;

    use chrono::TimeDelta;
    use itertools::Itertools;
    use serde::{Deserialize, Serialize};

    use crate::error::StrictSchemaError;

    pub type JobPertition = String;

    /// Custom String type of Job Time Limit
    /// We can convert it from and (try_)into [`chrono::TimeDelta`]
    /// -t HOUR:MINUTES:SECONDS,
    #[derive(Debug, Clone)]
    pub struct JobTimeLimit(String);

    impl From<TimeDelta> for JobTimeLimit {
        fn from(value: TimeDelta) -> Self {
            let total_seconds = value.num_seconds();
            let hours = total_seconds / 3600;
            let minutes = (total_seconds % 3600) / 60;
            let seconds = total_seconds % 60;
            let inner = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);
            Self(inner)
        }
    }

    impl std::fmt::Display for JobTimeLimit {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl TryFrom<JobTimeLimit> for TimeDelta {
        type Error = StrictSchemaError;

        fn try_from(value: JobTimeLimit) -> Result<Self, Self::Error> {
            let inner = value.0;
            let sp: Vec<_> = inner
                .split(':')
                .map(|i| i.parse::<i8>())
                .try_collect()
                .map_err(|_| StrictSchemaError::ParseError(inner))?;
            Ok(TimeDelta::hours(sp[0].into())
                + TimeDelta::minutes(sp[1].into())
                + TimeDelta::seconds(sp[2].into()))
        }
    }

    /// TODO: implement Custom struct
    pub type JobRSC = String;

    // TODO: impl TryFrom<String>;
    ///  SBATCH - a <start_num>-<end_num>[option]
    /// https://web.kudpc.kyoto-u.ac.jp/manual/ja/run/tips#arrayjob
    pub type SlurmArraySpec = String;

    // TODO: impl TryFrom<String>;
    ///  SBATCH  -d afterok:200
    /// https://slurm.schedmd.com/sbatch.html
    /// https://web.kudpc.kyoto-u.ac.jp/manual/ja/run/tips#dependency
    pub type SlurmDependency = String;

    pub type MailAddress = String;

    #[derive(Clone, Debug, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
    pub enum MailType {
        BEGIN,
        END,
        FAIL,
        REQUEUE,
        ALL,
    }

    impl TryFrom<&str> for MailType {
        type Error = StrictSchemaError;

        fn try_from(value: &str) -> Result<Self, Self::Error> {
            match value {
                "BEGIN" => Ok(MailType::BEGIN),
                "END" => Ok(MailType::END),
                "FAIL" => Ok(MailType::FAIL),
                "REQUEUE" => Ok(MailType::REQUEUE),
                "ALL" => Ok(MailType::ALL),
                _ => Err(StrictSchemaError::ParseError(value.to_string())),
            }
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct MailTypeInput(Vec<MailType>);

    impl TryFrom<String> for MailTypeInput {
        type Error = StrictSchemaError;

        fn try_from(value: String) -> Result<Self, Self::Error> {
            Ok(MailTypeInput(
                value.split(',').map(MailType::try_from).try_collect()?,
            ))
        }
    }

    #[derive(Debug, Clone)]
    pub struct SBatchDirectives {
        /// queue of job.
        pub pertition: JobPertition,
        /// time limit hh:mm:ss
        pub time_limit: Option<JobTimeLimit>,
        /// p=PROCS:t=THREADSc=CORES:m=MEMORY (or g=GPU)
        pub rsc: Option<JobRSC>,
        /// path of stdout
        pub log_stdout: Option<PathBuf>,
        /// path of stderr
        pub log_stderr: Option<PathBuf>,
        /// comment
        pub comment: Option<String>,
        /// job_name
        pub job_name: Option<String>,
        pub array_spec: Option<SlurmArraySpec>,
        pub dependency: Option<SlurmDependency>,
        pub mail_user: Option<MailAddress>,
        pub mail_type: Option<MailTypeInput>,
    }
}

#[cfg(test)]
mod tests {
    use super::common_config_entities::*;
    use super::experiment_config_entities::*;
    use super::slurm_config_entities::MailType;

    const COMMON_TOML: &str = r#"
[slurm]
partition   = "gr10641a"
job_name    = "GAUSSIAN"
time_limit  = "48:00:00"
log_stdout  = "/x/%x.%j.out"
log_stderr  = "/x/%x.%j.err"
mail_user   = "u@example.com"
mail_types  = ["BEGIN", "END", "FAIL"]

[slurm.resource_spec]
p = 1
t = 56
c = 56
m = "56G"

[env]
root          = "/LARGE0/gr10641/calc_data/GAUSSIAN"
tmp_root      = "/tmp/gaussian"
task_basename = "main"

[gaussian_cmd]
command = "g16"
"#;

    const EXPERIMENT_TOML: &str = r##"
[experiment]
id   = "exp_demo"
tags = { project = "donor_acceptor_screen" }

[[step]]
program      = "gaussian"
calc_type    = "opt"
compounds    = ["X"]
parent_uuids = []
tags         = { basis = "6-31g(d)" }

[step.params]
route        = "#p opt b3lyp/6-31g(d)"
charge       = 0
multiplicity = 1
extra_input  = ""
"##;

    #[test]
    fn deserializes_common_toml_from_readme() {
        let cfg: CommonConfig = toml::from_str(COMMON_TOML).expect("parse common.toml");
        assert_eq!(cfg.slurm.partition, "gr10641a");
        assert_eq!(cfg.slurm.job_name.as_deref(), Some("GAUSSIAN"));
        assert_eq!(cfg.slurm.time_limit.as_deref(), Some("48:00:00"));
        assert_eq!(
            cfg.slurm.mail_types,
            vec![MailType::BEGIN, MailType::END, MailType::FAIL]
        );
        assert_eq!(cfg.slurm.resource_spec.p, 1);
        assert_eq!(cfg.slurm.resource_spec.t, 56);
        assert_eq!(cfg.slurm.resource_spec.c, 56);
        assert_eq!(cfg.slurm.resource_spec.m, "56G");
        assert_eq!(cfg.env.task_basename, "main");
        assert_eq!(cfg.gaussian_cmd.command, "g16");
    }

    #[test]
    fn rejects_unknown_keys_in_common_toml() {
        let bad = r#"
[slurm]
partition = "p"
unexpected = "value"

[slurm.resource_spec]
p = 1
t = 1
c = 1
m = "1G"

[env]
root = "/r"
tmp_root = "/t"
task_basename = "main"

[gaussian_cmd]
command = "g16"
"#;
        let result: Result<CommonConfig, _> = toml::from_str(bad);
        assert!(result.is_err(), "deny_unknown_fields must reject extras");
    }

    #[test]
    fn deserializes_examples_common_toml_with_post_override() {
        let raw = include_str!("../examples/common.toml");
        let cfg: CommonConfig = toml::from_str(raw).expect("parse examples/common.toml");
        assert_eq!(cfg.slurm.partition, "gr10641a");
        let post = cfg.slurm.post.as_ref().expect("post override present");
        assert_eq!(post.time_limit.as_deref(), Some("1:00:00"));
        assert_eq!(post.job_name.as_deref(), Some("GAUSSIAN_post"));
        let post_rsc = post.resource_spec.as_ref().expect("post.resource_spec");
        assert_eq!(post_rsc.t, 1);
        assert_eq!(post_rsc.c, 4);
        assert_eq!(post_rsc.m, "8G");
        assert_eq!(cfg.env.task_basename, "main");
    }

    #[test]
    fn deserializes_examples_experiment_toml() {
        let raw = include_str!("../examples/experiment.toml");
        let doc: ExperimentDoc = toml::from_str(raw).expect("parse examples/experiment.toml");
        assert_eq!(doc.experiment.id, "exp_smoke_single_step");
        assert_eq!(doc.steps.len(), 1);
        assert_eq!(doc.steps[0].calc_type, "opt");
    }

    #[test]
    fn deserializes_experiment_toml_from_readme() {
        let doc: ExperimentDoc = toml::from_str(EXPERIMENT_TOML).expect("parse experiment.toml");
        assert_eq!(doc.experiment.id, "exp_demo");
        assert_eq!(
            doc.experiment.tags.get("project").map(String::as_str),
            Some("donor_acceptor_screen")
        );
        assert_eq!(doc.steps.len(), 1);
        let step = &doc.steps[0];
        assert_eq!(step.program, "gaussian");
        assert_eq!(step.calc_type, "opt");
        assert_eq!(step.compounds, vec!["X".to_string()]);
        assert!(step.parent_uuids.is_empty());
        assert_eq!(step.tags.get("basis").map(String::as_str), Some("6-31g(d)"));
        assert_eq!(
            step.params.0.get("route").and_then(|v| v.as_str()),
            Some("#p opt b3lyp/6-31g(d)")
        );
        assert_eq!(
            step.params.0.get("charge").and_then(|v| v.as_integer()),
            Some(0)
        );
    }

    // -----------------------------------------------------------------------
    // Ported edge-case tests (post-optional, nested rejection, free-form params)
    // -----------------------------------------------------------------------

    #[test]
    fn common_toml_post_section_is_optional() {
        let src = r#"
            [slurm]
            partition  = "p"
            job_name   = "j"
            time_limit = "1:00:00"
            log_stdout = "/tmp/o"
            log_stderr = "/tmp/e"
            mail_user  = "a@b"
            mail_types = []

            [slurm.resource_spec]
            p = 1
            t = 1
            c = 1
            m = "1G"

            [env]
            root          = "/tmp/r"
            tmp_root      = "/tmp/t"
            task_basename = "main"

            [gaussian_cmd]
            command = "g16"
        "#;
        let cfg: CommonConfig = toml::from_str(src).expect("parse minimal common");
        assert!(cfg.slurm.post.is_none());
    }

    #[test]
    fn common_toml_rejects_unknown_nested_key_in_resource_spec() {
        let src = r#"
            [slurm]
            partition  = "p"
            time_limit = "1:00:00"
            mail_types = []

            [slurm.resource_spec]
            p = 1
            t = 1
            c = 1
            m = "1G"
            unknown_inner_key = 4

            [env]
            root          = "/tmp/r"
            tmp_root      = "/tmp/t"
            task_basename = "main"

            [gaussian_cmd]
            command = "g16"
        "#;
        let result: Result<CommonConfig, _> = toml::from_str(src);
        assert!(result.is_err(), "unknown nested key must be rejected");
    }

    #[test]
    fn experiment_toml_rejects_unknown_step_field() {
        let src = r#"
            [experiment]
            id = "x"

            [[step]]
            program       = "gaussian"
            calc_type     = "opt"
            compounds     = ["X"]
            parent_uuids  = []
            unknown_field = "boom"

            [step.params]
        "#;
        let result: Result<ExperimentDoc, _> = toml::from_str(src);
        assert!(result.is_err(), "unknown step field must be rejected");
    }

    #[test]
    fn experiment_toml_allows_arbitrary_params_keys() {
        let src = r#"
            [experiment]
            id = "x"

            [[step]]
            program      = "gaussian"
            calc_type    = "opt"
            compounds    = ["X"]
            parent_uuids = []

            [step.params]
            anything_goes = "here"
            nested        = { foo = 1, bar = "baz" }
        "#;
        let doc: ExperimentDoc = toml::from_str(src).expect("free-form params accepted");
        let step = &doc.steps[0];
        assert!(step.params.0.contains_key("anything_goes"));
        assert!(step.params.0.contains_key("nested"));
    }
}
