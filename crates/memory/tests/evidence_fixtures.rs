//! Commit 8 — Evidence regression fixtures.
//!
//! Loads JSON fixture files and verifies evidence ranking behavior.

use std::fs;
use std::path::Path;

use openwand_memory::evidence::EvidenceKind;
use openwand_memory::ranking::{RankingWeights, compute_final_score, evidence_bps_from_kind, MemoryRankScore};
use openwand_memory::supersession::{RetrievalMode, supersession_penalty};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct FixtureRecord {
    id: String,
    text: String,
    evidence_kind: String,
    #[allow(dead_code)]
    scope: String,
    confidence_bps: u16,
}

#[derive(Debug, Deserialize)]
struct Fixture {
    name: String,
    records: Vec<FixtureRecord>,
    #[allow(dead_code)]
    query: String,
    mode: String,
    expected_order: Vec<String>,
    #[allow(dead_code)]
    expected_excluded: Vec<String>,
    expected_evidence_kinds: std::collections::HashMap<String, String>,
}

fn load_fixture(name: &str) -> Fixture {
    let path = Path::new("tests/fixtures/evidence").join(format!("{}.json", name));
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {:?}: {}", path, e));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("failed to parse fixture {:?}: {}", path, e))
}

fn parse_evidence_kind(s: &str) -> EvidenceKind {
    match s {
        "AcceptedClaim" => EvidenceKind::AcceptedClaim,
        "UserStatedClaim" => EvidenceKind::UserStatedClaim,
        "DeterministicEvidence" => EvidenceKind::DeterministicEvidence,
        "RawObservation" => EvidenceKind::RawObservation,
        "LlmExtractedCandidate" => EvidenceKind::LlmExtractedCandidate,
        "SupersededClaim" => EvidenceKind::SupersededClaim,
        "ConflictingClaim" => EvidenceKind::ConflictingClaim,
        _ => panic!("unknown evidence kind: {}", s),
    }
}

fn parse_mode(s: &str) -> RetrievalMode {
    match s {
        "Default" => RetrievalMode::Default,
        "CurrentState" => RetrievalMode::CurrentState,
        "ChangeHistory" => RetrievalMode::ChangeHistory,
        "ConflictSearch" => RetrievalMode::ConflictSearch,
        _ => panic!("unknown mode: {}", s),
    }
}

fn score_record(rec: &FixtureRecord, mode: RetrievalMode) -> (String, u16) {
    let kind = parse_evidence_kind(&rec.evidence_kind);
    let evidence_bps = evidence_bps_from_kind(&kind);
    let is_superseded = matches!(kind, EvidenceKind::SupersededClaim);
    let penalty = supersession_penalty(is_superseded, mode);

    let score = MemoryRankScore {
        relevance_bps: 8000,
        provenance_bps: 7000,
        scope_bps: 7000,
        recency_bps: 7000,
        confidence_bps: rec.confidence_bps,
        evidence_bps: if evidence_bps > penalty { evidence_bps - penalty } else { 0 },
        verification_bps: 0,
        final_bps: 0,
    };
    let weights = RankingWeights::default();
    let final_score = compute_final_score(&score, &weights);
    (rec.id.clone(), final_score)
}

fn run_fixture(name: &str) {
    let fixture = load_fixture(name);
    let mode = parse_mode(&fixture.mode);

    // Score each record
    let mut scored: Vec<(String, u16)> = fixture.records.iter()
        .map(|r| score_record(r, mode))
        .collect();

    // Sort descending by score
    scored.sort_by(|a, b| b.1.cmp(&a.1));
    let actual_order: Vec<String> = scored.iter().map(|(id, _)| id.clone()).collect();

    // Verify expected order
    assert_eq!(
        fixture.expected_order, actual_order,
        "fixture '{}' ordering mismatch",
        fixture.name
    );

    // Verify evidence kinds
    for (record_id, expected_kind_str) in &fixture.expected_evidence_kinds {
        let expected_kind = parse_evidence_kind(expected_kind_str);
        let rec = fixture.records.iter().find(|r| &r.id == record_id)
            .unwrap_or_else(|| panic!("fixture '{}' has no record '{}'", fixture.name, record_id));
        let actual_kind = parse_evidence_kind(&rec.evidence_kind);
        assert_eq!(
            expected_kind, actual_kind,
            "fixture '{}' record '{}' evidence kind mismatch",
            fixture.name, record_id
        );
    }
}

#[test]
fn fixture_observation_not_claim() {
    run_fixture("observation_not_claim");
}

#[test]
fn fixture_duplicate_observation() {
    run_fixture("duplicate_observation");
}

#[test]
fn fixture_supersession_chain() {
    run_fixture("supersession_chain");
}

#[test]
fn fixture_conflicting_claims() {
    run_fixture("conflicting_claims");
}

#[test]
fn fixture_deterministic_evidence_vs_raw_observation() {
    run_fixture("deterministic_evidence_vs_raw_observation");
}

#[test]
fn fixture_user_claim_vs_llm_candidate() {
    run_fixture("user_claim_vs_llm_candidate");
}
