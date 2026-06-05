//! Workflow verification readiness — DTOs and evaluation.
//!
//! Wave 47: Determine whether an evidence record is structurally eligible
//! for a future verification attempt, without performing verification.
//!
//! Boundary:
//!   Verification readiness is not verification.
//!   Eligibility is not trust promotion.
//!   Readiness does not fetch, read, execute, verify signatures, inspect artifacts,
//!   call shell/git, mutate workflow state, schedule verification, or certify truth.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::workflow_run::WorkflowExecutionId;

/// Content-addressed readiness ID. Format: `wvr_<blake3_hex>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowVerificationReadinessId(pub String);

impl WorkflowVerificationReadinessId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// What evidence type is being checked for verification readiness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationReadinessTargetKind {
    ManualResult,
    ManualResultReview,
    ManualResultReconciliationGate,
    ExternalAttestation,
    AuditPacket,
}

/// Patch 1: Status follows established readiness pattern.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationReadinessStatus {
    Ready,
    Blocked,
    Inconclusive,
}

/// Readiness decision with reason.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerificationReadinessDecision {
    Ready,
    Blocked { reason_code: String, summary: String },
    Inconclusive { reason_code: String, summary: String },
}

/// A single predicate being evaluated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationReadinessPredicate {
    TargetRecordExists,
    TargetWorkflowExecutionIdMatchesRequest,
    TargetHashMatchesRequest,
    TargetStatusEligibleForVerificationReadiness,
    ManualResultLatestReviewExists,
    ManualResultLatestReviewAccepted,
    ManualResultReviewHashMatchesManualResult,
    AttestationMarkedUnverified,
    AttestationDoesNotPromoteTrust,
    AttestationDoesNotCertifyTruth,
    AttestationReferencesAreMetadataOnly,
    AttestationHasOperatorSuppliedHash,
    AttestationSignatureClaimed,
    AuditPacketHashExists,
    AuditPacketDoesNotCertifyRecordedEvidence,
}

/// Result of evaluating a single predicate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReadinessPredicateResult {
    pub predicate: VerificationReadinessPredicate,
    pub passed: bool,
    pub reason: String,
}

/// Request to evaluate verification readiness.
#[derive(Debug, Clone)]
pub struct VerificationReadinessRequest {
    pub target_kind: VerificationReadinessTargetKind,
    pub target_id: String,
    pub workflow_execution_id: WorkflowExecutionId,
    /// Patch 2: expected hash of the target record.
    pub expected_target_hash: String,
    pub idempotency_key: String,
}

/// The readiness evaluation record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReadinessRecord {
    pub readiness_id: WorkflowVerificationReadinessId,
    pub target_kind: VerificationReadinessTargetKind,
    pub target_id: String,
    pub workflow_execution_id: WorkflowExecutionId,
    pub expected_target_hash: String,
    pub status: VerificationReadinessStatus,
    pub decision: VerificationReadinessDecision,
    pub predicate_results: Vec<VerificationReadinessPredicateResult>,
    pub computed_at: DateTime<Utc>,
    pub idempotency_key: String,
    // Patch 7: structural no-authority flags — all false
    pub performs_verification: bool,
    pub verifies_external_truth: bool,
    pub verifies_signature: bool,
    pub fetches_urls: bool,
    pub reads_artifacts: bool,
    pub executes_commands: bool,
    pub invokes_shell: bool,
    pub invokes_git: bool,
    pub promotes_trust: bool,
    pub certifies_evidence: bool,
    pub mutates_workflow_state: bool,
    pub schedules_verification: bool,
    pub creates_execution_grant: bool,
    pub execution_allowed_now: bool,
}

/// Compute a content-addressed readiness ID.
pub fn compute_readiness_id(
    target_id: &str,
    workflow_execution_id: &str,
    expected_target_hash: &str,
    idempotency_key: &str,
) -> WorkflowVerificationReadinessId {
    let input = format!("{}{}{}{}", target_id, workflow_execution_id, expected_target_hash, idempotency_key);
    let hash = blake3::hash(input.as_bytes());
    WorkflowVerificationReadinessId(format!("wvr_{}", &hash.to_hex()[..16]))
}

