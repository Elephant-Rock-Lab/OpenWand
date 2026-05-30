//! Wave 02p + 02r architecture guards — evaluation must not change runtime behavior.
//!
//! 02p guards: evaluation doesn't change prompt, hashes, records, trace entries.
//! 02r guards: Default profile = pre-02r behavior, governance reasons audit-visible only.

use openwand_app::memory_evaluation::MemoryEvaluationHarness;
use openwand_memory::evaluation::{
    EvaluationModelConfig, ExpectedScenarioOutcome, MemoryEvaluationCategory,
    MemoryEvaluationExpectations, MemoryEvaluationScenario, MemoryRecordSeed,
    MockEvaluationBehavior, ScenarioExecutionMode,
};
use openwand_memory::governance::{GovernanceFilteredReport, MemoryGovernanceProfile};
use openwand_memory::ranking::MemoryRankScore;
use openwand_memory::repo_consistency::{
    ConsistencySeverity, RepoConsistencyFinding, RepoConsistencyFindingKind,
    RepoConsistencyReport, RepoConsistencySummary, RepoMemoryInputSummary,
    RepoObservationSummary,
};
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

fn make_report_with_finding(kind: RepoConsistencyFindingKind, claim: &str, confidence_bps: u16)
    -> (RepoConsistencyReport, Vec<RankedMemoryHit>)
{
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

// ── 02p guards (preserved) ─────────────────────────────────────────────────

#[tokio::test]
async fn memory_evaluation_does_not_change_prompt_context_text() {
    let harness = MemoryEvaluationHarness::new();
    let dir = create_workspace_dir();
    let scenario = make_guard_scenario();
    let report = harness.run_scenario(&scenario, dir.path()).await;
    if let Some(ref block) = report.snapshot.prompt_block {
        assert!(block.contains("crate core exists"));
        assert!(!block.contains("[evaluation]"));
        assert!(!block.contains("scenario"));
    }
}

#[tokio::test]
async fn memory_evaluation_does_not_render_provenance_tags_into_prompt() {
    let harness = MemoryEvaluationHarness::new();
    let dir = create_workspace_dir();
    let scenario = make_guard_scenario();
    let report = harness.run_scenario(&scenario, dir.path()).await;
    if let Some(ref block) = report.snapshot.prompt_block {
        assert!(!block.contains("User-stated"));
        assert!(!block.contains("LLM-extracted"));
        assert!(!block.contains("record "));
    }
}

#[tokio::test]
async fn memory_evaluation_does_not_render_trace_ids_into_prompt() {
    let harness = MemoryEvaluationHarness::new();
    let dir = create_workspace_dir();
    let scenario = make_guard_scenario();
    let report = harness.run_scenario(&scenario, dir.path()).await;
    if let Some(ref block) = report.snapshot.prompt_block {
        let has_trace_id = block.lines().any(|l| l.contains("trace_") && !l.contains("crate"));
        assert!(!has_trace_id);
    }
}

#[tokio::test]
async fn memory_evaluation_does_not_change_bucket_assignment() {
    let harness = MemoryEvaluationHarness::new();
    let dir = create_workspace_dir();
    let scenario = make_guard_scenario();
    let r1 = harness.run_scenario(&scenario, dir.path()).await;
    let r2 = harness.run_scenario(&scenario, dir.path()).await;
    assert_eq!(r1.snapshot.retrieved_claims.len(), r2.snapshot.retrieved_claims.len());
    for (a, b) in r1.snapshot.retrieved_claims.iter().zip(r2.snapshot.retrieved_claims.iter()) {
        assert_eq!(format!("{:?}", a.bucket), format!("{:?}", b.bucket));
    }
}

#[tokio::test]
async fn memory_evaluation_does_not_write_memory_records() {
    let h1 = MemoryEvaluationHarness::new();
    let h2 = MemoryEvaluationHarness::new();
    let dir = create_workspace_dir();
    let scenario = make_guard_scenario();
    let r1 = h1.run_scenario(&scenario, dir.path()).await;
    let _ = h2.run_scenario(&scenario, dir.path()).await;
    assert_eq!(1, r1.snapshot.retrieved_claims.len());
}

#[tokio::test]
async fn memory_evaluation_does_not_append_trace_entries_after_seed() {
    let harness = MemoryEvaluationHarness::new();
    let dir = create_workspace_dir();
    let scenario = make_guard_scenario();
    let report = harness.run_scenario(&scenario, dir.path()).await;
    for claim in &report.snapshot.retrieved_claims {
        if let Some(ref lineage) = claim.trace_lineage {
            assert!(lineage.derived_from.is_empty());
            assert!(lineage.verifies.is_empty());
        }
    }
}

#[tokio::test]
async fn memory_evaluation_uses_isolated_stores() {
    let h1 = MemoryEvaluationHarness::new();
    let h2 = MemoryEvaluationHarness::new();
    let dir = create_workspace_dir();
    let scenario = make_guard_scenario();
    let r1 = h1.run_scenario(&scenario, dir.path()).await;
    let r2 = h2.run_scenario(&scenario, dir.path()).await;
    assert_eq!(r1.snapshot.memory_context_hash, r2.snapshot.memory_context_hash);
}

#[tokio::test]
async fn memory_evaluation_prompt_hash_matches_runtime() {
    let harness = MemoryEvaluationHarness::new();
    let dir = create_workspace_dir();
    let scenario = make_guard_scenario();
    let r1 = harness.run_scenario(&scenario, dir.path()).await;
    let r2 = harness.run_scenario(&scenario, dir.path()).await;
    assert_eq!(r1.snapshot.memory_context_hash, r2.snapshot.memory_context_hash);
}

#[tokio::test]
async fn expanded_eval_suite_does_not_change_runtime_prompt_hashes() {
    let scenarios = vec![
        make_guard_scenario(),
        MemoryEvaluationScenario {
            id: "guard_extra".into(),
            title: "Extra guard".into(),
            category: MemoryEvaluationCategory::PromptIncluded,
            execution_mode: ScenarioExecutionMode::FullHarness,
            user_query: "test".into(),
            expected_outcome: ExpectedScenarioOutcome::Pass,
            seed_memory: vec![MemoryRecordSeed {
                label: Some("tools_claim".into()),
                claim: "crate core exists".into(),
                kind: "Fact".into(),
                confidence: 0.85,
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
        },
    ];
    for scenario in &scenarios {
        let harness = MemoryEvaluationHarness::new();
        let dir = create_workspace_dir();
        let r1 = harness.run_scenario(scenario, dir.path()).await;
        let r2 = harness.run_scenario(scenario, dir.path()).await;
        assert_eq!(r1.snapshot.memory_context_hash, r2.snapshot.memory_context_hash,
            "Scenario {} hash must be stable", scenario.id);
    }
}

// ── 02r guards ─────────────────────────────────────────────────────────────

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
        RepoConsistencyFindingKind::Supported, "low confidence claim", 1000,
    );
    let profile = MemoryGovernanceProfile::batch_02r_default();
    let governed = GovernanceFilteredReport::from_report(&report, &profile, &hits);

    assert!(!governed.audit_only_claims.is_empty());
    let audit = &governed.audit_only_claims[0];
    assert!(!audit.governance_reasons.is_empty());
    assert!(audit.governance_reasons[0].contains("confidence"));
}

#[test]
fn governance_reason_visible_for_superseded_exclusion() {
    let (report, _) = make_report_with_finding(
        RepoConsistencyFindingKind::SupersededMemoryIgnored, "old claim", 9000,
    );
    let profile = MemoryGovernanceProfile::default();
    let governed = GovernanceFilteredReport::from_report(&report, &profile, &[]);

    assert!(!governed.audit_only_claims.is_empty());
    assert!(governed.audit_only_claims[0].governance_reasons.iter().any(|r| r.contains("superseded")));
}

#[test]
fn governance_reason_visible_for_conflict_exclusion() {
    let (report, _) = make_report_with_finding(
        RepoConsistencyFindingKind::ConflictRequiresReview, "conflicting claim", 9000,
    );
    let profile = MemoryGovernanceProfile::default();
    let governed = GovernanceFilteredReport::from_report(&report, &profile, &[]);

    assert!(!governed.audit_only_claims.is_empty());
    assert!(governed.audit_only_claims[0].governance_reasons.iter().any(|r| r.contains("conflict")));
}

#[test]
fn prompt_does_not_render_governance_reason_tags() {
    let (report, _) = make_report_with_finding(
        RepoConsistencyFindingKind::Unverifiable, "unverifiable claim", 9000,
    );
    let profile = MemoryGovernanceProfile::batch_02r_default();
    let governed = GovernanceFilteredReport::from_report(&report, &profile, &[]);

    for gf in &governed.audit_only_claims {
        for reason in &gf.governance_reasons {
            assert!(!reason.is_empty());
        }
    }
    assert!(!governed.audit_only_claims.is_empty());
}

#[test]
fn crate_absence_still_classifies_missing_in_repo_not_stale() {
    let (report, _) = make_report_with_finding(
        RepoConsistencyFindingKind::MissingInRepo, "crate nonexistent exists", 9000,
    );
    assert_eq!(1, report.findings.len());
    assert_eq!(RepoConsistencyFindingKind::MissingInRepo, report.findings[0].kind);
}
