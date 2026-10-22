use sqlx::Error as SqlxError;
use thiserror::Error;

/// Errors shared by all the repositories.
#[derive(Debug, Error)]
pub enum RepoError {
    #[error("not found")]
    NotFound,
    #[error("already exists")]
    AlreadyExists,
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
    #[error("referenced row does not exist")]
    InvalidReference,
    #[error("database error: {0}")]
    Database(#[source] SqlxError),
}

impl From<SqlxError> for RepoError {
    fn from(error: SqlxError) -> Self {
        match &error {
            SqlxError::Database(db_error) if db_error.is_unique_violation() => Self::AlreadyExists,
            SqlxError::Database(e) if e.is_foreign_key_violation() => Self::InvalidReference,
            _ => Self::Database(error),
        }
    }
}
