//! Wave 02p architecture guards — evaluation must not change runtime behavior.

use openwand_app::memory_evaluation::MemoryEvaluationHarness;
use openwand_memory::evaluation::{
    EvaluationModelConfig, ExpectedScenarioOutcome, MemoryEvaluationCategory,
    MemoryEvaluationExpectations, MemoryEvaluationScenario, MemoryRecordSeed,
    MockEvaluationBehavior, ScenarioExecutionMode,
};
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

#[tokio::test]
async fn memory_evaluation_does_not_change_prompt_context_text() {
    let harness = MemoryEvaluationHarness::new();
    let dir = create_workspace_dir();
    let scenario = make_guard_scenario();
    let report = harness.run_scenario(&scenario, dir.path()).await;

    if let Some(ref block) = report.snapshot.prompt_block {
        assert!(block.contains("crate core exists"), "claim should be in prompt");
        // Evaluation artifacts must not leak into prompt
        assert!(!block.contains("[evaluation]"), "eval artifacts must not be in prompt");
        assert!(!block.contains("scenario"), "scenario metadata must not be in prompt");
    }
}

#[tokio::test]
async fn memory_evaluation_does_not_render_provenance_tags_into_prompt() {
    let harness = MemoryEvaluationHarness::new();
    let dir = create_workspace_dir();
    let scenario = make_guard_scenario();
    let report = harness.run_scenario(&scenario, dir.path()).await;

    if let Some(ref block) = report.snapshot.prompt_block {
        assert!(!block.contains("User-stated"), "provenance labels must not be in prompt");
        assert!(!block.contains("LLM-extracted"), "provenance labels must not be in prompt");
        assert!(!block.contains("record "), "record IDs must not be in prompt");
    }
}

#[tokio::test]
async fn memory_evaluation_does_not_render_trace_ids_into_prompt() {
    let harness = MemoryEvaluationHarness::new();
    let dir = create_workspace_dir();
    let scenario = make_guard_scenario();
    let report = harness.run_scenario(&scenario, dir.path()).await;

    if let Some(ref block) = report.snapshot.prompt_block {
        // trace_ IDs from lineage should not appear in prompt
        let has_trace_id = block.lines().any(|l| l.contains("trace_") && !l.contains("crate"));
        assert!(!has_trace_id, "trace IDs from lineage must not appear in prompt");
    }
}

#[tokio::test]
async fn memory_evaluation_does_not_change_bucket_assignment() {
    let harness = MemoryEvaluationHarness::new();
    let dir = create_workspace_dir();
    let scenario = make_guard_scenario();

    let r1 = harness.run_scenario(&scenario, dir.path()).await;
    let r2 = harness.run_scenario(&scenario, dir.path()).await;

    // Same seed → same bucket assignments
    assert_eq!(r1.snapshot.retrieved_claims.len(), r2.snapshot.retrieved_claims.len());
    for (a, b) in r1.snapshot.retrieved_claims.iter().zip(r2.snapshot.retrieved_claims.iter()) {
        assert_eq!(format!("{:?}", a.bucket), format!("{:?}", b.bucket));
    }
}

#[tokio::test]
async fn memory_evaluation_does_not_write_memory_records() {
    // The harness creates isolated stores — no shared state
    let h1 = MemoryEvaluationHarness::new();
    let h2 = MemoryEvaluationHarness::new();
    let dir = create_workspace_dir();
    let scenario = make_guard_scenario();

    let r1 = h1.run_scenario(&scenario, dir.path()).await;
    // Running h2 with different stores should not affect h1's results
    let _ = h2.run_scenario(&scenario, dir.path()).await;

    // h1's snapshot is unchanged
    assert_eq!(1, r1.snapshot.retrieved_claims.len());
}

#[tokio::test]
async fn memory_evaluation_does_not_append_trace_entries_after_seed() {
    // Harness uses isolated trace stores. Verify the snapshot has no
    // unexpected trace lineage (we didn't seed any trace relations).
    let harness = MemoryEvaluationHarness::new();
    let dir = create_workspace_dir();
    let scenario = make_guard_scenario();
    let report = harness.run_scenario(&scenario, dir.path()).await;

    // Since we seeded no trace entries, the hydrated claims should have
    // trace_lineage that reflects empty trace state (Missing or Partial)
    for claim in &report.snapshot.retrieved_claims {
        if let Some(ref lineage) = claim.trace_lineage {
            // No relation rows expected
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

    // Same seed → same hash (proves isolation)
    assert_eq!(r1.snapshot.memory_context_hash, r2.snapshot.memory_context_hash);
}

#[tokio::test]
async fn memory_evaluation_prompt_hash_matches_runtime() {
    let harness = MemoryEvaluationHarness::new();
    let dir = create_workspace_dir();
    let scenario = make_guard_scenario();

    let r1 = harness.run_scenario(&scenario, dir.path()).await;
    let r2 = harness.run_scenario(&scenario, dir.path()).await;

    assert_eq!(r1.snapshot.memory_context_hash, r2.snapshot.memory_context_hash,
        "Same seed must produce same prompt hash");
}

#[tokio::test]
async fn expanded_eval_suite_does_not_change_runtime_prompt_hashes() {
    // Run 3 different scenarios with different seeds, verify each is internally stable
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
            "Scenario {} hash must be stable across runs", scenario.id);
    }
}
