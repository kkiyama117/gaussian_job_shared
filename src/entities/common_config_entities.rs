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
