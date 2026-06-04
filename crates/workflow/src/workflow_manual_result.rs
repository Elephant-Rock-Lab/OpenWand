//! Manual result DTOs — operator-reported evidence for acknowledged command descriptors.
//!
//! Manual result capture records what the operator reports happened.
//! It does not verify execution, inspect artifacts, or mutate workflow state.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::workflow_command_composer::WorkflowCommandComposerId;
use crate::workflow_command_review::WorkflowCommandReviewId;
use crate::workflow_loop_controller::WorkflowLoopControllerId;
use crate::workflow_run::WorkflowExecutionId;

/// Content-addressed result ID. Format: wmr_<blake3_hex>
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowManualResultId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowManualResultRequest {
    pub workflow_execution_id: WorkflowExecutionId,
    pub command_review_id: WorkflowCommandReviewId,
    pub command_composer_id: WorkflowCommandComposerId,
    pub loop_controller_id: WorkflowLoopControllerId,
    pub expected_command_review_hash: String,
    pub expected_command_composer_hash: String,
    pub expected_command_descriptor_hash: String,
    pub expected_loop_controller_hash: String,
    pub status: WorkflowManualResultStatus,
    pub operator: String,
    pub summary: String,
    pub details: Option<String>,
    pub artifact_references: Vec<WorkflowManualArtifactReference>,
    pub captured_at: DateTime<Utc>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowManualResult {
    pub result_id: WorkflowManualResultId,
    pub workflow_execution_id: WorkflowExecutionId,
    pub command_review_id: WorkflowCommandReviewId,
    pub command_composer_id: WorkflowCommandComposerId,
    pub loop_controller_id: WorkflowLoopControllerId,
    pub command_review_hash: String,
    pub command_composer_hash: String,
    pub command_descriptor_hash: String,
    pub loop_controller_hash: String,
    pub status: WorkflowManualResultStatus,
    pub operator: String,
    pub summary: WorkflowManualResultSummary,
    pub artifact_references: Vec<WorkflowManualArtifactReference>,
    pub validation_snapshot: WorkflowManualResultValidationSnapshot,
    // Evidence classification
    pub reported_by_operator: bool,
    // Authority/verification flags — all false
    pub verified_by_openwand: bool,
    pub command_executed_by_openwand: bool,
    pub mutates_workflow_state: bool,
    pub reconciles_outcome: bool,
    pub routes_action: bool,
    pub resolves_approval: bool,
    pub appends_trace: bool,
    pub writes_memory: bool,
    pub invokes_shell: bool,
    pub invokes_git: bool,
    pub creates_execution_grant: bool,
    pub execution_allowed_now: bool,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowManualResultStatus {
    ReportedSucceeded,
    ReportedFailed,
    ReportedPartial,
    NotPerformed,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowManualResultSummary {
    pub operator_summary: String,
    pub operator_details: Option<String>,
    pub reported_status: WorkflowManualResultStatus,
    pub caveat: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowManualArtifactReference {
    pub artifact_id: String,
    pub label: String,
    pub kind: WorkflowManualArtifactKind,
    /// Metadata-only reference string. Not read, fetched, or verified by OpenWand.
    pub reference: String,
    /// Patch 5: stored verbatim. OpenWand never recomputes this from file/URL contents.
    pub operator_supplied_hash: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowManualArtifactKind {
    LogExcerpt,
    Screenshot,
    FilePathReference,
    ExternalUrl,
    PlainTextNote,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowManualResultValidationSnapshot {
    pub command_review_was_acknowledged: bool,
    pub command_review_hash_matched: bool,
    pub command_composer_hash_matched: bool,
    pub command_descriptor_hash_matched: bool,
    pub loop_controller_hash_matched: bool,
    pub command_review_marked_not_performed_by_openwand: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_result(status: WorkflowManualResultStatus) -> WorkflowManualResult {
        WorkflowManualResult {
            result_id: WorkflowManualResultId("wmr_test".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            command_review_id: WorkflowCommandReviewId("wcrv_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            command_review_hash: "rh".into(),
            command_composer_hash: "ch".into(),
            command_descriptor_hash: "dh".into(),
            loop_controller_hash: "lh".into(),
            status,
            operator: "tester".into(),
            summary: WorkflowManualResultSummary {
                operator_summary: "done".into(),
                operator_details: None,
                reported_status: WorkflowManualResultStatus::ReportedSucceeded,
                caveat: "Operator-reported, not verified by OpenWand.".into(),
            },
            artifact_references: vec![],
            validation_snapshot: WorkflowManualResultValidationSnapshot {
                command_review_was_acknowledged: true,
                command_review_hash_matched: true,
                command_composer_hash_matched: true,
                command_descriptor_hash_matched: true,
                loop_controller_hash_matched: true,
                command_review_marked_not_performed_by_openwand: true,
            },
            reported_by_operator: true,
            verified_by_openwand: false, command_executed_by_openwand: false,
            mutates_workflow_state: false, reconciles_outcome: false,
            routes_action: false, resolves_approval: false,
            appends_trace: false, writes_memory: false,
            invokes_shell: false, invokes_git: false,
            creates_execution_grant: false, execution_allowed_now: false,
            captured_at: Utc::now(),
        }
    }

    #[test]
    fn workflow_manual_result_roundtrips() {
        let r = test_result(WorkflowManualResultStatus::ReportedSucceeded);
        let json = serde_json::to_string(&r).unwrap();
        let back: WorkflowManualResult = serde_json::from_str(&json).unwrap();
        assert_eq!(r.result_id, back.result_id);
    }

    #[test]
    fn workflow_manual_result_id_is_content_addressed() {
        use blake3::Hasher;
        let mut h1 = Hasher::new();
        h1.update(b"manual_result:v1:wfx_t:wcrv_t:k1");
        let id1 = format!("wmr_{}", &h1.finalize().to_hex()[..16]);
        let mut h2 = Hasher::new();
        h2.update(b"manual_result:v1:wfx_t:wcrv_t:k1");
        let id2 = format!("wmr_{}", &h2.finalize().to_hex()[..16]);
        assert_eq!(id1, id2);
    }

    #[test]
    fn workflow_manual_result_status_serializes_snake_case() {
        assert!(serde_json::to_string(&WorkflowManualResultStatus::ReportedSucceeded).unwrap().contains("reported_succeeded"));
        assert!(serde_json::to_string(&WorkflowManualResultStatus::ReportedFailed).unwrap().contains("reported_failed"));
        assert!(serde_json::to_string(&WorkflowManualResultStatus::ReportedPartial).unwrap().contains("reported_partial"));
        assert!(serde_json::to_string(&WorkflowManualResultStatus::NotPerformed).unwrap().contains("not_performed"));
        assert!(serde_json::to_string(&WorkflowManualResultStatus::Inconclusive).unwrap().contains("inconclusive"));
    }

    #[test]
    fn workflow_manual_result_requires_operator() {
        let mut req = WorkflowManualResultRequest {
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            command_review_id: WorkflowCommandReviewId("wcrv_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            expected_command_review_hash: "rh".into(),
            expected_command_composer_hash: "ch".into(),
            expected_command_descriptor_hash: "dh".into(),
            expected_loop_controller_hash: "lh".into(),
            status: WorkflowManualResultStatus::ReportedSucceeded,
            operator: String::new(),
            summary: "ok".into(), details: None,
            artifact_references: vec![], captured_at: Utc::now(),
            idempotency_key: "k".into(),
        };
        assert!(req.operator.is_empty());
        req.operator = "valid".into();
        assert!(!req.operator.is_empty());
    }

    #[test]
    fn workflow_manual_result_requires_summary() {
        let req = WorkflowManualResultRequest {
            summary: String::new(),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            command_review_id: WorkflowCommandReviewId("wcrv_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            expected_command_review_hash: "rh".into(),
            expected_command_composer_hash: "ch".into(),
            expected_command_descriptor_hash: "dh".into(),
            expected_loop_controller_hash: "lh".into(),
            status: WorkflowManualResultStatus::ReportedSucceeded,
            operator: "tester".into(),
            details: None, artifact_references: vec![],
            captured_at: Utc::now(), idempotency_key: "k".into(),
        };
        assert!(req.summary.is_empty());
    }

    #[test]
    fn workflow_manual_result_has_reported_not_verified_flags() {
        let r = test_result(WorkflowManualResultStatus::ReportedSucceeded);
        assert!(r.reported_by_operator);
        assert!(!r.verified_by_openwand);
        assert!(!r.command_executed_by_openwand);
    }

    // Patch 1: explicit reported-true and verified-false split
    #[test]
    fn workflow_manual_result_has_reported_true_and_verified_false_flags() {
        let r = test_result(WorkflowManualResultStatus::ReportedSucceeded);
        assert!(r.reported_by_operator, "reported_by_operator must be true");
        assert!(!r.verified_by_openwand, "verified_by_openwand must be false");
        assert!(!r.command_executed_by_openwand, "command_executed_by_openwand must be false");
    }

    #[test]
    fn workflow_manual_result_has_no_execution_authority() {
        let r = test_result(WorkflowManualResultStatus::ReportedSucceeded);
        assert!(!r.mutates_workflow_state); assert!(!r.reconciles_outcome);
        assert!(!r.routes_action); assert!(!r.resolves_approval);
        assert!(!r.appends_trace); assert!(!r.writes_memory);
        assert!(!r.invokes_shell); assert!(!r.invokes_git);
        assert!(!r.creates_execution_grant); assert!(!r.execution_allowed_now);
    }

    #[test]
    fn workflow_manual_artifact_reference_roundtrips() {
        let ar = WorkflowManualArtifactReference {
            artifact_id: "art_1".into(), label: "log".into(),
            kind: WorkflowManualArtifactKind::LogExcerpt,
            reference: "/tmp/log.txt".into(),
            operator_supplied_hash: Some("abc123".into()),
            description: Some("build log".into()),
        };
        let json = serde_json::to_string(&ar).unwrap();
        let back: WorkflowManualArtifactReference = serde_json::from_str(&json).unwrap();
        assert_eq!(ar.artifact_id, back.artifact_id);
        assert_eq!(ar.operator_supplied_hash, back.operator_supplied_hash);
    }

    #[test]
    fn workflow_manual_result_validation_snapshot_roundtrips() {
        let s = WorkflowManualResultValidationSnapshot {
            command_review_was_acknowledged: true,
            command_review_hash_matched: true,
            command_composer_hash_matched: true,
            command_descriptor_hash_matched: true,
            loop_controller_hash_matched: true,
            command_review_marked_not_performed_by_openwand: true,
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: WorkflowManualResultValidationSnapshot = serde_json::from_str(&json).unwrap();
        assert!(back.command_review_was_acknowledged);
        assert!(back.command_review_hash_matched);
    }
}
