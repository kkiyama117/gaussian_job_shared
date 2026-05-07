use std::path::PathBuf;

use chrono::TimeDelta;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::error::SchemaParseError;

pub mod array_spec;

pub mod dependency;

pub mod resource_spec;

pub mod slurm;
