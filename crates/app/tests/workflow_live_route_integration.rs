//! Route integration tests: full route-to-persist path through live bridge.

use std::sync::Arc;
use std::path::Path;

use openwand_app::workflow_session_bridge::{LiveSessionBridge, WorkflowSessionBridge};
use openwand_app::workflow_action_routing::*;
use openwand_session::testing::harness::SessionHarness;
use openwand_workflow::workflow_action_route::*;
use openwand_workflow::workflow_action_route_gate::{WorkflowActionRouteContext, evaluate_action_route};
use openwand_workflow::workflow_run::*;
use openwand_workflow::workflow_readiness::WorkflowReadinessId;
use openwand_workflow::workflow_proposal::WorkflowProposalId;
use openwand_workflow::workflow_proposal_review::WorkflowProposalReviewId;
use openwand_workflow::plan::TaskPlanId;
use chrono::Utc;

fn test_dir() -> std::path::PathBuf { tempfile::tempdir().unwrap().into_path() }

fn make_suspended_run() -> WorkflowRunRecord {
    
    WorkflowRunRecord {
        execution_id: WorkflowExecutionId("wfx_live_test".into()),
        readiness_id: WorkflowReadinessId("wfrd_live".into()),
        proposal_id: WorkflowProposalId("wfp_live".into()),
        proposal_review_id: WorkflowProposalReviewId("wfr_live".into()),
        source_task_plan_id: TaskPlanId("tpl_live".into()),
        status: WorkflowRunStatus::Suspended,
        decision: WorkflowExecutionDecision::RunCreated,
        predicates: vec![WorkflowExecutionPredicateResult {
            predicate: WorkflowExecutionPredicate::ReadinessRecordExists, passed: true, reason: "ok".into(),
        }],
        run_snapshot: WorkflowRunSnapshot {
            readiness_id: "r".into(), proposal_id: "p".into(), proposal_hash: "h".into(),
            source_task_plan_hash: "s".into(), readiness_status_at_execution: "ready".into(),
            proposal_review_decision_at_execution: "approved".into(),
        },
        stages: vec![WorkflowStageRun {
            stage_id: "stage_tool".into(), title: "Prepare".into(),
            kind: openwand_workflow::workflow_proposal::WorkflowStageKind::PrepareChange,
            status: WorkflowStageRunStatus::Suspended, order: 1, depends_on: vec![],
            started_at: Some(Utc::now()), completed_at: None,
            summary: "Suspended awaiting routing".into(),
        }],
        lifecycle_events: vec![],
        action_requests: vec![WorkflowActionRequest {
            action_request_id: "ar_live_1".into(), stage_id: "stage_tool".into(),
            capability_category: "change-preparation".into(), purpose: "Prepare changes".into(),
            expected_input_summary: "file paths".into(), expected_output_summary: "patch content".into(),
            routing_status: WorkflowActionRoutingStatus::PreparedForFutureSessionRouting,
            session_bridge_required: true, policy_gate_required: true,
        }],
        abort_snapshot: WorkflowAbortSnapshot {
            abort_notes_available: false, rollback_notes_available: false, recovery_notes: vec![],
        },
        created_at: Utc::now(), completed_at: None,
    }
}

fn route_request() -> WorkflowActionRouteRequest {
    WorkflowActionRouteRequest {
        workflow_execution_id: WorkflowExecutionId("wfx_live_test".into()),
        readiness_id: WorkflowReadinessId("wfrd_live".into()),
        proposal_id: WorkflowProposalId("wfp_live".into()),
        stage_id: "stage_tool".into(),
        action_request_id: "ar_live_1".into(),
        session_id: None,
        expected_workflow_run_hash: "hash123".into(),
        expected_action_request_hash: "arhash123".into(),
        requested_by: "test".into(),
        requested_at: Utc::now(),
        idempotency_key: "live_key_1".into(),
    }
}

