//! Validation for manual result reconciliation readiness requests.

/// Content-addressed readiness ID.
pub fn reconciliation_readiness_id_for(
    workflow_execution_id: &str,
    manual_result_id: &str,
    manual_result_review_id: &str,
    idempotency_key: &str,
) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"reconciliation_readiness:v1:");
    hasher.update(workflow_execution_id.as_bytes());
    hasher.update(b":");
    hasher.update(manual_result_id.as_bytes());
    hasher.update(b":");
    hasher.update(manual_result_review_id.as_bytes());
    hasher.update(b":");
    hasher.update(idempotency_key.as_bytes());
    let hex = hasher.finalize().to_hex().to_string();
    format!("wmrrr_{}", &hex[..16])
}

/// Validate request field requirements.
pub fn validate_reconciliation_readiness_request(
    evaluator: &str,
    expected_manual_result_review_hash: &str,
    expected_manual_result_hash: &str,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if evaluator.is_empty() { errors.push("evaluator is required".into()); }
    if expected_manual_result_review_hash.is_empty() { errors.push("expected_manual_result_review_hash is required".into()); }
    if expected_manual_result_hash.is_empty() { errors.push("expected_manual_result_hash is required".into()); }
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_deterministic() {
        let id1 = reconciliation_readiness_id_for("wfx_1", "wmr_1", "wmrr_1", "k1");
        let id2 = reconciliation_readiness_id_for("wfx_1", "wmr_1", "wmrr_1", "k1");
        assert_eq!(id1, id2);
        assert!(id1.starts_with("wmrrr_"));
    }

    #[test]
    fn id_differs_for_different_inputs() {
        let id1 = reconciliation_readiness_id_for("wfx_1", "wmr_1", "wmrr_1", "k1");
        let id2 = reconciliation_readiness_id_for("wfx_1", "wmr_1", "wmrr_1", "k2");
        assert_ne!(id1, id2);
    }

    #[test]
    fn validation_blocks_empty_evaluator() {
        assert!(validate_reconciliation_readiness_request("", "h", "h").is_err());
    }

    #[test]
    fn validation_blocks_empty_review_hash() {
        assert!(validate_reconciliation_readiness_request("e", "", "h").is_err());
    }

    #[test]
    fn validation_passes_with_required_fields() {
        assert!(validate_reconciliation_readiness_request("e", "h", "h").is_ok());
    }
}
