/// Strict schema validation utilities and exception family.
use thiserror::Error;

// TODO: Update args and converter
/// All loaders in [gaussian_job_shared] raise subclasses of `StrictSchemaError` on any
/// schema violation. Unknown keys, missing keys, type mismatches, and unknown
/// discriminator values all surface here.
#[derive(Debug, Error)]
pub enum StrictSchemaError {
    /// A TOML mapping contained a key not in the allowed set.
    #[error("unknown key(s): {0}")]
    UnknownKey(String),

    // TODO: merge with `missing key`
    /// A TOML mapping was missing a required key.
    #[error(" missing requred key(s): {0}")]
    MissianRequiredKey(String),

    /// A TOML value had the wrong Python type after parsing.
    #[error("expected {0}, got {1}")]
    TypeMismatch(String, String),

    /// Type conversion error
    #[error("parse error around '{0}'")]
    ParseError(String),

    /// A `[calc].program` (or `[step].program`) value is not registered.
    #[error("expected")]
    UnknownProgram,

    /// A `[calc].calc_type` (or `[step].calc_type`) is not registered for the program.
    #[error("expected")]
    UnknownCalcType,

    /// A program / calc_type string is not in lowercase canonical form.
    #[error("expected")]
    NonCanonicalName,
}
