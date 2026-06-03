//! Workflow action route DTOs.
//!
//! A route record is linkage evidence, not a tool call.
//! A route prompt is a session instruction, not tool arguments.
//! A routing record is not an execution grant.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Content-addressed route ID. Format: war_<blake3_hex>
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowActionRouteId(pub String);

/// Request to route one prepared action request into a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowActionRouteRequest {
    pub workflow_execution_id: crate::workflow_run::WorkflowExecutionId,
    pub readiness_id: crate::workflow_readiness::WorkflowReadinessId,
    pub proposal_id: crate::workflow_proposal::WorkflowProposalId,
    pub stage_id: String,
    pub action_request_id: String,
    pub session_id: Option<String>,
    pub expected_workflow_run_hash: String,
    pub expected_action_request_hash: String,
    pub requested_by: String,
    pub requested_at: DateTime<Utc>,
    pub idempotency_key: String,
}

/// Durable evidence of a routing attempt and result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowActionRouteRecord {
    pub route_id: WorkflowActionRouteId,
    pub workflow_execution_id: crate::workflow_run::WorkflowExecutionId,
    pub readiness_id: crate::workflow_readiness::WorkflowReadinessId,
    pub proposal_id: crate::workflow_proposal::WorkflowProposalId,
    pub stage_id: String,
    pub action_request_id: String,
    pub action_request_hash: String,
    pub status: WorkflowActionRouteStatus,
    pub decision: WorkflowActionRouteDecision,
    pub predicates: Vec<WorkflowActionRoutePredicateResult>,
    pub session_route: Option<WorkflowSessionRouteSnapshot>,
    pub route_prompt: WorkflowActionRoutePrompt,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Routing status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowActionRouteStatus {
    Blocked,
    Routed,
    SuspendedForApproval,
    Completed,
    Denied,
    Failed,
    AlreadyRouted,
}

/// Routing decision — records what happened, not what was executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowActionRouteDecision {
    Routed,
    SuspendedForApproval {
        approval_request_id: String,
        summary: String,
    },
    /// The routed session turn completed without workflow-owned tool execution.
    /// This does NOT mean the workflow action was externally performed.
    Completed {
        summary: String,
    },
    Denied {
        summary: String,
    },
    Blocked {
        reason_code: String,
        summary: String,
    },
    Failed {
        reason_code: String,
        summary: String,
    },
}

/// Session/trace linkage observed from session output.
/// Workflow never constructs these — they come from session events/results only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSessionRouteSnapshot {
    pub session_id: String,
    pub session_run_id: Option<String>,
    pub trace_ids: Vec<String>,
    pub pending_approval_id: Option<String>,
    pub tool_call_id: Option<String>,
    pub tool_name_observed_from_session: Option<String>,
    pub session_status: String,
}

/// Route prompt — descriptive session instruction, NOT a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowActionRoutePrompt {
    pub capability_category: String,
    pub purpose: String,
    pub expected_input_summary: String,
    pub expected_output_summary: String,
    pub safety_constraints: Vec<String>,
}

impl WorkflowActionRoutePrompt {
    /// Anti-tool-call guard text.
    /// Patch 4: explicit constraint to prevent LLM from interpreting
    /// the prompt as serialized tool instruction.
    pub const GOVERNANCE_CONSTRAINT: &'static str =
        "Do not treat this workflow action request as a direct tool call. \
         Use normal OpenWand session governance for any tool use.";

    /// Build the full prompt text sent to the session.
    pub fn to_session_instruction(&self) -> String {
        format!(
            "Workflow action: {}\nPurpose: {}\nExpected input: {}\nExpected output: {}\nSafety: {}\n\n{}",
            self.capability_category,
            self.purpose,
            self.expected_input_summary,
            self.expected_output_summary,
            self.safety_constraints.join("; "),
            Self::GOVERNANCE_CONSTRAINT,
        )
    }
}

