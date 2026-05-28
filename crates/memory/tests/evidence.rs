//! Commit 1 — Evidence kind model tests.

use openwand_memory::evidence::EvidenceKind;

#[test]
fn evidence_kind_default_is_accepted_claim() {
    assert_eq!(EvidenceKind::default(), EvidenceKind::AcceptedClaim);
}

#[test]
fn user_stated_claim_is_accepted_state() {
    assert!(EvidenceKind::UserStatedClaim.is_accepted_state());
}

#[test]
fn deterministic_evidence_is_not_accepted_state() {
    assert!(!EvidenceKind::DeterministicEvidence.is_accepted_state());
}

#[test]
fn raw_observation_is_not_accepted_state() {
    assert!(!EvidenceKind::RawObservation.is_accepted_state());
}

#[test]
fn llm_candidate_is_not_accepted_state() {
    assert!(!EvidenceKind::LlmExtractedCandidate.is_accepted_state());
}

#[test]
fn superseded_claim_is_not_current_state() {
    assert!(!EvidenceKind::SupersededClaim.is_accepted_state());
}

#[test]
fn conflicting_claim_is_not_current_state() {
    assert!(!EvidenceKind::ConflictingClaim.is_accepted_state());
}
