//! Error type to simplify propagating different error types.

#[cfg(feature = "sqlx")]
use sqlx::Error as SqlxError;

#[cfg(not(feature = "sqlx"))]
/// Represent all ways a method can fail within SQLx.
///
/// I.e. not at all since sqlx isn't a dependency.
#[derive(Debug, thiserror::Error)]
pub enum SqlxError {}

/// Error type to simplify propagating different error types.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error returned from Sqlx
    #[error("sqlx error: {0}")]
    SqlxError(#[from] SqlxError),

    /// Error for pointing to configuration errors.
    #[error("configuration error: {0}")]
    ConfigurationError(String),

    /// DecodeError
    #[error("decode error: {0}")]
    DecodeError(String),

    /// SQL building error
    #[error("sql error: {0}")]
    SQLBuildError(#[from] rorm_sql::error::Error),
}
