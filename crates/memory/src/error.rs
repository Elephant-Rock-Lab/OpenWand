use thiserror::Error;

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("memory query failed: {0}")]
    QueryFailed(String),

    #[error("memory store unavailable: {0}")]
    Unavailable(String),

    #[error("memory internal error: {0}")]
    Internal(String),
}
