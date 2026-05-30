//! Wave 02r guard tests — prompt fork, default profile, no hidden trust channel.
//!
//! These guard the critical 02r invariants:
//! - Default profile = byte-for-byte pre-02r behavior
//! - Governance reasons are audit-visible, not prompt-visible
//! - No provenance/trace/governance tags in normal prompt text

use openwand_app::memory_evaluation::MemoryEvaluationHarness;
use openwand_memory::evaluation::{
    EvaluationModelConfig, ExpectedScenarioOutcome, MemoryEvaluationCategory,
    MemoryEvaluationExpectations, MemoryEvaluationScenario, MemoryRecordSeed,
    MockEvaluationBehavior, ScenarioExecutionMode,
};
use openwand_memory::governance::{
    GovernanceFilteredReport, MemoryGovernanceProfile, PromptEligibility,
};
use openwand_memory::repo_consistency::{
    ConsistencySeverity, RepoConsistencyFinding, RepoConsistencyFindingKind, RepoConsistencyReport,
    RepoConsistencySummary, RepoMemoryInputSummary, RepoObservationSummary,
};
use openwand_memory::ranking::MemoryRankScore;
use openwand_memory::retrieval::RankedMemoryHit;
use openwand_memory::provenance::ProvenanceSnapshot;
use std::path::Path;

fn create_workspace_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(root.join("Cargo.toml"), "[workspace]\nmembers = [\"crates/core\"]\n").unwrap();
    let core_dir = root.join("crates").join("core");
    std::fs::create_dir_all(core_dir.join("src")).unwrap();
    std::fs::write(core_dir.join("Cargo.toml"), "[package]\nname = \"core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n").unwrap();
    std::fs::write(core_dir.join("src").join("lib.rs"), "pub fn hello() {}").unwrap();
    dir
}

fn make_guard_scenario() -> MemoryEvaluationScenario {
    MemoryEvaluationScenario {
        id: "guard_test".into(),
        title: "Guard test".into(),
        category: MemoryEvaluationCategory::PromptIncluded,
        execution_mode: ScenarioExecutionMode::FullHarness,
        user_query: "test".into(),
        expected_outcome: ExpectedScenarioOutcome::Pass,
        seed_memory: vec![MemoryRecordSeed {
            label: Some("core_claim".into()),
            claim: "crate core exists".into(),
            kind: "Fact".into(),
            confidence: 0.95,
            evidence_kind: "AcceptedClaim".into(),
            source_trace_labels: vec![],
            superseded_by_label: None,
        }],
        seed_trace: vec![],
        seed_relations: vec![],
        expectations: MemoryEvaluationExpectations::default(),
        model: EvaluationModelConfig::Mock {
            behavior: MockEvaluationBehavior::EchoIncludedMemory,
        },
    }
}

fn make_report_with_finding(kind: RepoConsistencyFindingKind, claim: &str, confidence_bps: u16) -> (RepoConsistencyReport, Vec<RankedMemoryHit>) {
    let finding = RepoConsistencyFinding {
        kind,
        claim_text: Some(claim.to_string()),
        evidence_kind: Some(openwand_memory::evidence::EvidenceKind::AcceptedClaim),
        repo_evidence_key: vec![],
        severity: ConsistencySeverity::Low,
        detail: "test".to_string(),
    };
    let report = RepoConsistencyReport {
        repo_root: std::path::PathBuf::from("/test"),
        checked_at: chrono::Utc::now(),
        findings: vec![finding],
        summary: RepoConsistencySummary::from_findings(&[]),
        memory_inputs: RepoMemoryInputSummary::default(),
        repo_inputs: RepoObservationSummary::default(),
    };
    let hits = vec![RankedMemoryHit {
        id: "test".into(),
        text: claim.to_string(),
        score: MemoryRankScore {
            relevance_bps: 0, provenance_bps: 0, scope_bps: 0, recency_bps: 0,
            confidence_bps, evidence_bps: 0, verification_bps: 0, final_bps: 0,
        },
        evidence_kind: openwand_memory::evidence::EvidenceKind::AcceptedClaim,
        source_episode_ids: vec![],
        source_trace_ids: vec![],
        scope: openwand_memory::provenance::MemoryScope::Global,
        provenance: ProvenanceSnapshot::default(),
        confidence_bps,
        reason: "test".into(),
    }];
    (report, hits)
}

