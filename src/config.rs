pub mod common;

pub mod gaussian16 {

    use serde::{Deserialize, Serialize};

    /// How to call gaussian
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GaussianCommonConfig {
        pub command: String,
        pub hoge: String,
    }
}

pub mod single_job {}
