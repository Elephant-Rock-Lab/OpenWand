//! Trace error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TraceError {
    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Entry not found: {0}")]
    NotFound(String),

    #[error("Duplicate idempotency key: {0}")]
    DuplicateIdempotencyKey(String),

    #[error("Integrity error: {0}")]
    Integrity(String),

    #[error("Projection error: {0}")]
    Projection(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Already initialized")]
    AlreadyInitialized,
}
