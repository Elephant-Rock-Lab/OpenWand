//! Commit 4 — Observation invariant tests.
//!
//! Proves:
//! - Observation episodes are never promoted to accepted claims
//! - Tool/shell/git outputs remain evidence only
//! - Test results can be deterministic evidence supporting claims
//! - LLM candidates cannot support claims without promotion

use openwand_memory::evidence::EvidenceKind;

#[test]
fn raw_observation_is_not_accepted_state() {
    assert!(!EvidenceKind::RawObservation.is_accepted_state());
}

#[test]
fn deterministic_evidence_is_not_accepted_state() {
    assert!(!EvidenceKind::DeterministicEvidence.is_accepted_state());
}

#[test]
fn test_result_can_support_claim_but_is_not_claim() {
    let kind = EvidenceKind::DeterministicEvidence;
    assert!(kind.can_support_claim(), "test results can support claims");
    assert!(!kind.is_accepted_state(), "but are not claims themselves");
}

#[test]
fn tool_result_can_support_claim_but_is_not_claim() {
    let kind = EvidenceKind::RawObservation;
    assert!(!kind.can_support_claim(), "raw tool output cannot support claims");
    assert!(!kind.is_accepted_state());
}

#[test]
fn git_output_can_support_claim_but_is_not_claim() {
    let kind = EvidenceKind::DeterministicEvidence;
    assert!(kind.can_support_claim(), "git output can support claims");
    assert!(!kind.is_accepted_state(), "but is not an accepted claim");
}

#[test]
fn llm_candidate_cannot_support_claim_without_promotion() {
    let kind = EvidenceKind::LlmExtractedCandidate;
    assert!(!kind.can_support_claim(), "LLM candidates must be promoted first");
    assert!(!kind.is_accepted_state());
}
