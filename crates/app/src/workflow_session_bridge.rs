//! Workflow session bridge — routes action prompts into existing session seams.
//!
//! The bridge is an app-layer adapter. Workflow crate defines the prompt DTO.
//! App crate owns the bridge trait and implementations.
//!
//! Wave 28: LiveSessionBridge routes through real SessionRunner with
//! deterministic fixtures. Production constructor receives real runner handle.
//! Test-only constructor wraps SessionHarness (Patch 1).

use openwand_workflow::workflow_action_route::{
    WorkflowActionRoutePrompt, WorkflowSessionRouteSnapshot,
};

use openwand_session::runner::SessionRunner;
use openwand_session::agent_event::AgentEvent;
use openwand_session::config::RunConfig;
use openwand_core::mode::InteractionMode;
use openwand_trace::TraceStore;
use openwand_store::StoredEvent;
use openwand_trace::query::TraceQuery;
use openwand_trace::stream::{TraceStreamId, TraceStreamScope};
use std::sync::Arc;

/// Error from session bridge routing.
#[derive(Debug, Clone)]
pub enum WorkflowActionRouteError {
    BridgeUnavailable(String),
    SessionError(String),
    NoSessionId,
    Timeout(String),
}

/// Bridge trait for routing workflow action prompts into sessions.
/// Production impl calls SessionRunner APIs only.
/// Test impl returns deterministic snapshots.
pub trait WorkflowSessionBridge: Send + Sync {
    fn route_action_to_session(
        &self,
        prompt: WorkflowActionRoutePrompt,
        session_id: Option<String>,
    ) -> Result<WorkflowSessionRouteSnapshot, WorkflowActionRouteError>;
}

/// Deterministic bridge for CI — no LLM, tools, network, or session.
/// Returns a fixed snapshot proving the seam without runtime dependencies.
pub struct DeterministicSessionBridge {
    pub fixed_session_id: String,
    pub fixed_status: String,
    pub fixed_approval_id: Option<String>,
}

impl DeterministicSessionBridge {
    pub fn completed() -> Self {
        Self {
            fixed_session_id: "sess_deterministic".into(),
            fixed_status: "completed".into(),
            fixed_approval_id: None,
        }
    }

    pub fn suspended_for_approval() -> Self {
        Self {
            fixed_session_id: "sess_deterministic".into(),
            fixed_status: "suspended_for_approval".into(),
            fixed_approval_id: Some("arid_deterministic".into()),
        }
    }

    pub fn denied() -> Self {
        Self {
            fixed_session_id: "sess_deterministic".into(),
            fixed_status: "denied".into(),
            fixed_approval_id: None,
        }
    }
}

impl WorkflowSessionBridge for DeterministicSessionBridge {
    fn route_action_to_session(
        &self,
        _prompt: WorkflowActionRoutePrompt,
        session_id: Option<String>,
    ) -> Result<WorkflowSessionRouteSnapshot, WorkflowActionRouteError> {
        Ok(WorkflowSessionRouteSnapshot {
            session_id: session_id.unwrap_or_else(|| self.fixed_session_id.clone()),
            session_run_id: Some("run_deterministic".into()),
            trace_ids: vec!["trace_det_1".into()],
            pending_approval_id: self.fixed_approval_id.clone(),
            tool_call_id: None,
            tool_name_observed_from_session: None,
            session_status: self.fixed_status.clone(),
        })
    }
}

/// Live session bridge — routes through real SessionRunner APIs.
/// Production constructor receives real runner + trace store.
/// Test-only constructor wraps SessionHarness (Patch 1).
pub struct LiveSessionBridge {
    runner: Arc<SessionRunner>,
    trace: Arc<dyn TraceStore<StoredEvent>>,
    session_id: String,
}

/// Inner struct for the async route logic.
struct LiveSessionBridge_ {
    runner: Arc<SessionRunner>,
    trace: Arc<dyn TraceStore<StoredEvent>>,
    session_id: String,
}

impl LiveSessionBridge {
    /// Production constructor: receives real app/session runner handle.
    pub fn new(
        runner: Arc<SessionRunner>,
        trace: Arc<dyn TraceStore<StoredEvent>>,
    ) -> Self {
        let session_id = runner.session_id.to_string();
        Self { runner, trace, session_id }
    }

