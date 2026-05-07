use crate::entities::slurm::SlurmJobConfig;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CommonConfig {
    /// Set default arguments of slurm_config
    pub slurm_default: SlurmJobConfig,
    /// Set config of directory.
    pub directories: DirectoryConfig,
}

#[derive(Debug, Clone)]
pub struct DirectoryConfig {
    /// Root of all project data.
    pub project_root: PathBuf,
}
