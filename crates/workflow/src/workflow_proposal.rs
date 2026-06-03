//! Workflow proposal DTOs.
//!
//! A workflow proposal describes ordered stages, tool-intent placeholders,
//! required approval markers, and abort/rollback notes derived from an
//! approved task plan. It is evidence, not a workflow run.
//!
//! Structurally omits all executable fields: no tool_name, tool_args,
//! command, shell, script, cwd, env, function_ref, workflow_run_id,
//! execution_grant, approval_request_id, process, git_args.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::plan::{TaskPlanEvidenceKind, TaskPlanId};
use crate::plan_review::TaskPlanReviewId;

/// Content-addressed proposal ID. Format: `wfp_<blake3_hex>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowProposalId(pub String);

impl WorkflowProposalId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Non-executing workflow proposal derived from an approved task plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowProposal {
    pub proposal_id: WorkflowProposalId,
    pub source_task_plan_id: TaskPlanId,
    pub source_task_plan_review_id: TaskPlanReviewId,
    pub source_task_plan_hash: String,
    pub title: String,
    pub status: WorkflowProposalStatus,
    pub stages: Vec<WorkflowStage>,
    pub required_approvals: Vec<WorkflowApprovalMarker>,
    pub risks: Vec<WorkflowProposalRisk>,
    pub abort_rollback_notes: Vec<WorkflowAbortRollbackNote>,
    pub evidence_links: Vec<WorkflowProposalEvidenceLink>,
    pub proposal_hash: String,
    pub created_at: DateTime<Utc>,
}

/// Proposal lifecycle status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowProposalStatus {
    Draft,
    Reviewable,
    Blocked,
}

/// A single ordered stage in a workflow proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStage {
    pub stage_id: String,
    pub title: String,
    pub description: String,
    pub kind: WorkflowStageKind,
    pub order: u32,
    pub depends_on: Vec<String>,
    pub tool_intents: Vec<WorkflowToolIntent>,
    pub expected_output: String,
    pub risk_level: String,
    pub requires_approval_before_execution: bool,
    pub evidence_links: Vec<WorkflowProposalEvidenceLink>,
}

/// What kind of work a stage describes. None of these execute tools.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStageKind {
    Observe,
    Analyze,
    PrepareChange,
    RequestApproval,
    ApplyChange,
    Verify,
    Report,
}

/// Describes intended tool category/action without executable args.
///
/// `capability` is a descriptive category string (e.g. "file-read",
/// "text-search"), NOT a registered tool name. It must not equal any
/// ToolExecutor tool name and must not contain executable arguments,
/// refspecs, commands, shell snippets, or JSON tool args.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowToolIntent {
    pub intent_id: String,
    /// Descriptive category, not a registered tool name.
    pub capability: String,
    pub purpose: String,
    pub expected_input_summary: String,
    pub expected_output_summary: String,
    pub requires_policy_gate: bool,
}

/// Indicates a future approval requirement without creating an approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowApprovalMarker {
    pub marker_id: String,
    pub stage_id: String,
    pub reason: String,
    pub required_before: String,
}

/// A risk identified in the proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowProposalRisk {
    pub risk_level: String,
    pub summary: String,
    pub mitigation: String,
}

/// Describes abort/rollback considerations for the proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowAbortRollbackNote {
    pub stage_id: Option<String>,
    pub summary: String,
    pub recovery_hint: String,
}

/// A link to evidence that informed the proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowProposalEvidenceLink {
    pub kind: WorkflowProposalEvidenceKind,
    pub id: String,
    pub summary: String,
}

/// What kind of evidence a proposal link references.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowProposalEvidenceKind {
    TaskPlan,
    TaskPlanReview,
    TaskPlanStep,
    Goal,
    Skill,
    TraceEvent,
    MemoryClaim,
    GovernanceRecord,
    UserIntent,
}

/// Known registered tool names that `WorkflowToolIntent.capability`
/// must not equal.
pub const FORBIDDEN_TOOL_NAMES: &[&str] = &[
    "shell",
    "git",
    "read_file",
    "write_file",
    "edit_file",
    "search",
    "web_fetch",
    "web_search",
    "memory_write",
    "memory_read",
    "trace_append",
    "policy_decide",
    "tool_execute",
    "process_spawn",
];