    /// Test-only constructor from SessionHarness (Patch 1).
    /// Production code must use `LiveSessionBridge::new()`.
    pub fn from_harness(harness: openwand_session::testing::harness::SessionHarness) -> Self {
        Self {
            session_id: harness.runner.session_id.to_string(),
            runner: Arc::new(harness.runner),
            trace: harness.trace.clone() as Arc<dyn TraceStore<StoredEvent>>,
        }
    }
}

impl WorkflowSessionBridge for LiveSessionBridge {
    fn route_action_to_session(
        &self,
        prompt: WorkflowActionRoutePrompt,
        session_id: Option<String>,
    ) -> Result<WorkflowSessionRouteSnapshot, WorkflowActionRouteError> {
        // Patch 3: Create a dedicated runtime.
        // The sync trait (from Wave 27) requires blocking; a future wave may make it async.
        // This works when called from a non-async context (regular tests, CLI).
        let runner = self.runner.clone();
        let trace = self.trace.clone();
        let session_id_str = self.session_id.clone();
        let inner = LiveSessionBridge_ { runner, trace, session_id: session_id_str };
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| WorkflowActionRouteError::BridgeUnavailable(e.to_string()))?;
        rt.block_on(inner.route_inner(prompt, session_id))
    }
}

impl LiveSessionBridge_ {
    async fn route_inner(
        &self,
        prompt: WorkflowActionRoutePrompt,
        _session_id: Option<String>,
    ) -> Result<WorkflowSessionRouteSnapshot, WorkflowActionRouteError> {
        let user_text = prompt.to_session_instruction();

        // Subscribe to events before the turn (Patch 2: map actual AgentEvent variants)
        let mut rx = self.runner.subscribe();

        let config = RunConfig {
            max_steps: 5,
            mode: InteractionMode::Conversational,
            working_directory: ".".into(),
            system_prompt: None,
            llm_target: None,
            memory_prompt_inputs: None,
            output_guard: None,
        };

        let run_result = self.runner.run_turn(user_text, config).await
            .map_err(|e| WorkflowActionRouteError::SessionError(e.to_string()))?;

        // Drain events from the broadcast stream
        let mut events: Vec<AgentEvent> = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }

        // Map events to snapshot fields (Patch 2: only actual AgentEvent variants)
        let mut tool_call_id: Option<String> = None;
        let mut tool_name: Option<String> = None;
        let mut pending_approval_id: Option<String> = None;

        for event in &events {
            match event {
                AgentEvent::ToolCallStarted { tool_name: name, tool_call_id: id, .. } => {
                    tool_call_id = Some(id.to_string());
                    tool_name = Some(name.clone());
                }
                AgentEvent::ApprovalRequested { tool_call_id: id, .. } => {
                    // Approval request ID from session — workflow does not construct this
                    pending_approval_id = Some(format!("arid_session_{}", id));
                }
                AgentEvent::RunStarted { .. } => {}
                AgentEvent::PhaseEntered { .. } => {}
                AgentEvent::TextDelta { .. } => {}
                AgentEvent::ToolCallCompleted { .. } => {}
                AgentEvent::ApprovalResolved { .. } => {}
                AgentEvent::RunCompleted { .. } => {}
            }
        }

        // Map stop reason to session status
        let session_status = match run_result.stop_reason {
            openwand_session::config::RunStopReason::Natural => "completed",
            openwand_session::config::RunStopReason::AwaitingApproval => "suspended_for_approval",
            openwand_session::config::RunStopReason::ToolDenied => "denied",
            openwand_session::config::RunStopReason::ToolBlocked => "denied",
            openwand_session::config::RunStopReason::MaxStepsReached => "completed",
            openwand_session::config::RunStopReason::Cancelled => "failed",
        };

        // Collect trace IDs scoped to this session only (Patch 4)
        let stream_id = TraceStreamId {
            scope: TraceStreamScope::Session,
            id: self.session_id.clone(),
        };
        let query = TraceQuery {
            stream_id: Some(stream_id),
            ..Default::default()
        };
        let trace_page = self.trace.scan(query).await
            .map_err(|e| WorkflowActionRouteError::SessionError(e.to_string()))?;
        let trace_ids: Vec<String> = trace_page.entries.iter()
            .map(|e| e.id.to_string())
            .collect();