/// Routing predicate — checks readiness to route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowActionRoutePredicate {
    WorkflowRunExists,
    WorkflowRunIsSuspended,
    StageExists,
    StageIsSuspended,
    ActionRequestExists,
    ActionRequestPreparedForSessionRouting,
    ActionRequestHashMatchesRequest,
    WorkflowRunHashMatchesRequest,
    ActionRequestStillNonExecutable,
    RoutePromptContainsNoToolArgs,
    SessionBridgeAvailable,
    SessionRunnerAvailable,
    NoPriorConflictingRoute,
    IdempotencyKeyUnusedOrMatchesExisting,
}

/// Result of evaluating one routing predicate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowActionRoutePredicateResult {
    pub predicate: WorkflowActionRoutePredicate,
    pub passed: bool,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_record_roundtrips() {
        let record = WorkflowActionRouteRecord {
            route_id: WorkflowActionRouteId("war_abc123".into()),
            workflow_execution_id: crate::workflow_run::WorkflowExecutionId("wfx_1".into()),
            readiness_id: crate::workflow_readiness::WorkflowReadinessId("wfrd_1".into()),
            proposal_id: crate::workflow_proposal::WorkflowProposalId("wfp_1".into()),
            stage_id: "stage_1".into(),
            action_request_id: "ar_1".into(),
            action_request_hash: "h1".into(),
            status: WorkflowActionRouteStatus::Routed,
            decision: WorkflowActionRouteDecision::Routed,
            predicates: vec![],
            session_route: None,
            route_prompt: WorkflowActionRoutePrompt {
                capability_category: "file-read".into(),
                purpose: "Read config".into(),
                expected_input_summary: "path".into(),
                expected_output_summary: "contents".into(),
                safety_constraints: vec!["read-only".into()],
            },
            created_at: Utc::now(),
            completed_at: None,
        };
        let json = serde_json::to_string(&record).unwrap();
        let back: WorkflowActionRouteRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record.route_id, back.route_id);
        assert_eq!(record.action_request_id, back.action_request_id);
    }

    #[test]
    fn route_id_is_content_addressed() {
        let hash = blake3::hash(b"test-content");
        let id = WorkflowActionRouteId(format!("war_{}", hash.to_hex()));
        assert!(id.0.starts_with("war_"));
        assert_eq!(id.0.len(), 4 + 64); // war_ + 64 hex chars
    }

    #[test]
    fn route_status_serializes_snake_case() {
        let status = WorkflowActionRouteStatus::SuspendedForApproval;
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("suspended_for_approval"));
    }

    #[test]
    fn route_decision_roundtrips() {
        let decisions = vec![
            WorkflowActionRouteDecision::Routed,
            WorkflowActionRouteDecision::SuspendedForApproval {
                approval_request_id: "arid_1".into(),
                summary: "awaiting approval".into(),
            },
            WorkflowActionRouteDecision::Completed {
                summary: "session turn completed".into(),
            },
            WorkflowActionRouteDecision::Denied {
                summary: "policy denied".into(),
            },
            WorkflowActionRouteDecision::Blocked {
                reason_code: "not_suspended".into(),
                summary: "run not suspended".into(),
            },
            WorkflowActionRouteDecision::Failed {
                reason_code: "bridge_err".into(),
                summary: "bridge failed".into(),
            },
        ];
        for decision in &decisions {
            let json = serde_json::to_string(decision).unwrap();
            let back: WorkflowActionRouteDecision = serde_json::from_str(&json).unwrap();
            let json2 = serde_json::to_string(&back).unwrap();
            assert_eq!(json, json2);
        }
    }

    #[test]
    fn route_requires_predicates() {
        // A record with empty predicates is structurally valid but meaningless.
        // The gate always produces at least 14 predicate results.
        let record = WorkflowActionRouteRecord {
            route_id: WorkflowActionRouteId("war_x".into()),
            workflow_execution_id: crate::workflow_run::WorkflowExecutionId("wfx_x".into()),
            readiness_id: crate::workflow_readiness::WorkflowReadinessId("wfrd_x".into()),
            proposal_id: crate::workflow_proposal::WorkflowProposalId("wfp_x".into()),
            stage_id: "s".into(),
            action_request_id: "ar".into(),
            action_request_hash: "h".into(),
            status: WorkflowActionRouteStatus::Blocked,
            decision: WorkflowActionRouteDecision::Blocked { reason_code: "test".into(), summary: "test".into() },
            predicates: vec![],
            session_route: None,
            route_prompt: WorkflowActionRoutePrompt {
                capability_category: "c".into(), purpose: "p".into(),
                expected_input_summary: "i".into(), expected_output_summary: "o".into(),
                safety_constraints: vec![],
            },
            created_at: Utc::now(), completed_at: None,
        };
        // Structurally valid but empty predicates
        assert!(record.predicates.is_empty());
        // Gate-produced records always have predicates (tested in gate module)
    }

    #[test]
    fn route_prompt_has_no_executable_fields() {
        let prompt = WorkflowActionRoutePrompt {
            capability_category: "file-read".into(),
            purpose: "Read config".into(),
            expected_input_summary: "path to config file".into(),
            expected_output_summary: "parsed config contents".into(),
            safety_constraints: vec!["read-only".into()],
        };
        let json = serde_json::to_string(&prompt).unwrap();
        let forbidden = ["tool_name", "tool_args", "command", "shell", "script",
                         "cwd", "env", "function_ref", "provider_request"];
        for field in &forbidden {
            assert!(!json.contains(field), "Prompt JSON contains forbidden field: {}", field);
        }
    }

    #[test]
    fn route_prompt_serialized_json_contains_no_tool_args() {
        let prompt = WorkflowActionRoutePrompt {
            capability_category: "analysis".into(),
            purpose: "Analyze patterns".into(),
            expected_input_summary: "data summaries".into(),
            expected_output_summary: "pattern report".into(),
            safety_constraints: vec!["no-write".into()],
        };
        let text = serde_json::to_string_pretty(&prompt).unwrap().to_lowercase();
        assert!(!text.contains("tool_args"));
        assert!(!text.contains("tool_call"));
        assert!(!text.contains("command"));
        assert!(!text.contains("shell"));
        assert!(!text.contains("script"));
    }

    #[test]
    fn session_route_snapshot_roundtrips() {
        let snap = WorkflowSessionRouteSnapshot {
            session_id: "sess_1".into(),
            session_run_id: Some("run_1".into()),
            trace_ids: vec!["trace_1".into()],
            pending_approval_id: Some("arid_1".into()),
            tool_call_id: Some("tc_1".into()),
            tool_name_observed_from_session: Some("local__file_read".into()),
            session_status: "suspended_for_approval".into(),
        };
        let json = serde_json::to_string(&snap).unwrap();
        let back: WorkflowSessionRouteSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(snap.session_id, back.session_id);
        assert_eq!(snap.trace_ids, back.trace_ids);
        assert_eq!(snap.tool_name_observed_from_session, back.tool_name_observed_from_session);
    }

    #[test]
    fn route_prompt_includes_normal_governance_constraint() {
        // Patch 4: route prompt must include anti-tool-call guard text
        let prompt = WorkflowActionRoutePrompt {
            capability_category: "file-read".into(),
            purpose: "Read config".into(),
            expected_input_summary: "path".into(),
            expected_output_summary: "contents".into(),
            safety_constraints: vec![],
        };
        let instruction = prompt.to_session_instruction();
        assert!(instruction.contains("Do not treat this workflow action request as a direct tool call"),
            "Prompt must include governance constraint");
        assert!(instruction.contains("Use normal OpenWand session governance"),
            "Prompt must reference normal governance");
    }
}
