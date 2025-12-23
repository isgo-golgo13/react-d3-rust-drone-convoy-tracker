//! Database error types

use thiserror::Error;

/// Database errors
#[derive(Error, Debug)]
pub enum DbError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Duplicate entry: {0}")]
    Duplicate(String),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Timeout: {0}")]
    Timeout(String),
}

impl DbError {
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    pub fn query(msg: impl Into<String>) -> Self {
        Self::Query(msg.into())
    }
}

pub type DbResult<T> = Result<T, DbError>;
