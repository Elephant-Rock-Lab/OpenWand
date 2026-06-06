//! Audit packet review — human review of exported audit packet metadata.
//!
//! Wave 49A: Records that a human reviewed an exported audit packet.
//! Review does not certify truth, verify packet contents, approve packet truth,
//! or modify the exported packet.
//!
//! Patch 1: Binds to audit_packet_hash + chain_hash + inspection_id.
//! Patch 3: Decision semantics avoid certification (ReviewedWithCaveats, etc.).
//! Patch 8: 14 no-authority flags, all false.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::workflow_run::WorkflowExecutionId;

/// Audit packet review ID. Content-addressed with `wapr_` prefix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditPacketReviewId(pub String);

/// Review decision — Patch 3: semantics avoid certification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditPacketReviewDecision {
    ReviewedWithCaveats,
    AcknowledgedForDistribution,
    NotedWithoutCertification,
}

/// Audit packet review request.
#[derive(Debug, Clone)]
pub struct AuditPacketReviewRequest {
    pub inspection_id: String,
    pub workflow_execution_id: WorkflowExecutionId,
    pub expected_audit_packet_hash: String,
    pub expected_chain_hash: String,
    pub reviewer: String,
    pub decision: AuditPacketReviewDecision,
    pub scope: String,
    pub caveats: Vec<String>,
    pub idempotency_key: String,
}

/// Audit packet review record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditPacketReview {
    pub review_id: AuditPacketReviewId,
    pub inspection_id: String,
    pub workflow_execution_id: WorkflowExecutionId,
    pub audit_packet_hash: String,
    pub chain_hash: String,
    pub reviewer: String,
    pub decision: AuditPacketReviewDecision,
    pub scope: String,
    pub caveats: Vec<String>,
    pub idempotency_key: String,
    pub reviewed_at: DateTime<Utc>,
    // Patch 3: snapshot fields — all false
    pub certifies_truth: bool,
    pub approves_packet_truth: bool,
    pub verifies_packet_contents: bool,
    // Patch 8: no-authority flags — all false
    pub certifies_external_truth: bool,
    pub verifies_delivery: bool,
    pub proves_recipient_acceptance: bool,
    pub modifies_audit_packet: bool,
    pub mutates_workflow_state: bool,
    pub promotes_trust: bool,
    pub executes_commands: bool,
    pub sends_external_message: bool,
    pub uploads_files: bool,
    pub appends_trace: bool,
    pub writes_memory: bool,
    pub creates_execution_grant: bool,
    pub execution_allowed_now: bool,
}

/// Build a content-addressed review ID: wapr_<blake3(request fields)>
pub fn compute_review_id(request: &AuditPacketReviewRequest) -> AuditPacketReviewId {
    let input = format!(
        "{}{}{}{}{}{}{}{}",
        request.inspection_id,
        request.workflow_execution_id.0,
        request.expected_audit_packet_hash,
        request.expected_chain_hash,
        request.reviewer,
        format!("{:?}", request.decision),
        request.scope,
        request.idempotency_key,
    );
    let hash = blake3::hash(input.as_bytes());
    AuditPacketReviewId(format!("wapr_{}", &hash.to_hex()[..16]))
}

