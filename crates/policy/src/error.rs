//! Policy error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("Rule evaluation error: {0}")]
    RuleEvaluation(String),

    #[error("Unknown tool: {0}")]
    UnknownTool(String),

    #[error("Invalid rule configuration: {0}")]
    InvalidConfig(String),

    #[error("Internal policy error: {0}")]
    Internal(String),
}

impl PolicyError {
    /// Returns a safe message suitable for trace events and user display.
    /// Never leaks internal paths, keys, or implementation details.
    pub fn safe_message(&self) -> String {
        match self {
            Self::RuleEvaluation(msg) => format!("Rule evaluation error: {msg}"),
            Self::UnknownTool(name) => format!("Unknown tool: {name}"),
            Self::InvalidConfig(msg) => format!("Invalid rule configuration: {msg}"),
            Self::Internal(_) => "Internal policy error".into(),
        }
    }
}
