//! Reconciliation validation — content addressing and checks.

use blake3::Hasher;

use crate::workflow_reconciliation::{WorkflowReconciliationId, WorkflowRunRevisionId};

/// Compute content-addressed reconciliation ID from linkage inputs.
pub fn reconciliation_id_for(
    workflow_execution_id: &str,
    route_id: &str,
    outcome_id: &str,
    stage_id: &str,
    idempotency_key: &str,
) -> WorkflowReconciliationId {
    let mut hasher = Hasher::new();
    hasher.update(b"reconciliation:v1:");
    hasher.update(workflow_execution_id.as_bytes());
    hasher.update(b":");
    hasher.update(route_id.as_bytes());
    hasher.update(b":");
    hasher.update(outcome_id.as_bytes());
    hasher.update(b":");
    hasher.update(stage_id.as_bytes());
    hasher.update(b":");
    hasher.update(idempotency_key.as_bytes());
    let hex = hasher.finalize().to_hex().to_string();
    WorkflowReconciliationId(format!("wrc_{}", &hex[..16]))
}

/// Compute content-addressed run revision ID from revision inputs.
pub fn run_revision_id_for(
    workflow_execution_id: &str,
    reconciliation_id: &str,
    run_hash_after: &str,
) -> WorkflowRunRevisionId {
    let mut hasher = Hasher::new();
    hasher.update(b"run_revision:v1:");
    hasher.update(workflow_execution_id.as_bytes());
    hasher.update(b":");
    hasher.update(reconciliation_id.as_bytes());
    hasher.update(b":");
    hasher.update(run_hash_after.as_bytes());
    let hex = hasher.finalize().to_hex().to_string();
    WorkflowRunRevisionId(format!("wrr_{}", &hex[..16]))
}

/// Validate that the expected hashes are non-empty.
pub fn validate_reconciliation_hashes(
    expected_workflow_run_hash: &str,
    expected_route_hash: &str,
    expected_outcome_hash: &str,
) -> Result<(), String> {
    if expected_workflow_run_hash.is_empty() {
        return Err("expected_workflow_run_hash is required".into());
    }
    if expected_route_hash.is_empty() {
        return Err("expected_route_hash is required".into());
    }
    if expected_outcome_hash.is_empty() {
        return Err("expected_outcome_hash is required".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconciliation_id_is_deterministic() {
        let a = reconciliation_id_for("wfx_1", "war_1", "wao_1", "s1", "key1");
        let b = reconciliation_id_for("wfx_1", "war_1", "wao_1", "s1", "key1");
        assert_eq!(a, b);
    }

    #[test]
    fn reconciliation_id_differs_for_different_inputs() {
        let a = reconciliation_id_for("wfx_1", "war_1", "wao_1", "s1", "key1");
        let b = reconciliation_id_for("wfx_2", "war_1", "wao_1", "s1", "key1");
        assert_ne!(a, b);
    }

    #[test]
    fn run_revision_id_is_deterministic() {
        let a = run_revision_id_for("wfx_1", "wrc_1", "hash1");
        let b = run_revision_id_for("wfx_1", "wrc_1", "hash1");
        assert_eq!(a, b);
    }

    #[test]
    fn validate_reconciliation_hashes_rejects_empty() {
        assert!(validate_reconciliation_hashes("", "h", "h").is_err());
        assert!(validate_reconciliation_hashes("h", "", "h").is_err());
        assert!(validate_reconciliation_hashes("h", "h", "").is_err());
        assert!(validate_reconciliation_hashes("h", "h", "h").is_ok());
    }
}