/// Build a review record from a request.
/// Patch 1: Binds to exact audit_packet_hash and chain_hash.
pub fn build_audit_packet_review(request: AuditPacketReviewRequest) -> AuditPacketReview {
    let review_id = compute_review_id(&request);
    AuditPacketReview {
        review_id,
        inspection_id: request.inspection_id,
        workflow_execution_id: request.workflow_execution_id,
        audit_packet_hash: request.expected_audit_packet_hash,
        chain_hash: request.expected_chain_hash,
        reviewer: request.reviewer,
        decision: request.decision,
        scope: request.scope,
        caveats: request.caveats,
        idempotency_key: request.idempotency_key,
        reviewed_at: Utc::now(),
        // Patch 3: all false
        certifies_truth: false,
        approves_packet_truth: false,
        verifies_packet_contents: false,
        // Patch 8: all false
        certifies_external_truth: false,
        verifies_delivery: false,
        proves_recipient_acceptance: false,
        modifies_audit_packet: false,
        mutates_workflow_state: false,
        promotes_trust: false,
        executes_commands: false,
        sends_external_message: false,
        uploads_files: false,
        appends_trace: false,
        writes_memory: false,
        creates_execution_grant: false,
        execution_allowed_now: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_request() -> AuditPacketReviewRequest {
        AuditPacketReviewRequest {
            inspection_id: "weci_test".into(),
            workflow_execution_id: WorkflowExecutionId("wfx_test".into()),
            expected_audit_packet_hash: "pkt_hash_123".into(),
            expected_chain_hash: "chain_hash_456".into(),
            reviewer: "alice".into(),
            decision: AuditPacketReviewDecision::ReviewedWithCaveats,
            scope: "Full packet review".into(),
            caveats: vec!["Scope limited to Q1 data".into()],
            idempotency_key: "key1".into(),
        }
    }

    #[test]
    fn review_id_has_wapr_prefix() {
        let rec = build_audit_packet_review(test_request());
        assert!(rec.review_id.0.starts_with("wapr_"));
    }

    #[test]
    fn review_id_is_deterministic() {
        let id1 = compute_review_id(&test_request());
        let id2 = compute_review_id(&test_request());
        assert_eq!(id1, id2);
    }

    #[test]
    fn review_id_changes_on_different_inputs() {
        let mut req = test_request();
        let id1 = compute_review_id(&req);
        req.reviewer = "bob".into();
        let id2 = compute_review_id(&req);
        assert_ne!(id1, id2);
    }

    #[test]
    fn review_roundtrips_json() {
        let rec = build_audit_packet_review(test_request());
        let json = serde_json::to_string(&rec).unwrap();
        let back: AuditPacketReview = serde_json::from_str(&json).unwrap();
        assert_eq!(rec.review_id, back.review_id);
    }

    // Patch 1: binds to packet hash and chain hash
    #[test]
    fn review_binds_to_audit_packet_hash() {
        let rec = build_audit_packet_review(test_request());
        assert_eq!("pkt_hash_123", rec.audit_packet_hash);
    }

    #[test]
    fn review_binds_to_chain_hash() {
        let rec = build_audit_packet_review(test_request());
        assert_eq!("chain_hash_456", rec.chain_hash);
    }

    #[test]
    fn review_binds_to_exact_inspection_id() {
        let rec = build_audit_packet_review(test_request());
        assert_eq!("weci_test", rec.inspection_id);
    }

    #[test]
    fn review_does_not_modify_packet() {
        let rec = build_audit_packet_review(test_request());
        assert!(!rec.modifies_audit_packet);
    }

    // Patch 3: decision semantics
    #[test]
    fn review_decision_does_not_certify_truth() {
        let rec = build_audit_packet_review(test_request());
        assert!(!rec.certifies_truth);
        assert!(!rec.approves_packet_truth);
        assert!(!rec.verifies_packet_contents);
    }

    #[test]
    fn review_acknowledgment_is_not_approval() {
        let mut req = test_request();
        req.decision = AuditPacketReviewDecision::AcknowledgedForDistribution;
        let rec = build_audit_packet_review(req);
        assert!(!rec.approves_packet_truth);
    }

    #[test]
    fn review_noted_is_not_verification() {
        let mut req = test_request();
        req.decision = AuditPacketReviewDecision::NotedWithoutCertification;
        let rec = build_audit_packet_review(req);
        assert!(!rec.verifies_packet_contents);
        assert!(!rec.certifies_truth);
    }

    #[test]
    fn all_three_decisions_serialize() {
        let decisions = vec![
            AuditPacketReviewDecision::ReviewedWithCaveats,
            AuditPacketReviewDecision::AcknowledgedForDistribution,
            AuditPacketReviewDecision::NotedWithoutCertification,
        ];
        for d in &decisions {
            let json = serde_json::to_string(d).unwrap();
            let back: AuditPacketReviewDecision = serde_json::from_str(&json).unwrap();
            assert_eq!(*d, back);
        }
    }

    // Patch 8: no-authority flags
    #[test]
    fn review_has_no_authority_flags() {
        let rec = build_audit_packet_review(test_request());
        assert!(!rec.certifies_external_truth);
        assert!(!rec.verifies_delivery);
        assert!(!rec.proves_recipient_acceptance);
        assert!(!rec.modifies_audit_packet);
        assert!(!rec.mutates_workflow_state);
        assert!(!rec.promotes_trust);
        assert!(!rec.executes_commands);
        assert!(!rec.sends_external_message);
        assert!(!rec.uploads_files);
        assert!(!rec.appends_trace);
        assert!(!rec.writes_memory);
        assert!(!rec.creates_execution_grant);
        assert!(!rec.execution_allowed_now);
    }

    #[test]
    fn review_serialized_contains_no_certified_verified_truth() {
        let rec = build_audit_packet_review(test_request());
        let json = serde_json::to_string(&rec).unwrap().to_lowercase();
        assert!(json.contains("\"certifies_truth\":false"));
        assert!(json.contains("\"verifies_packet_contents\":false"));
        assert!(json.contains("\"certifies_external_truth\":false"));
    }

    #[test]
    fn review_preserves_caveats() {
        let rec = build_audit_packet_review(test_request());
        assert_eq!(1, rec.caveats.len());
        assert!(rec.caveats[0].contains("Q1"));
    }

    #[test]
    fn review_preserves_scope() {
        let rec = build_audit_packet_review(test_request());
        assert!(rec.scope.contains("Full packet"));
    }
}
