//! Deterministic memory ranking.
//!
//! All scores in basis points (0–10000) for deterministic testing.
//! No floating-point ranking drift.

use serde::{Deserialize, Serialize};

/// Deterministic ranking score for a memory hit.
/// All fields in basis points (0–10000).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryRankScore {
    pub relevance_bps: u16,
    pub provenance_bps: u16,
    pub scope_bps: u16,
    pub recency_bps: u16,
    pub confidence_bps: u16,
    pub evidence_bps: u16,
    pub verification_bps: u16,
    pub final_bps: u16,
}

/// Weight configuration for ranking components.
/// Values should sum to 10000 (100%).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RankingWeights {
    pub relevance: u16,
    pub provenance: u16,
    pub scope: u16,
    pub recency: u16,
    pub confidence: u16,
    pub evidence: u16,
    pub verification: u16,
}

impl Default for RankingWeights {
    fn default() -> Self {
        Self {
            relevance: 3500,
            provenance: 2000,
            scope: 1500,
            recency: 1000,
            confidence: 1000,
            evidence: 1000,
            verification: 0, // pre-02r: no verification component
        }
    }
}

impl RankingWeights {
    pub fn sum(&self) -> u32 {
        self.relevance as u32
            + self.provenance as u32
            + self.scope as u32
            + self.recency as u32
            + self.confidence as u32
            + self.evidence as u32
            + self.verification as u32
    }
}

/// Compute the weighted final score from individual components.
/// Returns a value capped at 10000.
pub fn compute_final_score(score: &MemoryRankScore, weights: &RankingWeights) -> u16 {
    let raw: u32 = (score.relevance_bps as u32 * weights.relevance as u32 / 10000)
        + (score.provenance_bps as u32 * weights.provenance as u32 / 10000)
        + (score.scope_bps as u32 * weights.scope as u32 / 10000)
        + (score.recency_bps as u32 * weights.recency as u32 / 10000)
        + (score.confidence_bps as u32 * weights.confidence as u32 / 10000)
        + (score.evidence_bps as u32 * weights.evidence as u32 / 10000)
        + (score.verification_bps as u32 * weights.verification as u32 / 10000);
    raw.min(10000) as u16
}

/// Derive evidence_bps component from EvidenceKind.
/// Maps directly to the authority ranking of the evidence kind.
pub fn evidence_bps_from_kind(kind: &crate::evidence::EvidenceKind) -> u16 {
    kind.authority_bps()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranking_weights_sum_to_10000() {
        let w = RankingWeights::default();
        assert_eq!(10000, w.sum());
    }

    #[test]
    fn ranking_score_is_deterministic() {
        let score = MemoryRankScore {
            relevance_bps: 8000,
            provenance_bps: 7000,
            scope_bps: 6000,
            recency_bps: 5000,
            confidence_bps: 9000,
            evidence_bps: 8000,
            verification_bps: 0,
            final_bps: 0,
        };
        let weights = RankingWeights::default();

        let result1 = compute_final_score(&score, &weights);
        let result2 = compute_final_score(&score, &weights);
        assert_eq!(result1, result2, "same inputs must produce same output");
    }

    #[test]
    fn ranking_relevance_wins_when_all_else_equal() {
        let weights = RankingWeights::default();
        let low = MemoryRankScore {
            relevance_bps: 5000,
            provenance_bps: 7000,
            scope_bps: 7000,
            recency_bps: 7000,
            confidence_bps: 7000,
            evidence_bps: 7000,
            verification_bps: 0,
            final_bps: 0,
        };
        let high = MemoryRankScore {
            relevance_bps: 9000,
            provenance_bps: 7000,
            scope_bps: 7000,
            recency_bps: 7000,
            confidence_bps: 7000,
            evidence_bps: 7000,
            verification_bps: 0,
            final_bps: 0,
        };
        assert!(compute_final_score(&high, &weights) > compute_final_score(&low, &weights));
    }

    #[test]
    fn ranking_provenance_breaks_tie() {
        let weights = RankingWeights::default();
        let low_prov = MemoryRankScore {
            relevance_bps: 8000,
            provenance_bps: 3000,
            scope_bps: 7000,
            recency_bps: 7000,
            confidence_bps: 7000,
            evidence_bps: 7000,
            verification_bps: 0,
            final_bps: 0,
        };
        let high_prov = MemoryRankScore {
            relevance_bps: 8000,
            provenance_bps: 9000,
            scope_bps: 7000,
            recency_bps: 7000,
            confidence_bps: 7000,
            evidence_bps: 7000,
            verification_bps: 0,
            final_bps: 0,
        };
        assert!(compute_final_score(&high_prov, &weights) > compute_final_score(&low_prov, &weights));
    }

    #[test]
    fn ranking_scope_breaks_tie() {
        let weights = RankingWeights::default();
        let global = MemoryRankScore {
            relevance_bps: 8000,
            provenance_bps: 7000,
            scope_bps: 3000,
            recency_bps: 7000,
            confidence_bps: 7000,
            evidence_bps: 7000,
            verification_bps: 0,
            final_bps: 0,
        };
        let project = MemoryRankScore {
            relevance_bps: 8000,
            provenance_bps: 7000,
            scope_bps: 9000,
            recency_bps: 7000,
            confidence_bps: 7000,
            evidence_bps: 7000,
            verification_bps: 0,
            final_bps: 0,
        };
        assert!(compute_final_score(&project, &weights) > compute_final_score(&global, &weights));
    }

    #[test]
    fn ranking_score_saturates_at_10000() {
        let score = MemoryRankScore {
            relevance_bps: 10000,
            provenance_bps: 10000,
            scope_bps: 10000,
            recency_bps: 10000,
            confidence_bps: 10000,
            evidence_bps: 10000,
            verification_bps: 0,
            final_bps: 0,
        };
        let weights = RankingWeights::default();
        let result = compute_final_score(&score, &weights);
        assert_eq!(10000, result, "max inputs must saturate at 10000");
    }
}
