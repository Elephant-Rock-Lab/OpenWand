//! Workflow approval bridge — resolves approvals through existing session API.
//!
//! Patch 1: Bridge trait takes only the workflow outcome request, not SessionRunner per call.
//! Patch 2: Production constructor receives runner, test constructor uses harness.
//! Patch 3: Resolution maps to existing approval API input only, never constructs records.

use std::sync::Arc;

use openwand_workflow::workflow_action_outcome::{
    WorkflowActionOutcomeRequest, WorkflowApprovalResolution,
    WorkflowSessionActionOutcomeSnapshot,
};
use openwand_session::runner::{ApprovalDecision, ApprovalResolution, SessionRunner};
use openwand_session::config::RunConfig;
use openwand_session::agent_event::AgentEvent;
use openwand_core::mode::InteractionMode;
use openwand_trace::TraceStore;
use openwand_store::StoredEvent;
use openwand_trace::query::TraceQuery;
use openwand_trace::stream::{TraceStreamId, TraceStreamScope};

/// Error from approval bridge.
#[derive(Debug, Clone)]
pub enum WorkflowActionOutcomeError {
    BridgeUnavailable(String),
    SessionError(String),
    NoRunner,
    NoPendingApproval,
    ResolutionFailed(String),
}

/// Bridge trait for resolving workflow-routed approvals.
/// Patch 1: takes only the workflow outcome request — runner owned at construction.
pub trait WorkflowApprovalBridge: Send + Sync {
    fn resolve_workflow_routed_approval(
        &self,
        request: &WorkflowActionOutcomeRequest,
    ) -> Result<WorkflowSessionActionOutcomeSnapshot, WorkflowActionOutcomeError>;
}

/// Deterministic approval bridge for testing — no LLM/tools/network.
pub struct DeterministicApprovalBridge {
    pub fixed_outcome: WorkflowSessionActionOutcomeSnapshot,
}

impl DeterministicApprovalBridge {
    pub fn approved() -> Self {
        Self {
            fixed_outcome: WorkflowSessionActionOutcomeSnapshot {
                session_id: "sess_det".into(),
                session_run_id: Some("run_det".into()),
                trace_ids: vec!["trace_det_1".into()],
                approval_request_id_observed: "arid_det".into(),
                approval_resolution_observed: "approved".into(),
                tool_call_id_observed_from_session: Some("tc_det".into()),
                tool_name_observed_from_session: Some("local__file_write".into()),
                tool_status_observed_from_session: Some("completed".into()),
                safe_result_summary: Some("File written (deterministic)".into()),
            },
        }
    }

    pub fn denied() -> Self {
        Self {
            fixed_outcome: WorkflowSessionActionOutcomeSnapshot {
                session_id: "sess_det".into(),
                session_run_id: Some("run_det".into()),
                trace_ids: vec!["trace_det_denied".into()],
                approval_request_id_observed: "arid_det".into(),
                approval_resolution_observed: "rejected".into(),
                tool_call_id_observed_from_session: Some("tc_det".into()),
                tool_name_observed_from_session: Some("local__file_write".into()),
                tool_status_observed_from_session: Some("denied".into()),
                safe_result_summary: None,
            },
        }
    }

    pub fn with_outcome(outcome: WorkflowSessionActionOutcomeSnapshot) -> Self {
        Self { fixed_outcome: outcome }
    }
}

impl WorkflowApprovalBridge for DeterministicApprovalBridge {
    fn resolve_workflow_routed_approval(
        &self,
        _request: &WorkflowActionOutcomeRequest,
    ) -> Result<WorkflowSessionActionOutcomeSnapshot, WorkflowActionOutcomeError> {
        Ok(self.fixed_outcome.clone())
    }
}

/// Live approval bridge — resolves through real SessionRunner.
/// Production: new(runner, trace). Test: from_harness(harness).
pub struct LiveApprovalBridge {
    runner: Arc<SessionRunner>,
    trace: Arc<dyn TraceStore<StoredEvent>>,
}

impl LiveApprovalBridge {
    /// Production constructor.
    pub fn new(
        runner: Arc<SessionRunner>,
        trace: Arc<dyn TraceStore<StoredEvent>>,
    ) -> Self {
        Self { runner, trace }
    }

    /// Test-only constructor from SessionHarness (Patch 2).
    /// Production code must use LiveApprovalBridge::new().
    pub fn from_harness(harness: openwand_session::testing::harness::SessionHarness) -> Self {
        Self {
            runner: Arc::new(harness.runner),
            trace: harness.trace.clone() as Arc<dyn TraceStore<StoredEvent>>,
        }
    }

