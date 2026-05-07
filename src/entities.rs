pub mod slurm_config_entities {
    use std::path::PathBuf;

    use chrono::TimeDelta;
    use itertools::Itertools;

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

    #[derive(Clone, Debug, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