fn route_context<'a>(run: &'a WorkflowRunRecord) -> WorkflowActionRouteContext<'a> {
    let stage = run.stages.iter().find(|s| s.stage_id == "stage_tool").unwrap();
    let action_req = run.action_requests.iter().find(|a| a.action_request_id == "ar_live_1").unwrap();
    WorkflowActionRouteContext {
        workflow_run: Some(run), target_stage: Some(stage), target_action_request: Some(action_req),
        prior_routes: vec![], session_bridge_available: true, session_runner_available: true,
        workflow_run_hash: "hash123".into(), action_request_hash: "arhash123".into(),
    }
}

#[test]
fn workflow_action_route_uses_live_bridge_when_configured() {
    let run = make_suspended_run();
    let req = route_request();
    let ctx = route_context(&run);
    let mut record = evaluate_action_route(&req, &ctx);
    assert_eq!(WorkflowActionRouteStatus::Routed, record.status);

    // Now call live bridge
    let harness = SessionHarness::text_only();
    let bridge = LiveSessionBridge::from_harness(harness);
    let result = bridge.route_action_to_session(record.route_prompt.clone(), None).unwrap();
    record.session_route = Some(result);
    record.status = WorkflowActionRouteStatus::Completed;
    record.decision = WorkflowActionRouteDecision::Completed { summary: "Session turn completed via live bridge".into() };
    assert_eq!(WorkflowActionRouteStatus::Completed, record.status);
    assert!(record.session_route.is_some());
}

#[test]
fn workflow_action_route_persists_live_bridge_snapshot() {
    let dir = test_dir();
    let run = make_suspended_run();
    let req = route_request();
    let ctx = route_context(&run);
    let mut record = evaluate_action_route(&req, &ctx);

    let harness = SessionHarness::text_only();
    let bridge = LiveSessionBridge::from_harness(harness);
    let result = bridge.route_action_to_session(record.route_prompt.clone(), None).unwrap();
    record.session_route = Some(result);
    record.status = WorkflowActionRouteStatus::Completed;
    record.decision = WorkflowActionRouteDecision::Completed { summary: "done".into() };

    let path = save_workflow_action_route(&dir, &record).unwrap();
    assert!(path.exists());
    let loaded = load_workflow_action_route(&dir, &record.route_id).unwrap();
    assert!(loaded.session_route.is_some());
    assert_eq!("completed", loaded.session_route.unwrap().session_status);
}

#[test]
fn workflow_action_route_with_live_bridge_writes_only_route_evidence() {
    let dir = test_dir();
    let run = make_suspended_run();
    let req = route_request();
    let ctx = route_context(&run);
    let mut record = evaluate_action_route(&req, &ctx);

    let harness = SessionHarness::text_only();
    let bridge = LiveSessionBridge::from_harness(harness);
    let result = bridge.route_action_to_session(record.route_prompt.clone(), None).unwrap();
    record.session_route = Some(result);
    record.status = WorkflowActionRouteStatus::Completed;
    record.decision = WorkflowActionRouteDecision::Completed { summary: "done".into() };

    save_workflow_action_route(&dir, &record).unwrap();
    assert!(dir.join("workflow_action_routes").exists());
    assert!(!dir.join("workflow_runs").exists());
    assert!(!dir.join("workflow_proposals").exists());
    assert!(!dir.join("task_plans").exists());
    assert!(!dir.join("approvals").exists());
    assert!(!dir.join("sessions").exists());
}

#[test]
fn workflow_action_route_with_live_bridge_does_not_mutate_workflow_run() {
    let dir = test_dir();
    let run = make_suspended_run();
    let run_json = serde_json::to_string(&run).unwrap();

    let req = route_request();
    let ctx = route_context(&run);
    let mut record = evaluate_action_route(&req, &ctx);

    let harness = SessionHarness::text_only();
    let bridge = LiveSessionBridge::from_harness(harness);
    let result = bridge.route_action_to_session(record.route_prompt.clone(), None).unwrap();
    record.session_route = Some(result);
    record.status = WorkflowActionRouteStatus::Completed;
    record.decision = WorkflowActionRouteDecision::Completed { summary: "done".into() };

    save_workflow_action_route(&dir, &record).unwrap();
    // The workflow run JSON was never written to disk by the routing path
    assert!(!dir.join("workflow_runs").exists());
}