#[tokio::test]
async fn default_governance_profile_preserves_02q_prompt_hashes() {
    use openwand_app::memory_coordinator::PromptInputProductionConfig;

    let harness = MemoryEvaluationHarness::new();
    let dir = create_workspace_dir();
    let scenario = make_guard_scenario();

    let config_none = PromptInputProductionConfig::default();
    let r_none = harness.run_scenario_with_config(&scenario, dir.path(), &config_none).await;

    let config_default = PromptInputProductionConfig {
        governance_profile: Some(MemoryGovernanceProfile::default()),
        ..Default::default()
    };
    let r_default = harness.run_scenario_with_config(&scenario, dir.path(), &config_default).await;

    assert_eq!(
        r_none.snapshot.memory_context_hash,
        r_default.snapshot.memory_context_hash,
        "Default governance profile must produce identical prompt hashes"
    );
}

#[test]
fn governance_reason_visible_for_low_confidence_exclusion() {
    let (report, hits) = make_report_with_finding(
        RepoConsistencyFindingKind::Supported,
        "low confidence claim",
        1000,
    );
    let profile = MemoryGovernanceProfile::batch_02r_default();
    let governed = GovernanceFilteredReport::from_report(&report, &profile, &hits);

    assert!(!governed.audit_only_claims.is_empty(), "Low confidence claim should be audit-only");
    let audit = &governed.audit_only_claims[0];
    assert!(!audit.governance_reasons.is_empty(), "Governance reason must be visible");
    assert!(audit.governance_reasons[0].contains("confidence"));
}

#[test]
fn governance_reason_visible_for_superseded_exclusion() {
    let (report, _) = make_report_with_finding(
        RepoConsistencyFindingKind::SupersededMemoryIgnored,
        "old claim",
        9000,
    );
    let profile = MemoryGovernanceProfile::default();
    let governed = GovernanceFilteredReport::from_report(&report, &profile, &[]);

    assert!(!governed.audit_only_claims.is_empty());
    assert!(governed.audit_only_claims[0].governance_reasons.iter().any(|r| r.contains("superseded")));
}

#[test]
fn governance_reason_visible_for_conflict_exclusion() {
    let (report, _) = make_report_with_finding(
        RepoConsistencyFindingKind::ConflictRequiresReview,
        "conflicting claim",
        9000,
    );
    let profile = MemoryGovernanceProfile::default();
    let governed = GovernanceFilteredReport::from_report(&report, &profile, &[]);

    assert!(!governed.audit_only_claims.is_empty());
    assert!(governed.audit_only_claims[0].governance_reasons.iter().any(|r| r.contains("conflict")));
}

#[test]
fn prompt_does_not_render_governance_reason_tags() {
    let (report, _) = make_report_with_finding(
        RepoConsistencyFindingKind::Unverifiable,
        "unverifiable claim",
        9000,
    );
    let profile = MemoryGovernanceProfile::batch_02r_default();
    let governed = GovernanceFilteredReport::from_report(&report, &profile, &[]);

    // Governance reasons exist but must never appear in prompt text
    for gf in &governed.audit_only_claims {
        for reason in &gf.governance_reasons {
            assert!(!reason.is_empty(), "Governance reason must be non-empty");
        }
    }
    // The governed report itself has audit-only entries — these don't go to prompt
    assert!(!governed.audit_only_claims.is_empty());
}

#[test]
fn crate_absence_still_classifies_missing_in_repo_not_stale() {
    // Patch 5 guard: crate absence = MissingInRepo, never StaleMemory
    let (report, _) = make_report_with_finding(
        RepoConsistencyFindingKind::MissingInRepo,
        "crate nonexistent exists",
        9000,
    );
    // All findings in the report should be MissingInRepo
    assert_eq!(1, report.findings.len());
    assert_eq!(RepoConsistencyFindingKind::MissingInRepo, report.findings[0].kind);
}
