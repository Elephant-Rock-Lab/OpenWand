//! Memory extractors for testing and v0 production use.
//!
//! - `NullExtractor`: produces nothing. Use for CI tests that need the trait.
//! - `HeuristicExtractor`: deterministic rule-matcher for v0 production and testing.
//!
//! **Neither of these is semantic extraction.** They are heuristic placeholders.
//! A real LLM-based extractor will replace HeuristicExtractor in a later wave.

use crate::extractor::MemoryExtractor;
use crate::types::{CandidateMemory, MemoryEpisode};
use async_trait::async_trait;

/// Produces nothing. Use for CI tests that need a MemoryExtractor
/// but shouldn't produce any candidates.
pub struct NullExtractor;

#[async_trait]
impl MemoryExtractor for NullExtractor {
    async fn extract(&self, _episodes: &[MemoryEpisode]) -> Vec<CandidateMemory> {
        vec![]
    }
}

/// Deterministic heuristic rule-matcher for v0 production and testing.
///
/// This is NOT semantic extraction. It checks for keyword triggers in user
/// messages and promotes the entire message as a candidate fact if any
/// trigger is found.
///
/// Trigger keywords: "remember", "always", "never", "i prefer", "my name is"
///
/// Limitations:
/// - No understanding of semantics or context
/// - Promotes the entire message (not extracted facts from it)
/// - Cannot distinguish important from trivial uses of trigger words
/// - Will be replaced by an LLM-based extractor in a later wave
///
/// Confidence is fixed at 0.9 — still subject to deterministic acceptance
/// rules in the MemoryStore.
pub struct HeuristicExtractor;

#[async_trait]
impl MemoryExtractor for HeuristicExtractor {
    async fn extract(&self, episodes: &[MemoryEpisode]) -> Vec<CandidateMemory> {
        let mut candidates = Vec::new();

        for episode in episodes {
            if episode.role != crate::types::EpisodeRole::User {
                continue;
            }

            let text = episode.content.to_lowercase();
            let triggered = text.contains("remember")
                || text.contains("always")
                || text.contains("never")
                || text.contains("i prefer")
                || text.contains("my name is");

            if triggered {
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

// Backward-compatible alias for tests that reference the old name.
// Deprecated: use HeuristicExtractor instead.
#[deprecated(note = "Renamed to HeuristicExtractor. The old name was misleading.")]
pub type KeywordExtractor = HeuristicExtractor;
