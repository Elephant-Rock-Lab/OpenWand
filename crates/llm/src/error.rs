//! LLM error types.
//!
//! Normalized across all providers. Session failure taxonomy:
//! LLM failure is recoverable/retryable, unlike trace append failure (hard stop).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum LlmError {
    /// Network failure. `retryable` indicates whether retry is safe.
    #[error("Network error: {message}")]
    Network {
        message: String,
        retryable: bool,
    },

    /// Provider returned an error (rate limit, content filter, etc.)
    #[error("Provider error ({provider}): {message}")]
    Provider {
        provider: String,
        message: String,
        retryable: bool,
    },

    /// The request was invalid (bad model name, missing required field, etc.)
    #[error("Invalid request: {message}")]
    RequestInvalid {
        message: String,
    },

    /// Failed to decode provider response.
    #[error("Decode error: {message}")]
    Decode {
        message: String,
    },

    /// Stream error during delivery. `partial` = some deltas were delivered.
    #[error("Stream error: {message}")]
    Stream {
        message: String,
        partial: bool,
    },

    /// Request was cancelled (by user or circuit breaker).
    #[error("Cancelled")]
    Cancelled,

    /// Feature not supported by this provider.
    #[error("Unsupported: {provider} does not support {feature}")]
    Unsupported {
        provider: String,
        feature: String,
    },
}

impl LlmError {
    /// Whether the caller should retry this request.
    pub fn retryable(&self) -> bool {
        match self {
            Self::Network { retryable, .. } => *retryable,
            Self::Provider { retryable, .. } => *retryable,
            Self::Stream { .. } => true,
            Self::Cancelled => false,
            Self::RequestInvalid { .. } => false,
            Self::Decode { .. } => false,
            Self::Unsupported { .. } => false,
        }
    }

    /// Returns a safe display string that doesn't leak API keys or internal details.
    pub fn safe_display(&self) -> String {
        match self {
            Self::Network { message, .. } => format!("Network error: {message}"),
            Self::Provider { provider, message, .. } => {
                format!("Provider error ({provider}): {message}")
            }
            Self::RequestInvalid { message } => format!("Invalid request: {message}"),
            Self::Decode { message } => format!("Decode error: {message}"),
            Self::Stream { message, .. } => format!("Stream error: {message}"),
            Self::Cancelled => "Cancelled".into(),
            Self::Unsupported { provider, feature } => {
                format!("Unsupported: {provider} does not support {feature}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_error_display_safe() {
        let err = LlmError::Network {
            message: "connection refused".into(),
            retryable: true,
        };
        assert!(err.safe_display().contains("connection refused"));
        assert!(err.retryable());

        let err = LlmError::Provider {
            provider: "openai".into(),
            message: "rate limited".into(),
            retryable: true,
        };
        assert!(err.safe_display().contains("openai"));
        assert!(err.retryable());

        let err = LlmError::RequestInvalid {
            message: "bad model".into(),
        };
        assert!(!err.retryable());

        let err = LlmError::Unsupported {
            provider: "ollama".into(),
            feature: "vision".into(),
        };
        assert!(!err.retryable());
        assert!(err.safe_display().contains("vision"));
    }
}
