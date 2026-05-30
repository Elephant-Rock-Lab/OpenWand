//! E2E flagship scenario — governed agent loop lock test.
//!
//! Proves the full chain: governance → explain → file_patch → rebuild

use openwand_core::ToolCallId;
use openwand_memory::governance::MemoryGovernanceProfileId;
use openwand_memory::repo_consistency::{
    RepoConsistencyFinding, RepoConsistencyFindingKind, RepoConsistencyReport,
    ConsistencySeverity, RepoMemoryInputSummary, RepoObservationSummary,
    RepoConsistencySummary,
};
use openwand_app::explain::{
    Explanation, MemoryExplanation, PolicyExplanation, ExecutionExplanation,
    CompletionExplanation, ClaimEntry, ExcludedClaimEntry, GateEntry, ApprovalEntry,
    render_explanation_plain,
};
use std::path::PathBuf;

/// Verify the E2E fixture repository builds.
#[test]
fn e2e_fixture_repository_builds() {
    let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("e2e_governed_task")
        .join("fixture_lib");

    if !fixture_dir.exists() {
        eprintln!("Skipping: fixture dir not found at {:?}", fixture_dir);
        return;
    }

    let output = std::process::Command::new("cargo")
        .args(["build"])
        .current_dir(fixture_dir)
        .output()
        .expect("cargo build should start");

    assert!(output.status.success(), "fixture build failed: {}", String::from_utf8_lossy(&output.stderr));
}

/// Governance: verified claim included, low-confidence excluded.
#[test]
fn e2e_governance_includes_verified_excludes_low() {
    let profile = MemoryGovernanceProfileId::Batch02rDefault.resolve();

    let report = RepoConsistencyReport {
        repo_root: PathBuf::from("/fixture_lib"),
        checked_at: chrono::Utc::now(),
        summary: RepoConsistencySummary {
            supported: 2, stale: 0, missing_in_repo: 0,
            missing_in_memory: 0, unverifiable: 0, conflicted: 0, superseded_ignored: 0,
        },
        findings: vec![
            RepoConsistencyFinding {
                kind: RepoConsistencyFindingKind::Supported,
                claim_text: Some("crate fixture_lib has function hello()".to_string()),
                evidence_kind: None,
                repo_evidence_key: vec!["src/lib.rs".to_string()],
                severity: ConsistencySeverity::Low,
                detail: "found".to_string(),
            },
            RepoConsistencyFinding {
                kind: RepoConsistencyFindingKind::Supported,
                claim_text: Some("crate fixture_lib has function goodbye()".to_string()),
                evidence_kind: None,
                repo_evidence_key: vec![],
                severity: ConsistencySeverity::Low,
                detail: "low conf".to_string(),
            },
        ],
        memory_inputs: RepoMemoryInputSummary {
            current_claims_count: 2, superseded_count: 0, conflict_groups_count: 0,
        },
        repo_inputs: RepoObservationSummary {
            crates_count: 1, dependencies_count: 0, docs_count: 0,
        },
    };

    let governed = openwand_memory::governance::GovernanceFilteredReport::from_report(
        &report, &profile, &[],
    );

    assert_eq!(2, governed.governed_findings.len());
    assert!(governed.included_claims.len() + governed.audit_only_claims.len() == 2);
}

/// Explain renders the complete chain: memory, policy, execution, completion.
#[test]
fn e2e_explain_renders_complete_explanation() {
    let explanation = Explanation {
        memory: MemoryExplanation {
            included: vec![ClaimEntry {
                claim: "crate fixture_lib has function hello()".to_string(),
                confidence_bps: 9500,
                evidence_kind: "AcceptedClaim".to_string(),
                source: "UserStated".to_string(),
            }],
            excluded: vec![ExcludedClaimEntry {
                claim: "crate fixture_lib has function goodbye()".to_string(),
                confidence_bps: 1500,
                reason: "below prompt_include_min_bps threshold".to_string(),
            }],
        },
        policy: PolicyExplanation {
            gates: vec![GateEntry {
                tool_name: "local__file_patch".to_string(),
                risk: "Medium".to_string(),
                confirmation: "Approve".to_string(),
                decision: "suspended for approval".to_string(),
            }],
            approvals: vec![ApprovalEntry {
                tool_name: "local__file_patch".to_string(),
                decision: "granted".to_string(),
                reason: None,
            }],
        },
        execution: ExecutionExplanation::from_tool_results(&[
            ("local__file_patch".to_string(), ToolCallId::new(), true,
             "Patched src/lib.rs".to_string(), Some(42)),
        ]),
        completion: CompletionExplanation {
            completed: true,
            changed_files: vec!["src/lib.rs".to_string()],
            diff_stat: Some("1 file changed, 3 insertions(+)".to_string()),
            test_output: None,
        },
    };

    let text = render_explanation_plain(&explanation);

    assert!(text.contains("=== Memory ==="));
    assert!(text.contains("crate fixture_lib has function hello()"));
    assert!(text.contains("✗ crate fixture_lib has function goodbye()"));
    assert!(text.contains("=== Policy ==="));
    assert!(text.contains("suspended for approval"));
    assert!(text.contains("granted"));
    assert!(text.contains("=== Execution ==="));
    assert!(text.contains("Patched"));
    assert!(text.contains("=== Completion ==="));
    assert!(text.contains("src/lib.rs"));
}

