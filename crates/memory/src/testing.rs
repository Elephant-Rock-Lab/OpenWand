//! Mock extractor for CI. Extracts nothing — always returns empty.
//!
//! Use for tests that need a MemoryExtractor but shouldn't depend on LLM.

use crate::extractor::MemoryExtractor;
use crate::types::{CandidateMemory, MemoryEpisode};
use async_trait::async_trait;

/// Mock extractor that extracts nothing.
pub struct NullExtractor;

#[async_trait]
impl MemoryExtractor for NullExtractor {
    async fn extract(&self, _episodes: &[MemoryEpisode]) -> Vec<CandidateMemory> {
        vec![]
    }
}

/// Deterministic extractor for testing. Extracts user messages as facts
/// if they contain "remember" or "always" or "never".
pub struct KeywordExtractor;

#[async_trait]
impl MemoryExtractor for KeywordExtractor {
    async fn extract(&self, episodes: &[MemoryEpisode]) -> Vec<CandidateMemory> {
        let mut candidates = Vec::new();

        for episode in episodes {
            if episode.role != crate::types::EpisodeRole::User {
                continue;
            }

            let text = episode.content.to_lowercase();
            let is_memory_worthy = text.contains("remember")
                || text.contains("always")
                || text.contains("never")
                || text.contains("i prefer")
                || text.contains("my name is");

            if is_memory_worthy {
                candidates.push(CandidateMemory {
                    claim: episode.content.clone(),
                    kind: crate::types::CandidateKind::Fact,
                    confidence: 0.9,
                    source_episode_ids: vec![episode.episode_id.clone()],
                });
            }
        }

        candidates
    }
}
