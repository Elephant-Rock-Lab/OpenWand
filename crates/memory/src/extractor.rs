//! Memory extractor trait.
//!
//! The extractor proposes candidate memories from episodes.
//! It does NOT write to the memory store — acceptance is separate.
//!
//! Invariant: LLM extraction proposes. Deterministic memory policy accepts.

use crate::types::{CandidateMemory, MemoryEpisode};
use async_trait::async_trait;

/// Trait for memory extractors. Proposes candidates from episodes.
#[async_trait]
pub trait MemoryExtractor: Send + Sync {
    /// Extract candidate memories from a set of episodes.
    /// Returns candidates that may or may not be accepted.
    /// Extractors MUST NOT write to the memory store directly.
    async fn extract(&self, episodes: &[MemoryEpisode]) -> Vec<CandidateMemory>;
}
