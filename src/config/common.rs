use slurm_async_runner::entities::slurm::SlurmJobConfig;
use std::path::PathBuf;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommonConfig {
    /// Set default arguments of slurm_config
    pub slurm_default: SlurmJobConfig,
    /// Set config of directory.
    pub directories: DirectoryConfig,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectoryConfig {
    /// Root of all project data.
    pub project_root: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use slurm_async_runner::entities::slurm::SlurmJobConfig;

    fn sample() -> CommonConfig {
        CommonConfig {
            slurm_default: SlurmJobConfig {
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
            },
            directories: DirectoryConfig {
                project_root: PathBuf::from("/work"),
            },
        }
    }

    #[test]
    fn serde_round_trip() {
        let original = sample();
        let toml_str = toml::to_string(&original).unwrap();
        let restored: CommonConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(
            restored.slurm_default.partition,
            original.slurm_default.partition
        );
        assert_eq!(
            restored.directories.project_root,
            original.directories.project_root
        );
    }

    #[test]
    fn deny_unknown_fields_rejects_extra_top_level() {
        let bad = r#"
[slurm_default]
partition = "long"

[directories]
project_root = "/work"

[bogus]
key = "value"
"#;
        let result: Result<CommonConfig, _> = toml::from_str(bad);
        assert!(result.is_err());
    }
}
