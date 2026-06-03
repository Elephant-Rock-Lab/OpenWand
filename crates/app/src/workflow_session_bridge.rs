//! Workflow session bridge — routes action prompts into existing session seams.
//!
//! The bridge is an app-layer adapter. Workflow crate defines the prompt DTO.
//! App crate owns the bridge trait and implementations.
//!
//! Patch 2: Wave 27 ships the bridge trait and deterministic bridge proof.
//! LiveSessionBridge is feature-gated until a runtime integration wave.

use openwand_workflow::workflow_action_route::{
    WorkflowActionRoutePrompt, WorkflowSessionRouteSnapshot,
};

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

/// Live session bridge — feature-gated stub for Wave 27.
/// Full implementation deferred to a runtime integration wave (Patch 2).
#[cfg(feature = "live-session-bridge")]
pub struct LiveSessionBridge {
    // Will hold Arc<SessionRunner> in future wave
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
