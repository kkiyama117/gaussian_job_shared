pub mod slurm;

pub mod workflow;
pub use workflow::{
    CalcType, Job, JobEdge, JobFlow, JobId, JobLifecycleStatus, JobSpec, Program, StatusEntry,
};