/// Build a readiness record from evaluation results.
pub fn build_readiness_record(
    request: &VerificationReadinessRequest,
    predicate_results: Vec<VerificationReadinessPredicateResult>,
) -> VerificationReadinessRecord {
    let readiness_id = compute_readiness_id(
        &request.target_id,
        &request.workflow_execution_id.0,
        &request.expected_target_hash,
        &request.idempotency_key,
    );

    let all_passed = predicate_results.iter().all(|r| r.passed);
    let any_inconclusive = predicate_results.iter().any(|r| {
        !r.passed && r.reason.to_lowercase().contains("inconclusive")
    });

    let (status, decision) = if all_passed {
        (VerificationReadinessStatus::Ready, VerificationReadinessDecision::Ready)
    } else if any_inconclusive {
        let failed: Vec<&str> = predicate_results.iter()
            .filter(|r| !r.passed)
            .map(|r| r.reason.as_str())
            .collect();
        (VerificationReadinessStatus::Inconclusive,
         VerificationReadinessDecision::Inconclusive {
             reason_code: "inconclusive_predicate".into(),
             summary: failed.join("; "),
         })
    } else {
        let failed: Vec<&str> = predicate_results.iter()
            .filter(|r| !r.passed)
            .map(|r| r.reason.as_str())
            .collect();
        (VerificationReadinessStatus::Blocked,
         VerificationReadinessDecision::Blocked {
             reason_code: "predicate_failed".into(),
             summary: failed.join("; "),
         })
    };

    VerificationReadinessRecord {
        readiness_id,
        target_kind: request.target_kind.clone(),
        target_id: request.target_id.clone(),
        workflow_execution_id: request.workflow_execution_id.clone(),
        expected_target_hash: request.expected_target_hash.clone(),
        status,
        decision,
        predicate_results,
        computed_at: Utc::now(),
        idempotency_key: request.idempotency_key.clone(),
        // Patch 7: all false
        performs_verification: false,
        verifies_external_truth: false,
        verifies_signature: false,
        fetches_urls: false,
        reads_artifacts: false,
        executes_commands: false,
        invokes_shell: false,
        invokes_git: false,
        promotes_trust: false,
        certifies_evidence: false,
        mutates_workflow_state: false,
        schedules_verification: false,
        creates_execution_grant: false,
        execution_allowed_now: false,
    }
}

pub(crate) fn p(predicate: VerificationReadinessPredicate, passed: bool, reason: &str) -> VerificationReadinessPredicateResult {
    VerificationReadinessPredicateResult { predicate, passed, reason: reason.into() }
}

/// Evaluate verification readiness for a target where the record cannot be loaded.
pub fn evaluate_readiness_target_not_found(
    request: &VerificationReadinessRequest,
) -> VerificationReadinessRecord {
    let predicates = vec![
        p(VerificationReadinessPredicate::TargetRecordExists, false, "Target record not found"),
    ];
    build_readiness_record(request, predicates)
}

