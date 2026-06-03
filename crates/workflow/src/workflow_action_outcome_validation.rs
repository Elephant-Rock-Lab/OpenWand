//! Validation for workflow action outcome records.

use crate::workflow_action_outcome::*;

/// Generate content-addressed outcome ID.
pub fn action_outcome_id_for(
    execution_id: &str,
    route_id: &str,
    pending_approval_id: &str,
    idempotency_key: &str,
) -> WorkflowActionOutcomeId {
    let preimage = format!("{}:{}:{}:{}", execution_id, route_id, pending_approval_id, idempotency_key);
    let hash = blake3::hash(preimage.as_bytes());
    WorkflowActionOutcomeId(format!("wao_{}", hash.to_hex()))
}

/// Validate that approval resolution has non-empty rationale.
pub fn validate_resolution_rationale(resolution: &WorkflowApprovalResolution) -> Result<(), String> {
    let rationale = match resolution {
        WorkflowApprovalResolution::Approve { rationale } => rationale,
        WorkflowApprovalResolution::Reject { rationale } => rationale,
    };
    if rationale.trim().is_empty() {
        return Err("Approval resolution requires non-empty rationale".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_id_is_deterministic() {
        let id1 = action_outcome_id_for("wfx_1", "war_1", "arid_1", "key");
        let id2 = action_outcome_id_for("wfx_1", "war_1", "arid_1", "key");
        assert_eq!(id1, id2);
    }

    #[test]
    fn outcome_id_differs_for_different_inputs() {
        let id1 = action_outcome_id_for("wfx_1", "war_1", "arid_1", "key1");
        let id2 = action_outcome_id_for("wfx_1", "war_1", "arid_1", "key2");
        assert_ne!(id1, id2);
    }

    #[test]
    fn validate_approve_rationale_ok() {
        let r = WorkflowApprovalResolution::Approve { rationale: "safe".into() };
        assert!(validate_resolution_rationale(&r).is_ok());
    }

    #[test]
    fn validate_approve_rationale_empty_fails() {
        let r = WorkflowApprovalResolution::Approve { rationale: "  ".into() };
        assert!(validate_resolution_rationale(&r).is_err());
    }

    #[test]
    fn validate_reject_rationale_ok() {
        let r = WorkflowApprovalResolution::Reject { rationale: "too risky".into() };
        assert!(validate_resolution_rationale(&r).is_ok());
    }

    #[test]
    fn validate_reject_rationale_empty_fails() {
        let r = WorkflowApprovalResolution::Reject { rationale: "".into() };
        assert!(validate_resolution_rationale(&r).is_err());
    }
}