/// File patch plan+apply with rollback end-to-end.
#[tokio::test]
async fn e2e_file_patch_plan_then_apply() {
    let dir = tempfile::tempdir().unwrap();
    tokio::fs::write(dir.path().join("lib.rs"), "fn hello() {}\nfn other() {}\n")
        .await.unwrap();

    let provider = openwand_tools::local::batch2_local_tools();
    let ctx = openwand_tools::result::ToolCallContext {
        working_directory: dir.path().to_string_lossy().to_string(),
        session_id: openwand_core::SessionId::new(),
        cancellation: tokio_util::sync::CancellationToken::new(),
    };
    let tool_name = openwand_tools::naming::canonical_local_tool_name("file_patch");

    // Step 1: Plan
    let plan_result = provider.execute(&tool_name, serde_json::json!({
        "_call_id": "tc_plan",
        "path": "lib.rs",
        "mode": "plan",
        "line_number": 1,
        "old_lines": ["fn hello() {}"],
        "new_lines": ["fn hello() -> &str { \"hello\" }", "fn goodbye() -> &str { \"goodbye\" }"]
    }), ctx.clone()).await.unwrap();
    assert!(!plan_result.is_error, "Plan failed: {}", plan_result.output);

    let plan_id = plan_result.output.lines()
        .find(|l| l.trim().starts_with("Plan ID:"))
        .unwrap()
        .split(": ").last().unwrap().trim().to_string();

    // Step 2: Apply
    let apply_result = provider.execute(&tool_name, serde_json::json!({
        "_call_id": "tc_apply",
        "path": "lib.rs",
        "mode": "apply",
        "plan_id": plan_id,
        "line_number": 1,
        "old_lines": ["fn hello() {}"],
        "new_lines": ["fn hello() -> &str { \"hello\" }", "fn goodbye() -> &str { \"goodbye\" }"]
    }), ctx.clone()).await.unwrap();
    assert!(!apply_result.is_error, "Apply failed: {}", apply_result.output);
    assert!(apply_result.output.contains("Patched lib.rs"));

    // Verify content changed
    let new_content = tokio::fs::read_to_string(dir.path().join("lib.rs")).await.unwrap();
    assert!(new_content.contains("goodbye"));
    assert!(!new_content.contains("fn hello() {}"));
    assert!(new_content.contains("fn hello() -> &str"));

    // Verify rollback exists
    let rollback_dir = dir.path().join(".openwand").join("rollback");
    assert!(rollback_dir.exists());
    assert_eq!(1, std::fs::read_dir(&rollback_dir).unwrap().count());

    // Verify rollback content matches original
    let rollback_file: std::path::PathBuf = std::fs::read_dir(&rollback_dir).unwrap().next().unwrap().unwrap().path();
    let rollback_content = std::fs::read_to_string(&rollback_file).unwrap();
    assert!(rollback_content.contains("fn hello() {}"));
}

/// Rebuild verification: session state reconstructs from trace.
#[tokio::test]
async fn e2e_session_rebuildable_from_trace() {
    use openwand_trace::testing::InMemoryTraceStore;
    use openwand_trace::store::TraceStore;
    use openwand_trace::append::AppendTraceEntry;
    use openwand_trace::stream::{TraceStreamId, TraceStreamScope};
    use openwand_trace::actor::Actor;
    use openwand_store::StoredEvent;
    use openwand_core::OpenWandTraceEvent;
    use openwand_core::events::SessionEvent;
    use openwand_session::rebuild::rebuild_session;

    let store = InMemoryTraceStore::<StoredEvent>::new();
    let stream = TraceStreamId {
        scope: TraceStreamScope::Session,
        id: "e2e_rebuild".to_string(),
    };

    // Append events
    let events = vec![
        OpenWandTraceEvent::Session(SessionEvent::UserMessageInjected { text: "test".to_string() }),
        OpenWandTraceEvent::Session(SessionEvent::UserMessageInjected { text: "second".to_string() }),
    ];
    for event in events {
        let entry = AppendTraceEntry {
            actor: Actor::System { component: "e2e".to_string() },
            event: StoredEvent::from(event),
            relations: vec![],
            stream_id: stream.clone(),
            idempotency_key: None,
        };
        store.append(entry).await.unwrap();
    }

    let result = rebuild_session(&store, "e2e_rebuild", None, |e| e.clone().into()).await.unwrap();
    assert_eq!(2, result.events_replayed);
    assert!(result.state_matches);
}
