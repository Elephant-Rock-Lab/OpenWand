use serde::{Deserialize, Serialize};

use crate::provenance::{MemoryScope, ProvenanceSnapshot};
use crate::ranking::MemoryRankScore;

/// Context retrieved from memory for a given query.
/// Layered by utility — most useful first.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalContext {
    /// Factual statements relevant to the query.
    pub facts: Vec<String>,
    /// Relevant past decisions.
    pub decisions: Vec<String>,
    /// Source episode summaries.
    pub episodes: Vec<String>,
    /// Query metadata.
    pub query_text: String,
    pub total_hits: usize,
}

impl RetrievalContext {
    pub fn empty() -> Self {
        Self {
            facts: vec![],
            decisions: vec![],
            episodes: vec![],
            query_text: String::new(),
            total_hits: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.facts.is_empty() && self.decisions.is_empty() && self.episodes.is_empty()
    }

    /// Format as a context block for LLM injection.
    pub fn to_context_block(&self) -> Option<String> {
        if self.is_empty() {
            return None;
        }

        let mut parts = Vec::new();
        if !self.facts.is_empty() {
            parts.push(format!("## Facts\n{}", self.facts.join("\n")));
        }
        if !self.decisions.is_empty() {
            parts.push(format!("## Past Decisions\n{}", self.decisions.join("\n")));
        }
        if !self.episodes.is_empty() {
            parts.push(format!("## Context\n{}", self.episodes.join("\n")));
        }

        Some(parts.join("\n\n"))
    }
}

// ---------------------------------------------------------------------------
// Ranked retrieval (Wave 02i)
// ---------------------------------------------------------------------------

/// A single ranked memory hit with full provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedMemoryHit {
    pub id: String,
    pub text: String,
    pub score: MemoryRankScore,
    pub source_episode_ids: Vec<String>,
    pub source_trace_ids: Vec<String>,
    pub scope: MemoryScope,
    pub provenance: ProvenanceSnapshot,
    pub confidence_bps: u16,
    /// Human-readable explanation of why this hit ranked where it did.
    pub reason: String,
}

/// Ranked retrieval context — ordered by final score, each hit explainable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedRetrievalContext {
    pub hits: Vec<RankedMemoryHit>,
    pub query_text: String,
    pub total_hits: usize,
}

impl RankedRetrievalContext {
    pub fn empty() -> Self {
        Self {
            hits: vec![],
            query_text: String::new(),
            total_hits: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.hits.is_empty()
    }

    /// Flatten to plain strings for legacy consumers.
    pub fn as_flat_strings(&self) -> Vec<String> {
        self.hits.iter().map(|h| h.text.clone()).collect()
    }

    /// Convert to the legacy RetrievalContext format.
    pub fn to_legacy(&self) -> RetrievalContext {
        let mut facts = Vec::new();
        let mut decisions = Vec::new();
        let mut episodes = Vec::new();

        for hit in &self.hits {
            // Best-effort: use text as-is, no kind info in flat format
            facts.push(hit.text.clone());
        }

        RetrievalContext {
            facts,
            decisions,
            episodes,
            query_text: self.query_text.clone(),
            total_hits: self.total_hits,
        }
    }
}

#[cfg(test)]
mod ranked_tests {
    use super::*;
    use crate::provenance::{MemoryScope, ProvenanceKind, ProvenanceSnapshot};
    use crate::ranking::MemoryRankScore;

    fn test_hit(id: &str, final_bps: u16) -> RankedMemoryHit {
        RankedMemoryHit {
            id: id.to_string(),
            text: format!("Test hit {}", id),
            score: MemoryRankScore {
                relevance_bps: 0,
                provenance_bps: 0,
                scope_bps: 0,
                recency_bps: 0,
                confidence_bps: 0,
                evidence_bps: 0,
                final_bps,
            },
            source_episode_ids: vec!["ep_1".to_string()],
            source_trace_ids: vec!["tr_1".to_string()],
            scope: MemoryScope::Global,
            provenance: ProvenanceSnapshot {
                kind: ProvenanceKind::Unknown,
            },
            confidence_bps: 7000,
            reason: "test".to_string(),
        }
    }

    #[test]
    fn ranked_hit_has_score() {
        let hit = test_hit("a", 8500);
        assert_eq!(8500, hit.score.final_bps);
    }

    #[test]
    fn ranked_hit_has_reason() {
        let hit = test_hit("a", 8500);
        assert!(!hit.reason.is_empty());
    }

    #[test]
    fn ranked_hit_preserves_trace_refs() {
        let hit = test_hit("a", 8500);
        assert!(!hit.source_trace_ids.is_empty());
        assert!(hit.source_trace_ids.contains(&"tr_1".to_string()));
    }

    #[test]
    fn ranked_hit_preserves_episode_refs() {
        let hit = test_hit("a", 8500);
        assert!(!hit.source_episode_ids.is_empty());
        assert!(hit.source_episode_ids.contains(&"ep_1".to_string()));
    }

    #[test]
    fn ranked_context_can_flatten_for_legacy_consumers() {
        let ctx = RankedRetrievalContext {
            hits: vec![test_hit("a", 8500), test_hit("b", 7000)],
            query_text: "test".to_string(),
            total_hits: 2,
        };
        let flat = ctx.as_flat_strings();
        assert_eq!(2, flat.len());
        assert_eq!("Test hit a", flat[0]);
    }

    #[test]
    fn ranked_context_to_legacy_preserves_total() {
        let ctx = RankedRetrievalContext {
            hits: vec![test_hit("a", 8500)],
            query_text: "test".to_string(),
            total_hits: 1,
        };
        let legacy = ctx.to_legacy();
        assert_eq!(1, legacy.total_hits);
        assert_eq!("test", legacy.query_text);
    }
}
