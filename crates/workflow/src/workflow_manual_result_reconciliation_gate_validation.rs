//! Validation for manual result reconciliation gate requests.

/// Content-addressed gate ID.
pub fn gate_id_for(
    workflow_execution_id: &str,
    manual_result_id: &str,
    manual_result_review_id: &str,
    readiness_id: &str,
    stage_id: &str,
    idempotency_key: &str,
) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"manual_reconciliation_gate:v1:");
    hasher.update(workflow_execution_id.as_bytes());
    hasher.update(b":");
    hasher.update(manual_result_id.as_bytes());
    hasher.update(b":");
    hasher.update(manual_result_review_id.as_bytes());
    hasher.update(b":");
    hasher.update(readiness_id.as_bytes());
    hasher.update(b":");
    hasher.update(stage_id.as_bytes());
    hasher.update(b":");
    hasher.update(idempotency_key.as_bytes());
    let hex = hasher.finalize().to_hex().to_string();
    format!("wmrrg_{}", &hex[..16])
}

/// Validate all 8 required hashes (Patch 3).
#[allow(clippy::too_many_arguments)]
pub fn validate_gate_hashes(
    expected_workflow_run_hash: &str,
    expected_reconciliation_readiness_hash: &str,
    expected_manual_result_review_hash: &str,
    expected_manual_result_hash: &str,
    expected_command_review_hash: &str,
    expected_command_composer_hash: &str,
    expected_command_descriptor_hash: &str,
    expected_loop_controller_hash: &str,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if expected_workflow_run_hash.is_empty() { errors.push("expected_workflow_run_hash is required".into()); }
    if expected_reconciliation_readiness_hash.is_empty() { errors.push("expected_reconciliation_readiness_hash is required".into()); }
    if expected_manual_result_review_hash.is_empty() { errors.push("expected_manual_result_review_hash is required".into()); }
    if expected_manual_result_hash.is_empty() { errors.push("expected_manual_result_hash is required".into()); }
    if expected_command_review_hash.is_empty() { errors.push("expected_command_review_hash is required".into()); }
    if expected_command_composer_hash.is_empty() { errors.push("expected_command_composer_hash is required".into()); }
    if expected_command_descriptor_hash.is_empty() { errors.push("expected_command_descriptor_hash is required".into()); }
    if expected_loop_controller_hash.is_empty() { errors.push("expected_loop_controller_hash is required".into()); }
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_id_is_deterministic() {
        let a = gate_id_for("wfx_1", "wmr_1", "wmrr_1", "wmrrr_1", "s1", "k1");
        let b = gate_id_for("wfx_1", "wmr_1", "wmrr_1", "wmrrr_1", "s1", "k1");
        assert_eq!(a, b);
        assert!(a.starts_with("wmrrg_"));
    }

    #[test]
    fn gate_id_differs_for_different_inputs() {
        let a = gate_id_for("wfx_1", "wmr_1", "wmrr_1", "wmrrr_1", "s1", "k1");
        let b = gate_id_for("wfx_2", "wmr_1", "wmrr_1", "wmrrr_1", "s1", "k1");
        assert_ne!(a, b);
    }

    #[test]
    fn validate_gate_hashes_rejects_empty_workflow_run_hash() {
        assert!(validate_gate_hashes("", "h", "h", "h", "h", "h", "h", "h").is_err());
    }

    #[test]
    fn validate_gate_hashes_rejects_empty_readiness_hash() {
        assert!(validate_gate_hashes("h", "", "h", "h", "h", "h", "h", "h").is_err());
    }

    #[test]
    fn validate_gate_hashes_passes_with_all_8() {
        assert!(validate_gate_hashes("h", "h", "h", "h", "h", "h", "h", "h").is_ok());
    }
}
