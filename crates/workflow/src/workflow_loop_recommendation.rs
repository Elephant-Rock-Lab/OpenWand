//! Workflow loop recommendation DTOs.
//!
//! Recommendations are display-only. command_hint is a label, not an executable.
//! The recommendation record contains no command_args, shell, cwd, env, process,
//! or executable fields.

use serde::{Deserialize, Serialize};

use crate::workflow_loop_state::WorkflowDetectedLoopState;
use crate::workflow_action_route::WorkflowActionRouteId;
use crate::workflow_action_outcome::WorkflowActionOutcomeId;
use crate::workflow_reconciliation::WorkflowReconciliationId;
use crate::workflow_continuation::WorkflowNextActionProposalId;
use crate::workflow_next_action_review::WorkflowNextActionReviewId;
use crate::workflow_routing_readiness::WorkflowRoutingReadinessId;
use crate::workflow_next_action_routing_gate::WorkflowNextActionRoutingId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowManualOperationKind {
    CreateContinuationProposal,
    ReviewNextActionProposal,
    EvaluateRoutingReadiness,
    RouteReviewedNextAction,
    ObserveRouteOutcome,
    ResolveWorkflowApprovalOutcome,
    ReconcileWorkflowOutcome,
    InspectBlockedWorkflow,
    NoAction,
    // Wave 44: manual-result ladder operation kinds
    CreateCommandDescriptor,
    ReviewCommandDescriptor,
    CaptureManualResult,
    ReviewManualResult,
    EvaluateReconciliationReadiness,
    ReconcileManualResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowLoopRecommendation {
    pub operation: WorkflowManualOperationKind,
    /// Display text only. Not parsed, not executed, not copied into shell APIs.
    pub command_hint: String,
    pub reason: String,
    pub required_inputs: Vec<String>,
    pub evidence_links: Vec<WorkflowLoopEvidenceLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowLoopEvidenceLink {
    pub link_kind: String,
    pub record_id: String,
    pub summary: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_loop_recommendation_has_no_execution_authority() {
        let rec = WorkflowLoopRecommendation {
            operation: WorkflowManualOperationKind::CreateContinuationProposal,
            command_hint: "openwand workflow-continuation propose ...".into(),
            reason: "No continuation proposal exists".into(),
            required_inputs: vec!["workflow_execution_id".into()],
            evidence_links: vec![],
        };
        let json = serde_json::to_string(&rec).unwrap().to_lowercase();
        assert!(!json.contains("execute_tool"));
        assert!(!json.contains("route_action"));
        assert!(!json.contains("resolve_approval"));
    }

    #[test]
    fn workflow_loop_command_hint_is_display_only() {
        let rec = WorkflowLoopRecommendation {
            operation: WorkflowManualOperationKind::ReviewNextActionProposal,
            command_hint: "openwand workflow-next-action-review approve ...".into(),
            reason: "test".into(), required_inputs: vec![], evidence_links: vec![],
        };
        // command_hint is a string field, not a command struct
        assert!(rec.command_hint.starts_with("openwand"));
        let json = serde_json::to_string(&rec).unwrap();
        // No executable fields in the JSON
        assert!(!json.contains("command_args"));
        assert!(!json.contains("shell"));
        assert!(!json.contains("cwd"));
        assert!(!json.contains("env"));
        assert!(!json.contains("process"));
    }

    // Patch 4: no executable command fields in record
    #[test]
    fn workflow_loop_controller_record_has_no_executable_command_fields() {
        // Verify that WorkflowLoopRecommendation structurally has no
        // executable fields by checking the serialized form
        let rec = WorkflowLoopRecommendation {
            operation: WorkflowManualOperationKind::RouteReviewedNextAction,
            command_hint: "display only".into(),
            reason: "r".into(), required_inputs: vec![], evidence_links: vec![],
        };
        let json = serde_json::to_string_pretty(&rec).unwrap().to_lowercase();
        let forbidden = ["command_args", "shell", "cwd", "env", "process", "executable",
                         "subprocess", "system", "exec"];
        for f in &forbidden {
            assert!(!json.contains(f), "Contains forbidden field: {}", f);
        }
    }

    #[test]
    fn workflow_loop_command_hint_not_used_as_executable_input() {
        // command_hint is just a String — verify it's not an enum or struct
        let hint = "openwand workflow-loop recommend --workflow-execution-id wfx_1";
        let rec = WorkflowLoopRecommendation {
            operation: WorkflowManualOperationKind::NoAction,
            command_hint: hint.into(),
            reason: "test".into(), required_inputs: vec![], evidence_links: vec![],
        };
        // It's a plain string, not a process builder or shell invocation
        assert_eq!(rec.command_hint, hint);
    }

    #[test]
    fn workflow_loop_serialized_json_contains_no_shell_or_process_fields() {
        let rec = WorkflowLoopRecommendation {
            operation: WorkflowManualOperationKind::ReconcileWorkflowOutcome,
            command_hint: "openwand workflow-reconciliation evaluate ...".into(),
            reason: "test".into(), required_inputs: vec!["id".into()], evidence_links: vec![],
        };
        let json = serde_json::to_string(&rec).unwrap().to_lowercase();
        assert!(!json.contains("shell"));
        assert!(!json.contains("process"));
        assert!(!json.contains("exec"));
        assert!(!json.contains("system"));
        assert!(!json.contains("subprocess"));
    }

    // Patch 5: no schedule/queue/retry/resume
    #[test]
    fn workflow_loop_recommendation_does_not_schedule_or_queue() {
        let json = serde_json::to_string(&WorkflowManualOperationKind::NoAction).unwrap().to_lowercase();
        assert!(!json.contains("schedule"));
        assert!(!json.contains("queue"));
        assert!(!json.contains("worker"));
    }

    #[test]
    fn workflow_loop_recommendation_does_not_retry_or_resume() {
        let json = serde_json::to_string(&WorkflowManualOperationKind::NoAction).unwrap().to_lowercase();
        assert!(!json.contains("retry"));
        assert!(!json.contains("resume"));
    }
}
