/**
Error type to simplify propagating different error types.
 */
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error returned from Sqlx
    #[error("sqlx error: {0}")]
    SqlxError(#[from] sqlx::Error),

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