/// Evaluate verification readiness when only structural metadata is available.
pub fn evaluate_readiness_metadata_only(
    request: &VerificationReadinessRequest,
    target_status_str: &str,
    target_hash: &str,
    target_workflow_id: &str,
) -> VerificationReadinessRecord {
    let mut predicates = Vec::new();

    // Target exists (we have metadata)
    predicates.push(p(VerificationReadinessPredicate::TargetRecordExists, true, "Target record loaded"));

    // Patch 2: workflow execution ID match
    let wfx_match = target_workflow_id == request.workflow_execution_id.0;
    predicates.push(p(VerificationReadinessPredicate::TargetWorkflowExecutionIdMatchesRequest,
        wfx_match, if wfx_match { "Match" } else { "Mismatch" }));

    // Patch 2: target hash match
    let hash_match = target_hash == request.expected_target_hash;
    predicates.push(p(VerificationReadinessPredicate::TargetHashMatchesRequest,
        hash_match, if hash_match { "Match" } else { "Hash mismatch" }));

    // Patch 3: target-specific status eligibility
    let status_eligible = match request.target_kind {
        VerificationReadinessTargetKind::ManualResult => {
            !matches!(target_status_str, "not_performed")
        }
        VerificationReadinessTargetKind::ManualResultReview => {
            matches!(target_status_str, "accepted")
        }
        VerificationReadinessTargetKind::ManualResultReconciliationGate => {
            matches!(target_status_str, "reconciled" | "already_reconciled")
        }
        VerificationReadinessTargetKind::ExternalAttestation => true,
        VerificationReadinessTargetKind::AuditPacket => true,
    };
    predicates.push(p(VerificationReadinessPredicate::TargetStatusEligibleForVerificationReadiness,
        status_eligible, if status_eligible { "Eligible" } else { "Status not eligible" }));

    build_readiness_record(request, predicates)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_request() -> VerificationReadinessRequest {
        VerificationReadinessRequest {
            target_kind: VerificationReadinessTargetKind::ManualResult,
            target_id: "wmr_t".into(),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            expected_target_hash: "hash_t".into(),
            idempotency_key: "key1".into(),
        }
    }

    // Patch 1: status consistency
    #[test]
    fn verification_readiness_status_serializes_ready_blocked_inconclusive() {
        let s1 = serde_json::to_string(&VerificationReadinessStatus::Ready).unwrap();
        assert!(s1.contains("ready"));
        let s2 = serde_json::to_string(&VerificationReadinessStatus::Blocked).unwrap();
        assert!(s2.contains("blocked"));
        let s3 = serde_json::to_string(&VerificationReadinessStatus::Inconclusive).unwrap();
        assert!(s3.contains("inconclusive"));
    }

    #[test]
    fn blocked_decision_sets_blocked_status() {
        let req = test_request();
        let rec = evaluate_readiness_target_not_found(&req);
        assert_eq!(VerificationReadinessStatus::Blocked, rec.status);
    }

    // Patch 2: hash binding
    #[test]
    fn blocks_target_hash_mismatch() {
        let rec = evaluate_readiness_metadata_only(
            &test_request(), "reported_succeeded", "wrong_hash", "wfx_t",
        );
        assert_eq!(VerificationReadinessStatus::Blocked, rec.status);
        assert!(rec.predicate_results.iter().any(|r|
            r.predicate == VerificationReadinessPredicate::TargetHashMatchesRequest && !r.passed));
    }

    #[test]
    fn blocks_workflow_execution_id_mismatch() {
        let rec = evaluate_readiness_metadata_only(
            &test_request(), "reported_succeeded", "hash_t", "wfx_other",
        );
        assert_eq!(VerificationReadinessStatus::Blocked, rec.status);
        assert!(rec.predicate_results.iter().any(|r|
            r.predicate == VerificationReadinessPredicate::TargetWorkflowExecutionIdMatchesRequest && !r.passed));
    }

    #[test]
    fn ready_requires_loaded_target_record() {
        let rec = evaluate_readiness_metadata_only(
            &test_request(), "reported_succeeded", "hash_t", "wfx_t",
        );
        assert_eq!(VerificationReadinessStatus::Ready, rec.status);
    }

    // Patch 3: target-specific status
    #[test]
    fn manual_result_not_performed_blocks() {
        let rec = evaluate_readiness_metadata_only(
            &test_request(), "not_performed", "hash_t", "wfx_t",
        );
        assert_eq!(VerificationReadinessStatus::Blocked, rec.status);
    }

    #[test]
    fn manual_result_reported_succeeded_is_eligible() {
        let rec = evaluate_readiness_metadata_only(
            &test_request(), "reported_succeeded", "hash_t", "wfx_t",
        );
        assert_eq!(VerificationReadinessStatus::Ready, rec.status);
    }

    #[test]
    fn manual_result_reported_failed_is_eligible() {
        let rec = evaluate_readiness_metadata_only(
            &test_request(), "reported_failed", "hash_t", "wfx_t",
        );
        assert_eq!(VerificationReadinessStatus::Ready, rec.status);
    }

    #[test]
    fn manual_result_reported_partial_is_eligible() {
        let rec = evaluate_readiness_metadata_only(
            &test_request(), "reported_partial", "hash_t", "wfx_t",
        );
        assert_eq!(VerificationReadinessStatus::Ready, rec.status);
    }

    #[test]
    fn review_rejected_blocks() {
        let mut req = test_request();
        req.target_kind = VerificationReadinessTargetKind::ManualResultReview;
        let rec = evaluate_readiness_metadata_only(&req, "rejected", "hash_t", "wfx_t");
        assert_eq!(VerificationReadinessStatus::Blocked, rec.status);
    }

    #[test]
    fn review_accepted_is_eligible() {
        let mut req = test_request();
        req.target_kind = VerificationReadinessTargetKind::ManualResultReview;
        let rec = evaluate_readiness_metadata_only(&req, "accepted", "hash_t", "wfx_t");
        assert_eq!(VerificationReadinessStatus::Ready, rec.status);
    }

    #[test]
    fn gate_blocked_blocks() {
        let mut req = test_request();
        req.target_kind = VerificationReadinessTargetKind::ManualResultReconciliationGate;
        let rec = evaluate_readiness_metadata_only(&req, "blocked", "hash_t", "wfx_t");
        assert_eq!(VerificationReadinessStatus::Blocked, rec.status);
    }

    #[test]
    fn gate_reconciled_is_eligible() {
        let mut req = test_request();
        req.target_kind = VerificationReadinessTargetKind::ManualResultReconciliationGate;
        let rec = evaluate_readiness_metadata_only(&req, "reconciled", "hash_t", "wfx_t");
        assert_eq!(VerificationReadinessStatus::Ready, rec.status);
    }

    // Patch 7: no-authority flags
    #[test]
    fn verification_readiness_record_has_no_verification_authority() {
        let rec = evaluate_readiness_metadata_only(
            &test_request(), "reported_succeeded", "hash_t", "wfx_t",
        );
        assert!(!rec.performs_verification);
        assert!(!rec.verifies_external_truth);
        assert!(!rec.verifies_signature);
        assert!(!rec.fetches_urls);
        assert!(!rec.reads_artifacts);
        assert!(!rec.executes_commands);
        assert!(!rec.invokes_shell);
        assert!(!rec.invokes_git);
        assert!(!rec.promotes_trust);
        assert!(!rec.certifies_evidence);
        assert!(!rec.mutates_workflow_state);
        assert!(!rec.schedules_verification);
        assert!(!rec.creates_execution_grant);
        assert!(!rec.execution_allowed_now);
    }

    #[test]
    fn ready_readiness_does_not_perform_verification() {
        let rec = evaluate_readiness_metadata_only(
            &test_request(), "reported_succeeded", "hash_t", "wfx_t",
        );
        assert_eq!(VerificationReadinessStatus::Ready, rec.status);
        assert!(!rec.performs_verification);
    }

    #[test]
    fn ready_readiness_does_not_promote_trust() {
        let rec = evaluate_readiness_metadata_only(
            &test_request(), "reported_succeeded", "hash_t", "wfx_t",
        );
        assert!(!rec.promotes_trust);
    }

    #[test]
    fn ready_readiness_does_not_schedule_verification() {
        let rec = evaluate_readiness_metadata_only(
            &test_request(), "reported_succeeded", "hash_t", "wfx_t",
        );
        assert!(!rec.schedules_verification);
    }

    #[test]
    fn ready_readiness_does_not_allow_execution_now() {
        let rec = evaluate_readiness_metadata_only(
            &test_request(), "reported_succeeded", "hash_t", "wfx_t",
        );
        assert!(!rec.execution_allowed_now);
    }

    // Basic
    #[test]
    fn readiness_id_has_wvr_prefix() {
        let id = compute_readiness_id("wmr_1", "wfx_1", "hash", "key");
        assert!(id.0.starts_with("wvr_"));
    }

    #[test]
    fn readiness_id_is_deterministic() {
        let a = compute_readiness_id("wmr_1", "wfx_1", "hash", "key");
        let b = compute_readiness_id("wmr_1", "wfx_1", "hash", "key");
        assert_eq!(a, b);
    }

    #[test]
    fn readiness_record_roundtrips_json() {
        let rec = evaluate_readiness_metadata_only(
            &test_request(), "reported_succeeded", "hash_t", "wfx_t",
        );
        let json = serde_json::to_string(&rec).unwrap();
        let back: VerificationReadinessRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(rec.readiness_id, back.readiness_id);
    }

    #[test]
    fn target_not_found_produces_blocked() {
        let rec = evaluate_readiness_target_not_found(&test_request());
        assert_eq!(VerificationReadinessStatus::Blocked, rec.status);
        assert!(rec.predicate_results.iter().any(|r|
            r.predicate == VerificationReadinessPredicate::TargetRecordExists && !r.passed));
    }
}
