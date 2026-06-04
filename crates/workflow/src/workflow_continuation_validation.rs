//! Continuation validation — content addressing and checks.

use blake3::Hasher;

use crate::workflow_continuation::{WorkflowContinuationReadinessId, WorkflowNextActionProposalId};

/// Compute content-addressed readiness ID from request inputs.
pub fn continuation_readiness_id_for(
    workflow_execution_id: &str,
    run_revision_id: &str,
    idempotency_key: &str,
) -> WorkflowContinuationReadinessId {
    let mut hasher = Hasher::new();
    hasher.update(b"continuation_readiness:v1:");
    hasher.update(workflow_execution_id.as_bytes());
    hasher.update(b":");
    hasher.update(run_revision_id.as_bytes());
    hasher.update(b":");
    hasher.update(idempotency_key.as_bytes());
    let hex = hasher.finalize().to_hex().to_string();
    WorkflowContinuationReadinessId(format!("wcr_{}", &hex[..16]))
}

/// Compute content-addressed proposal ID from proposal inputs.
pub fn next_action_proposal_id_for(
    workflow_execution_id: &str,
    run_revision_id: &str,
    stage_id: &str,
) -> WorkflowNextActionProposalId {
    let mut hasher = Hasher::new();
    hasher.update(b"next_action_proposal:v1:");
    hasher.update(workflow_execution_id.as_bytes());
    hasher.update(b":");
    hasher.update(run_revision_id.as_bytes());
    hasher.update(b":");
    hasher.update(stage_id.as_bytes());
    let hex = hasher.finalize().to_hex().to_string();
    WorkflowNextActionProposalId(format!("wnap_{}", &hex[..16]))
}

/// Compute proposal hash from candidate identity.
pub fn proposal_hash_for(
    workflow_execution_id: &str,
    run_revision_id: &str,
    stage_id: &str,
    action_request_id: Option<&str>,
) -> String {
    let mut hasher = Hasher::new();
    hasher.update(b"proposal_hash:v1:");
    hasher.update(workflow_execution_id.as_bytes());
    hasher.update(b":");
    hasher.update(run_revision_id.as_bytes());
    hasher.update(b":");
    hasher.update(stage_id.as_bytes());
    hasher.update(b":");
    if let Some(ar) = action_request_id {
        hasher.update(ar.as_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

/// Validate continuation request has required fields.
pub fn validate_continuation_request(expected_hash: &str) -> Result<(), String> {
    if expected_hash.is_empty() {
        return Err("expected_run_revision_hash is required".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readiness_id_is_deterministic() {
        let a = continuation_readiness_id_for("wfx_1", "wrr_1", "key1");
        let b = continuation_readiness_id_for("wfx_1", "wrr_1", "key1");
        assert_eq!(a, b);
    }

    #[test]
    fn readiness_id_differs_for_different_inputs() {
        let a = continuation_readiness_id_for("wfx_1", "wrr_1", "key1");
        let b = continuation_readiness_id_for("wfx_2", "wrr_1", "key1");
        assert_ne!(a, b);
    }

    #[test]
    fn proposal_id_is_deterministic() {
        let a = next_action_proposal_id_for("wfx_1", "wrr_1", "s1");
        let b = next_action_proposal_id_for("wfx_1", "wrr_1", "s1");
        assert_eq!(a, b);
    }

    #[test]
    fn proposal_hash_changes_when_candidate_changes() {
        let a = proposal_hash_for("wfx_1", "wrr_1", "s1", Some("ar_1"));
        let b = proposal_hash_for("wfx_1", "wrr_1", "s1", Some("ar_2"));
        assert_ne!(a, b);
    }

    #[test]
    fn validate_continuation_request_rejects_empty_hash() {
        assert!(validate_continuation_request("").is_err());
        assert!(validate_continuation_request("h").is_ok());
    }
}
