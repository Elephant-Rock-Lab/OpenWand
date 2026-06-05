//! Validation for evidence chain inspection requests.
//!
//! Patch 1: WorkflowEvidenceChainInspectionId is content-addressed.

use crate::workflow_evidence_chain_inspector::compute_inspection_id;

/// Content-addressed inspection ID. Format: `weci_<blake3_hex>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct WorkflowEvidenceChainInspectionId(pub String);

impl WorkflowEvidenceChainInspectionId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validate an inspection request.
pub fn validate_inspection_request(
    workflow_execution_id: &str,
) -> Result<(), String> {
    if workflow_execution_id.is_empty() {
        return Err("workflow_execution_id must not be empty".to_string());
    }
    if !workflow_execution_id.starts_with("wfx_") {
        return Err(format!("workflow_execution_id must start with wfx_: got {}", workflow_execution_id));
    }
    Ok(())
}

/// Build inspection ID from request parameters.
pub fn build_inspection_id(
    workflow_execution_id: &str,
    chain_hash: &str,
    link_ids: &[String],
    packet_mode: bool,
) -> WorkflowEvidenceChainInspectionId {
    WorkflowEvidenceChainInspectionId(
        compute_inspection_id(workflow_execution_id, chain_hash, link_ids, packet_mode),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_request_passes() {
        assert!(validate_inspection_request("wfx_abc123").is_ok());
    }

    #[test]
    fn empty_execution_id_fails() {
        assert!(validate_inspection_request("").is_err());
    }

    #[test]
    fn non_wfx_prefix_fails() {
        assert!(validate_inspection_request("tpl_123").is_err());
    }

    #[test]
    fn inspection_id_has_weci_prefix() {
        let id = build_inspection_id("wfx_1", "hash", &["wfx_1".to_string()], false);
        assert!(id.0.starts_with("weci_"));
    }

    #[test]
    fn inspection_id_is_deterministic() {
        let a = build_inspection_id("wfx_1", "hash", &["wfx_1".to_string()], false);
        let b = build_inspection_id("wfx_1", "hash", &["wfx_1".to_string()], false);
        assert_eq!(a, b);
    }

    #[test]
    fn inspection_id_roundtrips_json() {
        let id = build_inspection_id("wfx_1", "hash", &[], false);
        let json = serde_json::to_string(&id).unwrap();
        let back: WorkflowEvidenceChainInspectionId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }

    #[test]
    fn inspection_id_as_str() {
        let id = build_inspection_id("wfx_1", "hash", &[], false);
        assert!(id.as_str().starts_with("weci_"));
    }
}
