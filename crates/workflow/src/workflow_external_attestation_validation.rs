//! Validation for external attestation requests.

use crate::workflow_external_attestation::*;

/// Validate an attestation request structurally.
pub fn validate_attestation_request(request: &ExternalAttestationRequest) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if request.workflow_execution_id.0.is_empty() {
        errors.push("workflow_execution_id must not be empty".to_string());
    }
    if !request.workflow_execution_id.0.starts_with("wfx_") {
        errors.push(format!("workflow_execution_id must start with wfx_: got {}", request.workflow_execution_id.0));
    }
    if request.target_id.is_empty() {
        errors.push("target_id must not be empty".to_string());
    }
    if request.claim.is_empty() {
        errors.push("claim must not be empty".to_string());
    }
    if request.source_name.is_empty() {
        errors.push("source_name must not be empty".to_string());
    }
    if request.idempotency_key.is_empty() {
        errors.push("idempotency_key must not be empty".to_string());
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_run::WorkflowExecutionId;

    fn valid_request() -> ExternalAttestationRequest {
        ExternalAttestationRequest {
            workflow_execution_id: WorkflowExecutionId("wfx_test".into()),
            target_kind: ExternalAttestationTargetKind::ManualResult,
            target_id: "wmr_test".into(),
            expected_target_hash: None,
            kind: ExternalAttestationKind::ThirdPartySignoff,
            source_name: "Alice".into(),
            source_role: "reviewer".into(),
            source_system_identifier: None,
            claim: "Reviewed".into(),
            references: vec![],
            reported_signature: None,
            attested_at: chrono::Utc::now(),
            idempotency_key: "key1".into(),
        }
    }

    #[test]
    fn valid_request_passes() {
        assert!(validate_attestation_request(&valid_request()).is_ok());
    }

    #[test]
    fn blocks_empty_execution_id() {
        let mut req = valid_request();
        req.workflow_execution_id = WorkflowExecutionId("".into());
        let result = validate_attestation_request(&req);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("workflow_execution_id")));
    }

    #[test]
    fn blocks_non_wfx_prefix() {
        let mut req = valid_request();
        req.workflow_execution_id = WorkflowExecutionId("tpl_123".into());
        let result = validate_attestation_request(&req);
        assert!(result.is_err());
    }

    #[test]
    fn blocks_empty_attestation_target_id() {
        let mut req = valid_request();
        req.target_id = "".into();
        let result = validate_attestation_request(&req);
        assert!(result.is_err());
        assert!(result.unwrap_err().iter().any(|e| e.contains("target_id")));
    }

    #[test]
    fn blocks_empty_claim() {
        let mut req = valid_request();
        req.claim = "".into();
        assert!(validate_attestation_request(&req).is_err());
    }

    #[test]
    fn blocks_empty_source_name() {
        let mut req = valid_request();
        req.source_name = "".into();
        assert!(validate_attestation_request(&req).is_err());
    }

    #[test]
    fn blocks_empty_idempotency_key() {
        let mut req = valid_request();
        req.idempotency_key = "".into();
        assert!(validate_attestation_request(&req).is_err());
    }

    #[test]
    fn multiple_errors_reported() {
        let mut req = valid_request();
        req.target_id = "".into();
        req.claim = "".into();
        let result = validate_attestation_request(&req);
        assert_eq!(2, result.unwrap_err().len());
    }
}