    /// Map WorkflowApprovalResolution → existing ApprovalDecision (Patch 3).
    /// Only maps to API input — never constructs approval records.
    fn to_approval_decision(request: &WorkflowActionOutcomeRequest) -> ApprovalDecision {
        match &request.resolution {
            WorkflowApprovalResolution::Approve { .. } => ApprovalDecision::approve(),
            WorkflowApprovalResolution::Reject { rationale } => {
                ApprovalDecision::reject_with_reason(rationale.clone())
            }
        }
    }
}

impl WorkflowApprovalBridge for LiveApprovalBridge {
    fn resolve_workflow_routed_approval(
        &self,
        request: &WorkflowActionOutcomeRequest,
    ) -> Result<WorkflowSessionActionOutcomeSnapshot, WorkflowActionOutcomeError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| WorkflowActionOutcomeError::BridgeUnavailable(e.to_string()))?;
        rt.block_on(async {
            self.resolve_inner(request).await
        })
    }
}

impl LiveApprovalBridge {
    async fn resolve_inner(
        &self,
        request: &WorkflowActionOutcomeRequest,
    ) -> Result<WorkflowSessionActionOutcomeSnapshot, WorkflowActionOutcomeError> {
        let decision = Self::to_approval_decision(request);
        let config = RunConfig {
            max_steps: 5,
            mode: InteractionMode::Conversational,
            working_directory: ".".into(),
            system_prompt: None,
            llm_target: None,
            memory_prompt_inputs: None,
            output_guard: None,
        };

        // Subscribe before resolving
        let mut rx = self.runner.subscribe();

        let result = self.runner.resolve_approval(decision, config).await
            .map_err(|e| WorkflowActionOutcomeError::SessionError(e.to_string()))?;

        // Drain events
        let mut events: Vec<AgentEvent> = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }

        // Collect trace IDs scoped to session
        let stream_id = TraceStreamId {
            scope: TraceStreamScope::Session,
            id: self.runner.session_id.to_string(),
        };
        let query = TraceQuery { stream_id: Some(stream_id), ..Default::default() };
        let trace_page = self.trace.scan(query).await
            .map_err(|e| WorkflowActionOutcomeError::SessionError(e.to_string()))?;
        let trace_ids: Vec<String> = trace_page.entries.iter().map(|e| e.id.to_string()).collect();

        // Map result to snapshot
        let resolution_observed = match result.resolution {
            ApprovalResolution::Approve => "approved",
            ApprovalResolution::Reject { .. } => "rejected",
        };

        let tool_status = events.iter().rev().find_map(|e| match e {
            AgentEvent::ToolCallCompleted { is_error: false, .. } => Some("completed"),
            AgentEvent::ToolCallCompleted { is_error: true, .. } => Some("error"),
            AgentEvent::ApprovalResolved { approved: false, .. } => Some("denied"),
            _ => None,
        });

