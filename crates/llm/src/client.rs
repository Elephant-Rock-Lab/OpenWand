//! LLM client trait — the provider-normalized boundary.
//!
//! This is the only LLM abstraction that leaves openwand-llm.
//! Session calls `LlmClient`, never Rig directly.

use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

use crate::error::LlmError;
use crate::request::{LlmCapabilities, LlmTarget};
use crate::response::{LlmDelta, LlmResponse};
use crate::request::LlmRequest;

/// Stream of LLM deltas. All errors go through Result Err, never through a delta variant.
pub type LlmStream = Pin<Box<dyn Stream<Item = Result<LlmDelta, LlmError>> + Send>>;

/// OpenWand's LLM client trait. Object-safe.
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Stream a completion request.
    async fn chat_stream(&self, request: LlmRequest) -> Result<LlmStream, LlmError>;

    /// Non-streaming completion. For tests and fallback.
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError>;

    /// Check that a specific provider target is reachable.
    async fn health_check(&self, target: &LlmTarget) -> Result<(), LlmError>;

    /// Report capabilities for a given target.
    fn capabilities(&self, target: &LlmTarget) -> LlmCapabilities;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prove LlmClient is object-safe.
    #[test]
    fn llm_client_trait_object_compiles() {
        fn _uses_arc_dyn(_store: std::sync::Arc<dyn LlmClient>) {}
    }
}
