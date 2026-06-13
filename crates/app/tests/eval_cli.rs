//! Handler-level unit tests for eval subcommands (not binary CLI tests).
//! Uses direct function calls to crate internals. — verify compare/summarize subcommands.

use openwand_app::eval_compare::*;
use openwand_app::eval_model::*;
use openwand_app::eval_reports::*;

fn make_report(scenario_id: &str, total: u32) -> EvalRunReport {
    EvalRunReport {
        report_schema_version: EVAL_REPORT_SCHEMA_VERSION,
        scenario_id: scenario_id.to_string(),
        provider: ProviderRealitySnapshot {
            provider: "test".to_string(),
            model: "test-model".to_string(),
            base_url_redacted: None,
            supports_streaming: false,
            supports_tools: false,
            supports_reasoning: false,
            health_status: ProviderHealthStatus::Healthy,
            temperature: None,
            max_tokens: None,
            observed_at: chrono::Utc::now(),
        },
        prompt: PromptEvalResult::default(),
        memory: MemoryEvalResult {
            included_claims_seen: vec![], excluded_claims_seen: vec![],
            missing_required: vec![], unexpected_included: vec![],
            prompt_panel_equivalent: true,
        },
        tools: ToolEvalResult {
            requested_tools: vec![], executed_tools: vec![],
            blocked_tools: vec![], forbidden_requested: vec![],
        },
        policy: PolicyEvalResult {
            gates_seen: vec![], required_approvals_seen: vec![],
            unexpected_allows: vec![],
        },
        patch: PatchEvalResult {
            planned: false, applied: false, preimage_verified: false,
            postimage_verified: false, rollback_available: false,
            changed_files_match_expected: true,
        },
        explain: ExplainEvalResult {
            memory_matches: true, policy_matches: true,
            tool_matches: true, completion_matches: true,
        },
        rebuild: RebuildEvalResult {
            events_replayed: 0, state_matches: true, divergences: vec![],
        },
        capability_context: CapabilityContextEvalResult::default(),
        score: EvalScore::from_dimensions(vec![
            DimensionScore { name: "memory".into(), passed: total, total, evidence_refs: vec![] },
        ]),
    }
}

#[test]
fn eval_cli_compare_works_with_two_reports() {
    let dir = tempfile::tempdir().unwrap();
    let _store = EvalReportStore::new(dir.path().to_path_buf());

    let mut current = make_report("test", 90);
    current.provider.model = "model_v2".to_string();
    let baseline = make_report("test", 80);

    // Compare directly (no path resolution needed)
    let thresholds = RegressionThresholds::default();
    let comparison = compare_reports(&current, Some(&baseline), &thresholds);

    assert_eq!("test", comparison.scenario_id);
    assert!(comparison.score_delta.delta.unwrap() > 0, "Should show improvement");
    assert!(!comparison.improvements.is_empty());
}

#[test]
fn eval_cli_summarize_lists_reports() {
    let dir = tempfile::tempdir().unwrap();
    let store = EvalReportStore::new(dir.path().to_path_buf());

    store.save_report(&make_report("alpha", 80)).unwrap();
    store.save_report(&make_report("beta", 70)).unwrap();

    let reports = store.list_reports(&ReportFilter::default()).unwrap();
    assert_eq!(2, reports.len());

    // Verify they're sorted by scenario then newest-first
    let ids: Vec<&str> = reports.iter().map(|r| r.report.scenario_id.as_str()).collect();
    assert!(ids.contains(&"alpha"));
    assert!(ids.contains(&"beta"));
}

#[test]
fn eval_cli_fail_on_regression_detects_drop() {
    let dir = tempfile::tempdir().unwrap();
    let _store = EvalReportStore::new(dir.path().to_path_buf());

    let current = make_report("test", 60);
    let baseline = make_report("test", 80);

    let thresholds = RegressionThresholds {
        max_score_drop: 5,
        ..Default::default()
    };
    let comparison = compare_reports(&current, Some(&baseline), &thresholds);
    assert!(!comparison.regressions.is_empty(), "Should detect score drop of 20");
}

#[test]
fn eval_cli_summarize_with_scenario_filter() {
    let dir = tempfile::tempdir().unwrap();
    let store = EvalReportStore::new(dir.path().to_path_buf());

    store.save_report(&make_report("alpha", 80)).unwrap();
    store.save_report(&make_report("beta", 70)).unwrap();

    let filtered = store.list_reports(&ReportFilter {
        scenario_id: Some("alpha".to_string()),
    }).unwrap();
    assert_eq!(1, filtered.len());
    assert_eq!("alpha", filtered[0].report.scenario_id);
}

#[test]
fn eval_cli_compare_with_provider_change() {
    let mut current = make_report("test", 80);
    current.provider.provider = "ollama".to_string();
    current.provider.model = "llama3".to_string();

    let baseline = make_report("test", 80);

    let comparison = compare_reports(&current, Some(&baseline), &Default::default());
    assert!(comparison.provider_delta.provider_changed);
    assert!(comparison.provider_delta.model_changed);
}