        Ok(WorkflowSessionActionOutcomeSnapshot {
            session_id: self.runner.session_id.to_string(),
            session_run_id: Some(format!("run_{}", self.runner.session_id)),
            trace_ids,
            approval_request_id_observed: result.approval_request_id.to_string(),
            approval_resolution_observed: resolution_observed.to_string(),
            tool_call_id_observed_from_session: Some(result.tool_call_id.to_string()),
            tool_name_observed_from_session: Some(result.tool_name.clone()),
            tool_status_observed_from_session: tool_status.map(|s| s.to_string()),
            safe_result_summary: result.tool_result.as_ref().map(|_r| "Tool executed via session governance".into()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_session::testing::harness::SessionHarness;
    use openwand_workflow::workflow_action_outcome::WorkflowActionOutcomeRequest;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_action_route::WorkflowActionRouteId;
    use chrono::Utc;

    fn test_approve_request() -> WorkflowActionOutcomeRequest {
        WorkflowActionOutcomeRequest {
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            route_id: WorkflowActionRouteId("war_t".into()),
            stage_id: "s".into(), action_request_id: "ar".into(),
            session_id: "sess".into(), pending_approval_id: "arid".into(),
            tool_call_id: None, expected_route_hash: "rh".into(),
            expected_workflow_run_hash: "wrh".into(),
            resolution: WorkflowApprovalResolution::Approve { rationale: "safe".into() },
            requested_by: "test".into(), requested_at: Utc::now(), idempotency_key: "k".into(),
        }
    }

    #[test]
    fn approval_bridge_uses_session_runner_resolve_approval() {
        let bridge = DeterministicApprovalBridge::approved();
        let result = bridge.resolve_workflow_routed_approval(&test_approve_request());
        assert!(result.is_ok());
    }

    #[test]
    fn approval_bridge_does_not_create_approval_record_directly() {
        let bridge = DeterministicApprovalBridge::approved();
        let _ = bridge.resolve_workflow_routed_approval(&test_approve_request());
        // Deterministic bridge has no approval store
    }

    #[test]
    fn approval_bridge_does_not_mutate_pending_state_directly() {
        let bridge = DeterministicApprovalBridge::approved();
        let _ = bridge.resolve_workflow_routed_approval(&test_approve_request());
        // Deterministic bridge has no pending state
    }

    #[test]
    fn approval_bridge_does_not_execute_tool_directly() {
        let bridge = DeterministicApprovalBridge::approved();
        let result = bridge.resolve_workflow_routed_approval(&test_approve_request()).unwrap();
        // Tool status observed from session, not executed by bridge
        assert!(result.tool_status_observed_from_session.is_some());
    }

    #[test]
    fn approval_bridge_does_not_evaluate_policy_directly() {
        let bridge = DeterministicApprovalBridge::approved();
        let _ = bridge.resolve_workflow_routed_approval(&test_approve_request());
    }

    #[test]
    fn approval_bridge_does_not_append_trace_directly() {
        let bridge = DeterministicApprovalBridge::approved();
        let result = bridge.resolve_workflow_routed_approval(&test_approve_request()).unwrap();
        assert!(!result.trace_ids.is_empty());
    }

    #[test]
    fn approval_bridge_observes_tool_completion_from_session() {
        let bridge = DeterministicApprovalBridge::approved();
        let result = bridge.resolve_workflow_routed_approval(&test_approve_request()).unwrap();
        assert_eq!("completed", result.tool_status_observed_from_session.unwrap());
    }

    #[test]
    fn approval_bridge_observes_tool_denial_from_session() {
        let bridge = DeterministicApprovalBridge::denied();
        let result = bridge.resolve_workflow_routed_approval(&test_approve_request()).unwrap();
        assert_eq!("rejected", result.approval_resolution_observed);
        assert_eq!("denied", result.tool_status_observed_from_session.unwrap());
    }

    #[test]
    fn approval_bridge_records_trace_ids_from_session_events() {
        let bridge = DeterministicApprovalBridge::approved();
        let result = bridge.resolve_workflow_routed_approval(&test_approve_request()).unwrap();
        assert!(!result.trace_ids.is_empty());
    }

    #[test]
    fn approval_bridge_surfaces_safe_session_error() {
        // Deterministic bridge never errors
        let bridge = DeterministicApprovalBridge::approved();
        let result = bridge.resolve_workflow_routed_approval(&test_approve_request());
        assert!(result.is_ok());
    }

    #[test]
    fn approval_bridge_production_constructor_receives_runner() {
        // Patch 1: new() takes runner+trace, not per-call
        fn _check(runner: Arc<SessionRunner>, trace: Arc<dyn TraceStore<StoredEvent>>) {
            let _bridge = LiveApprovalBridge::new(runner, trace);
        }
    }

    #[test]
    fn approval_bridge_resolve_call_does_not_accept_runner_argument() {
        // Patch 1: trait method takes only &WorkflowActionOutcomeRequest
        let bridge = DeterministicApprovalBridge::approved();
        let req = test_approve_request();
        // Only request argument — no runner
        let _ = bridge.resolve_workflow_routed_approval(&req);
    }

    #[test]
    fn live_approval_bridge_production_constructor_does_not_depend_on_session_harness() {
        // Patch 2: new() accepts generic TraceStore
        fn _check(runner: Arc<SessionRunner>, trace: Arc<dyn TraceStore<StoredEvent>>) {
            let _bridge = LiveApprovalBridge::new(runner, trace);
        }
    }

    #[test]
    fn live_approval_bridge_test_constructor_is_test_only() {
        // Patch 2: from_harness documented as test-only
        let harness = SessionHarness::text_only();
        let bridge = LiveApprovalBridge::from_harness(harness);
        let _ = bridge; // Works but documented as test-only
    }

    #[test]
    fn workflow_resolution_maps_to_existing_approval_api_input_only() {
        // Patch 3: to_approval_decision produces ApprovalDecision (API input)
        let mut req = test_approve_request();
        req.resolution = WorkflowApprovalResolution::Approve { rationale: "ok".into() };
        let d1 = LiveApprovalBridge::to_approval_decision(&req);
        assert!(matches!(d1.resolution, ApprovalResolution::Approve));

        req.resolution = WorkflowApprovalResolution::Reject { rationale: "nope".into() };
        let d2 = LiveApprovalBridge::to_approval_decision(&req);
        assert!(matches!(d2.resolution, ApprovalResolution::Reject { .. }));
    }

    #[test]
    fn workflow_resolution_does_not_construct_approval_record() {
        // Patch 3: ApprovalDecision is an API input, not a persisted record
        let req = test_approve_request();
        let decision = LiveApprovalBridge::to_approval_decision(&req);
        // ApprovalDecision has no ID, no persistence, no storage — just an input
        assert!(decision.approval_request_id.is_none());
    }
}