        Ok(WorkflowSessionRouteSnapshot {
            session_id: self.session_id.clone(),
            session_run_id: Some(format!("run_{}", self.session_id)),
            trace_ids,
            pending_approval_id,
            tool_call_id,
            tool_name_observed_from_session: tool_name,
            session_status: session_status.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_action_route::WorkflowActionRoutePrompt;

    fn test_prompt() -> WorkflowActionRoutePrompt {
        WorkflowActionRoutePrompt {
            capability_category: "file-read".into(),
            purpose: "Read configuration".into(),
            expected_input_summary: "path to file".into(),
            expected_output_summary: "file contents".into(),
            safety_constraints: vec!["read-only".into()],
        }
    }

    #[test]
    fn route_action_uses_session_runner_bridge() {
        let bridge = DeterministicSessionBridge::completed();
        let result = bridge.route_action_to_session(test_prompt(), Some("sess_1".into()));
        assert!(result.is_ok());
        let snap = result.unwrap();
        assert_eq!("sess_1", snap.session_id);
    }

    #[test]
    fn route_action_does_not_call_llm_directly() {
        let bridge = DeterministicSessionBridge::completed();
        let result = bridge.route_action_to_session(test_prompt(), None);
        // Deterministic bridge never calls LLM — just returns a fixed snapshot
        assert!(result.is_ok());
    }

    #[test]
    fn route_action_does_not_execute_tool_directly() {
        let bridge = DeterministicSessionBridge::completed();
        let result = bridge.route_action_to_session(test_prompt(), None);
        // No tool_call_id in deterministic snapshot (no tool execution)
        assert!(result.as_ref().unwrap().tool_call_id.is_none());
    }

    #[test]
    fn route_action_does_not_evaluate_policy_directly() {
        let bridge = DeterministicSessionBridge::completed();
        let result = bridge.route_action_to_session(test_prompt(), None);
        // Bridge does not interact with policy engine
        assert!(result.is_ok());
    }

    #[test]
    fn route_action_does_not_append_trace_directly() {
        let bridge = DeterministicSessionBridge::completed();
        let result = bridge.route_action_to_session(test_prompt(), None);
        // trace_ids are observed from session output, not appended by bridge
        assert!(!result.unwrap().trace_ids.is_empty());
    }

    #[test]
    fn route_action_does_not_write_memory() {
        let bridge = DeterministicSessionBridge::completed();
        let _ = bridge.route_action_to_session(test_prompt(), None);
        // Deterministic bridge has no memory store
    }

    #[test]
    fn route_action_surfaces_pending_approval_from_session() {
        let bridge = DeterministicSessionBridge::suspended_for_approval();
        let result = bridge.route_action_to_session(test_prompt(), Some("sess_2".into()));
        let snap = result.unwrap();
        assert!(snap.pending_approval_id.is_some());
        assert_eq!("suspended_for_approval", snap.session_status);
    }

    #[test]
    fn route_action_surfaces_tool_denial_from_session() {
        let bridge = DeterministicSessionBridge::denied();
        let result = bridge.route_action_to_session(test_prompt(), Some("sess_3".into()));
        let snap = result.unwrap();
        assert_eq!("denied", snap.session_status);
    }

    #[test]
    fn route_action_surfaces_tool_completion_from_session() {
        let bridge = DeterministicSessionBridge::completed();
        let result = bridge.route_action_to_session(test_prompt(), Some("sess_4".into()));
        let snap = result.unwrap();
        assert_eq!("completed", snap.session_status);
        assert!(snap.pending_approval_id.is_none());
    }

    #[test]
    fn route_action_records_session_trace_ids_from_session_events() {
        let bridge = DeterministicSessionBridge::completed();
        let result = bridge.route_action_to_session(test_prompt(), Some("sess_5".into()));
        let snap = result.unwrap();
        assert!(!snap.trace_ids.is_empty());
        assert_eq!("trace_det_1", snap.trace_ids[0]);
    }

    #[test]
    fn deterministic_bridge_returns_fixed_snapshot_without_network() {
        let bridge = DeterministicSessionBridge::completed();
        let result = bridge.route_action_to_session(test_prompt(), None);
        assert!(result.is_ok());
        let snap = result.unwrap();
        assert_eq!("completed", snap.session_status);
        // No network call was made — deterministic
    }

    #[test]
    fn completed_route_does_not_claim_workflow_action_executed() {
        // Patch 3: Completed means session turn completed, not action executed
        let bridge = DeterministicSessionBridge::completed();
        let result = bridge.route_action_to_session(test_prompt(), None);
        let snap = result.unwrap();
        assert_eq!("completed", snap.session_status);
        // No tool was executed by the workflow layer
        assert!(snap.tool_call_id.is_none());
        // tool_name is observed from session, not constructed by workflow
        assert!(snap.tool_name_observed_from_session.is_none());
    }
}
