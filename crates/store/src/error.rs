//! Store error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Read error: {message}")]
    Read { message: String },

    #[error("Write error: {message}")]
    Write { message: String },

    #[error("Writer channel closed")]
    WriterClosed,
}

#[cfg(feature = "sqlite")]
impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        StoreError::Database(e.to_string())
    }
}

#[cfg(feature = "sqlite")]
impl From<StoreError> for openwand_trace::TraceError {
    fn from(e: StoreError) -> Self {
        openwand_trace::TraceError::Storage(e.to_string())
    }
}