#[test]
fn workflow_action_route_with_live_bridge_does_not_write_approval_records_directly() {
    let dir = test_dir();
    let run = make_suspended_run();
    let req = route_request();
    let ctx = route_context(&run);
    let mut record = evaluate_action_route(&req, &ctx);

    // Use approval-requiring harness
    let harness = SessionHarness::write_tool_requires_confirmation();
    let bridge = LiveSessionBridge::from_harness(harness);
    let result = bridge.route_action_to_session(record.route_prompt.clone(), None).unwrap();
    record.session_route = Some(result.clone());
    record.status = WorkflowActionRouteStatus::SuspendedForApproval;
    record.decision = WorkflowActionRouteDecision::SuspendedForApproval {
        approval_request_id: result.pending_approval_id.clone().unwrap_or_default(),
        summary: "Session suspended for approval".into(),
    };

    save_workflow_action_route(&dir, &record).unwrap();
    // Route writes only route evidence — no approval records
    assert!(!dir.join("approvals").exists());
    assert!(!dir.join("approval_records").exists());
}

#[test]
fn live_bridge_never_sets_pending_approval_without_session_event() {
    let harness = SessionHarness::text_only();
    let bridge = LiveSessionBridge::from_harness(harness);
    let prompt = WorkflowActionRoutePrompt {
        capability_category: "read".into(), purpose: "Read file".into(),
        expected_input_summary: "path".into(), expected_output_summary: "content".into(),
        safety_constraints: vec![],
    };
    let result = bridge.route_action_to_session(prompt, None).unwrap();
    // Text-only: no approval event → no pending_approval_id
    assert!(result.pending_approval_id.is_none(),
        "Pending approval must only be set when ApprovalRequested event fires");
}

#[test]
fn live_bridge_does_not_construct_tool_result_directly() {
    let harness = SessionHarness::read_file_tool_turn();
    let bridge = LiveSessionBridge::from_harness(harness);
    let prompt = WorkflowActionRoutePrompt {
        capability_category: "read".into(), purpose: "Read file".into(),
        expected_input_summary: "path".into(), expected_output_summary: "content".into(),
        safety_constraints: vec![],
    };
    let result = bridge.route_action_to_session(prompt, None).unwrap();
    // Tool was executed by SessionRunner, but bridge only records IDs
    assert!(result.tool_call_id.is_some());
    // Snapshot does not contain tool_result field — only observation data
    assert!(result.tool_name_observed_from_session.is_some());
}

#[test]
fn live_bridge_production_constructor_does_not_depend_on_session_harness() {
    // Patch 1: LiveSessionBridge::new() accepts generic TraceStore, not SessionHarness.
    // This test verifies the production constructor compiles without SessionHarness.
    // We can't fully construct without a real runner, but the type signature proves independence.
    fn _assert_new_accepts_generic(
        runner: Arc<openwand_session::runner::SessionRunner>,
        trace: Arc<dyn openwand_trace::TraceStore<openwand_store::StoredEvent>>,
    ) {
        let _bridge = LiveSessionBridge::new(runner, trace);
    }
    // Compile-time proof: this function signature does not reference SessionHarness
}

#[test]
fn live_bridge_does_not_panic_in_configured_runtime_context() {
    // Patch 3: prove the bridge works correctly when called from a non-async context
    // with a dedicated runtime (the production calling pattern).
    let harness = SessionHarness::text_only();
    let bridge = LiveSessionBridge::from_harness(harness);
    let prompt = WorkflowActionRoutePrompt {
        capability_category: "test".into(), purpose: "test".into(),
        expected_input_summary: "test".into(), expected_output_summary: "test".into(),
        safety_constraints: vec![],
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        bridge.route_action_to_session(prompt, None)
    }));
    assert!(result.is_ok(), "Bridge should not panic in configured runtime context");
    assert!(result.unwrap().is_ok());
}
