/// Entities that represents slurm Job and JobConfig
/// For detail, see [Kyoto Univ doc](https://web.kudpc.kyoto-u.ac.jp/manual/ja/run/batch#slurm) and [Official SLURM page](https://slurm.schedmd.com/sbatch.html)
use super::*;

// Each field of Slurm job entity.
pub type JobPartition = String;

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
    type Error = SchemaParseError;

    fn try_from(value: JobTimeLimit) -> Result<Self, Self::Error> {
        let inner = value.0;
        let sp: Vec<_> = inner
            .split(':')
            .map(|i| i.parse::<i8>())
            .try_collect()
            .map_err(|_| SchemaParseError::ParseError {
                key: "-t".to_string(),
                value: inner.to_string(),
            })?;
        Ok(TimeDelta::hours(sp[0].into())
            + TimeDelta::minutes(sp[1].into())
            + TimeDelta::seconds(sp[2].into()))
    }
}

/// TODO: implement Custom struct
pub type JobRSC = String;

// `SlurmArraySpec` and `ArrayIndex` live in their own file so the parsing
// and serde plumbing can be reasoned about in isolation. They are
// re-exported here so existing call sites referencing
// `crate::entities::slurm::SlurmArraySpec` keep working.
pub use crate::entities::array_spec::{ArrayIndex, SlurmArraySpec};

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
    type Error = SchemaParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "BEGIN" => Ok(MailType::BEGIN),
            "END" => Ok(MailType::END),
            "FAIL" => Ok(MailType::FAIL),
            "REQUEUE" => Ok(MailType::REQUEUE),
            "ALL" => Ok(MailType::ALL),
            _ => Err(Self::Error::ParseError {
                key: "mail_types".to_string(),
                value: value.to_string(),
            }),
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MailTypeInput(Vec<MailType>);

impl TryFrom<String> for MailTypeInput {
    type Error = SchemaParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(MailTypeInput(
            value.split(',').map(MailType::try_from).try_collect()?,
        ))
    }
}

/// `[slurm]` table — Replesent config  of SLURM submission.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlurmJobConfig {
    /// queue of job. It is required thing
    pub partition: JobPartition,

    /// Wall-clock limit as `"HH:MM:SS"`. Convert to
    /// [`chrono::TimeDelta`] via [`super::slurm_config_entities::JobTimeLimit`].
    #[serde(default)]
    pub time_limit: Option<String>,

    /// path of stdout
    #[serde(default)]
    pub log_stdout: Option<PathBuf>,

    /// path of stderr
    #[serde(default)]
    pub log_stderr: Option<PathBuf>,

    /// comment
    pub comment: Option<String>,

    /// job_name
    #[serde(default)]
    pub job_name: Option<String>,

    /// spec of Array job
    #[serde(default)]
    pub array_spec: Option<SlurmArraySpec>,

    // / Optional per-field overrides for the post-processing batch.
    // / When omitted, the post batch inherits the surrounding `[slurm]`
    // / values verbatim (per-field fallback per the α design).
    pub dependency: Option<SlurmDependency>,

    #[serde(default)]
    pub mail_user: Option<MailAddress>,

    #[serde(default)]
    pub mail_types: Option<MailTypeInput>,

    /// p=PROCS:t=THREADSc=CORES:m=MEMORY (or g=GPU)
    #[serde(default)]
    pub resource_spec: Option<ResourceSpec>,
}

/// resource_spec
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ResourceSpec {
    /// using cpu  — `p` procs, `t` threads, `c` cores, `m` memory,
    CPU(ResourceSpecCPU),
    /// using gpu  — `g` gpus
    GPU(ResourceSpecGPU),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceSpecCPU {
    pub p: u32,
    pub t: u32,
    pub c: u32,
    /// Memory string such as `"56G"`.
    pub m: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceSpecGPU {
    #[serde(default)]
    pub g: Option<u32>,
}