/// Check if a capability string looks like a forbidden tool name or
/// contains executable content.
pub fn is_valid_capability_category(capability: &str) -> Result<(), String> {
    let lower = capability.to_lowercase();

    // Must not match a registered tool name
    for &forbidden in FORBIDDEN_TOOL_NAMES {
        if lower == forbidden {
            return Err(format!(
                "capability '{}' matches forbidden tool name '{}'",
                capability, forbidden
            ));
        }
    }

    // Must not contain executable markers
    let forbidden_patterns = [
        "--", " -c ", "&&", "||", "; ", "$(", "`", "|",
        ".sh ", ".bash ", ".exe ", ".cmd ",
    ];
    for pattern in &forbidden_patterns {
        if lower.contains(pattern) {
            return Err(format!(
                "capability '{}' contains executable pattern '{}'",
                capability, pattern
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_proposal_roundtrips() {
        let proposal = WorkflowProposal {
            proposal_id: WorkflowProposalId("wfp_abc123".into()),
            source_task_plan_id: TaskPlanId("tpl_xyz".into()),
            source_task_plan_review_id: TaskPlanReviewId("tpr_review".into()),
            source_task_plan_hash: "hash123".into(),
            title: "Test proposal".into(),
            status: WorkflowProposalStatus::Reviewable,
            stages: vec![],
            required_approvals: vec![],
            risks: vec![],
            abort_rollback_notes: vec![],
            evidence_links: vec![],
            proposal_hash: "phash".into(),
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&proposal).unwrap();
        let back: WorkflowProposal = serde_json::from_str(&json).unwrap();
        assert_eq!(proposal.proposal_id, back.proposal_id);
        assert_eq!(proposal.title, back.title);
    }

    #[test]
    fn workflow_proposal_id_is_content_addressed() {
        let id = WorkflowProposalId("wfp_abc123".into());
        assert!(id.as_str().starts_with("wfp_"));
    }

    #[test]
    fn workflow_stage_roundtrips() {
        let stage = WorkflowStage {
            stage_id: "stage_1".into(),
            title: "Observe".into(),
            description: "Gather context".into(),
            kind: WorkflowStageKind::Observe,
            order: 0,
            depends_on: vec![],
            tool_intents: vec![],
            expected_output: "Context summary".into(),
            risk_level: "low".into(),
            requires_approval_before_execution: false,
            evidence_links: vec![],
        };
        let json = serde_json::to_string(&stage).unwrap();
        let back: WorkflowStage = serde_json::from_str(&json).unwrap();
        assert_eq!(stage.stage_id, back.stage_id);
    }

    #[test]
    fn workflow_proposal_status_serializes_snake_case() {
        let status = WorkflowProposalStatus::Reviewable;
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("reviewable"));
        assert!(!json.contains("Reviewable"));
    }

    #[test]
    fn workflow_stage_kind_serializes_snake_case() {
        let kind = WorkflowStageKind::PrepareChange;
        let json = serde_json::to_string(&kind).unwrap();
        assert!(json.contains("prepare_change"));
    }

    #[test]
    fn workflow_tool_intent_roundtrips() {
        let intent = WorkflowToolIntent {
            intent_id: "intent_1".into(),
            capability: "context-gathering".into(),
            purpose: "Read project files".into(),
            expected_input_summary: "File paths".into(),
            expected_output_summary: "File contents".into(),
            requires_policy_gate: false,
        };
        let json = serde_json::to_string(&intent).unwrap();
        let back: WorkflowToolIntent = serde_json::from_str(&json).unwrap();
        assert_eq!(intent.intent_id, back.intent_id);
    }

    #[test]
    fn workflow_approval_marker_roundtrips() {
        let marker = WorkflowApprovalMarker {
            marker_id: "marker_1".into(),
            stage_id: "stage_3".into(),
            reason: "Changes require human review".into(),
            required_before: "stage_4".into(),
        };
        let json = serde_json::to_string(&marker).unwrap();
        let back: WorkflowApprovalMarker = serde_json::from_str(&json).unwrap();
        assert_eq!(marker.marker_id, back.marker_id);
    }

    #[test]
    fn workflow_abort_rollback_note_roundtrips() {
        let note = WorkflowAbortRollbackNote {
            stage_id: Some("stage_3".into()),
            summary: "File changes may need manual revert".into(),
            recovery_hint: "Use git checkout to revert".into(),
        };
        let json = serde_json::to_string(&note).unwrap();
        let back: WorkflowAbortRollbackNote = serde_json::from_str(&json).unwrap();
        assert_eq!(note.summary, back.summary);
    }

    #[test]
    fn workflow_proposal_evidence_link_roundtrips() {
        let link = WorkflowProposalEvidenceLink {
            kind: WorkflowProposalEvidenceKind::TaskPlan,
            id: "tpl_abc".into(),
            summary: "Source plan".into(),
        };
        let json = serde_json::to_string(&link).unwrap();
        let back: WorkflowProposalEvidenceLink = serde_json::from_str(&json).unwrap();
        assert_eq!(link.id, back.id);
    }

    #[test]
    fn workflow_proposal_evidence_kind_serializes_snake_case() {
        let kind = WorkflowProposalEvidenceKind::TaskPlanReview;
        let json = serde_json::to_string(&kind).unwrap();
        assert!(json.contains("task_plan_review"));
    }

    #[test]
    fn is_valid_capability_category_accepts_descriptive() {
        assert!(is_valid_capability_category("file-read-observation").is_ok());
        assert!(is_valid_capability_category("text-search").is_ok());
        assert!(is_valid_capability_category("context-gathering").is_ok());
    }

    #[test]
    fn is_valid_capability_category_rejects_tool_names() {
        assert!(is_valid_capability_category("shell").is_err());
        assert!(is_valid_capability_category("git").is_err());
        assert!(is_valid_capability_category("write_file").is_err());
        assert!(is_valid_capability_category("tool_execute").is_err());
    }

    #[test]
    fn is_valid_capability_category_rejects_executable_patterns() {
        assert!(is_valid_capability_category("bash && echo").is_err());
        assert!(is_valid_capability_category("$(whoami)").is_err());
        assert!(is_valid_capability_category("ls || true").is_err());
        assert!(is_valid_capability_category("cmd; echo").is_err());
    }
}
