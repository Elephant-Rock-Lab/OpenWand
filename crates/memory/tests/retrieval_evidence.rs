//! Commit 3 — Retrieval hit evidence labeling tests.

use openwand_memory::evidence::EvidenceKind;
use openwand_memory::provenance::{MemoryScope, ProvenanceKind, ProvenanceSnapshot};
use openwand_memory::ranking::{MemoryRankScore, RankingWeights, compute_final_score, evidence_bps_from_kind};
use openwand_memory::retrieval::{RankedMemoryHit, RankedRetrievalContext};

fn hit_with_evidence(id: &str, kind: EvidenceKind) -> RankedMemoryHit {
    let evidence_bps = evidence_bps_from_kind(&kind);
    RankedMemoryHit {
        id: id.to_string(),
        text: format!("Test claim {}", id),
        score: MemoryRankScore {
            relevance_bps: 8000,
            provenance_bps: 7000,
            scope_bps: 7000,
            recency_bps: 7000,
            confidence_bps: 7000,
            evidence_bps,
            final_bps: 0,
        },
        evidence_kind: kind,
        source_episode_ids: vec![],
        source_trace_ids: vec![],
        scope: MemoryScope::Global,
        provenance: ProvenanceSnapshot { kind: ProvenanceKind::Unknown },
        confidence_bps: 7000,
        reason: "test".to_string(),
    }
}

#[test]
fn ranked_hit_carries_evidence_kind() {
    let hit = hit_with_evidence("a", EvidenceKind::AcceptedClaim);
    assert_eq!(EvidenceKind::AcceptedClaim, hit.evidence_kind);
}

#[test]
fn user_stated_claim_ranks_above_llm_candidate() {
    let weights = RankingWeights::default();
    let user = hit_with_evidence("a", EvidenceKind::UserStatedClaim);
    let llm = hit_with_evidence("b", EvidenceKind::LlmExtractedCandidate);
    let user_score = compute_final_score(&user.score, &weights);
    let llm_score = compute_final_score(&llm.score, &weights);
    assert!(user_score > llm_score, "UserStatedClaim must rank above LlmExtractedCandidate");
}

#[test]
fn deterministic_evidence_ranks_above_raw_observation() {
    let weights = RankingWeights::default();
    let det = hit_with_evidence("a", EvidenceKind::DeterministicEvidence);
    let raw = hit_with_evidence("b", EvidenceKind::RawObservation);
    let det_score = compute_final_score(&det.score, &weights);
    let raw_score = compute_final_score(&raw.score, &weights);
    assert!(det_score > raw_score, "DeterministicEvidence must rank above RawObservation");
}

#[test]
fn superseded_claim_is_penalized_by_default() {
    let weights = RankingWeights::default();
    let active = hit_with_evidence("a", EvidenceKind::AcceptedClaim);
    let superseded = hit_with_evidence("b", EvidenceKind::SupersededClaim);
    let active_score = compute_final_score(&active.score, &weights);
    let superseded_score = compute_final_score(&superseded.score, &weights);
    assert!(active_score > superseded_score, "SupersededClaim must be penalized");
}

#[test]
fn conflicting_claim_is_labeled() {
    let hit = hit_with_evidence("a", EvidenceKind::ConflictingClaim);
    assert_eq!(EvidenceKind::ConflictingClaim, hit.evidence_kind);
    assert!(!hit.evidence_kind.is_accepted_state());
}

#[test]
fn legacy_flattening_preserves_text_only() {
    let ctx = RankedRetrievalContext {
        hits: vec![hit_with_evidence("a", EvidenceKind::RawObservation)],
        query_text: "test".to_string(),
        total_hits: 1,
    };
    let flat = ctx.as_flat_strings();
    assert_eq!(1, flat.len());
    assert_eq!("Test claim a", flat[0]);
    // Flat strings don't carry evidence kind — that's intentional
}
