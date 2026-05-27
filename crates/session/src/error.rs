use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("trace append failed: {0}")]
    Trace(#[from] openwand_trace::TraceError),

    #[error("LLM error: {0}")]
    Llm(#[from] openwand_llm::LlmError),

    #[error("policy error: {0}")]
    Policy(String),

    #[error("projection stale marker failed: {0}")]
    ProjectionStaleMarker(String),

    #[error("a run is already active for this session")]
    RunAlreadyActive,

    #[error("session cancelled")]
    Cancelled,

    #[error("session internal error: {0}")]
    Internal(String),
}
