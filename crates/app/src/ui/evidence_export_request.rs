//! Evidence export request DTO for desktop-initiated evidence export.
//!
//! This module is a pure data type — it imports NO backend authority types.
//! It represents the desktop's INTENT to export an evidence audit packet
//! for a selected workflow run, not the authority to assemble or write it.
//!
//! Wave 88C patches applied:
//! - Separate store_root (evidence source) from export_root (allowed destination)
//! - output_path validated against explicit export_root, not store_root
//! - record_count is Option<usize> (parsed from exported packet, not guessed)
//! - certifies_external_truth and verifies_artifacts match packet semantics
//! - Path traversal/symlink escape is handled in the service layer

use serde::{Deserialize, Serialize};

/// Desktop-initiated evidence export request.
///
/// The desktop constructs this DTO with a workflow execution ID and
/// a desired output path. The UiSessionService validates the output path
/// against an allowed export root, delegates to the existing exporter,
/// and returns the honest export result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceExportRequest {
    /// The workflow execution ID to export evidence for.
    /// Must be non-empty.
    pub workflow_execution_id: String,

    /// The desired output file path for the exported JSON.
    /// Must be non-empty. The service validates this is within
    /// the allowed export root.
    pub output_path: String,

    /// Who requested this export (always "desktop" from the UI layer).
    pub requested_by: String,

    /// Deterministic idempotency key for this request.
    pub idempotency_key: String,
}

/// State of an evidence export request lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EvidenceExportState {
    /// No request made.
    Idle,

    /// Request submitted, awaiting export.
    Pending,

    /// Export completed successfully.
    Exported {
        /// Path where the audit packet JSON was written.
        artifact_path: String,
        /// Number of records in the exported packet, if known.
        /// Parsed from the exported JSON, not guessed.
        record_count: Option<usize>,
        /// SHA-256 checksum of the exported artifact file.
        packet_hash: String,
        /// Whether the packet certifies external truth.
        /// Always false unless the packet itself says otherwise.
        certifies_external_truth: bool,
        /// Whether the packet verifies artifacts.
        /// Always false unless the packet itself says otherwise.
        verifies_artifacts: bool,
    },

    /// Export failed due to an error.
    Failed {
        error: String,
    },

    /// Export unavailable (no workflow run selected, no evidence found).
    Unavailable {
        reason: String,
    },
}

impl Default for EvidenceExportState {
    fn default() -> Self {
        Self::Idle
    }
}

impl EvidenceExportRequest {
    /// Validate the DTO before service delegation.
    pub fn validate(&self) -> Result<(), String> {
        if self.workflow_execution_id.trim().is_empty() {
            return Err("workflow_execution_id must be non-empty".into());
        }
        if self.output_path.trim().is_empty() {
            return Err("output_path must be non-empty".into());
        }
        if self.idempotency_key.trim().is_empty() {
            return Err("idempotency_key must be non-empty".into());
        }
        Ok(())
    }
}

impl EvidenceExportState {
    /// Human-readable status label for UI display.
    pub fn status_label(&self) -> &'static str {
        match self {
            Self::Idle => "No export requested",
            Self::Pending => "Exporting...",
            Self::Exported { .. } => "Exported",
            Self::Failed { .. } => "Export failed",
            Self::Unavailable { .. } => "Export unavailable",
        }
    }

    /// Whether this state is terminal.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Exported { .. } | Self::Failed { .. } | Self::Unavailable { .. }
        )
    }

    /// Whether this state is pending.
    pub fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_empty_workflow_execution_id() {
        let req = EvidenceExportRequest {
            workflow_execution_id: "".into(),
            output_path: "/tmp/out.json".into(),
            requested_by: "desktop".into(),
            idempotency_key: "k".into(),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn validate_rejects_empty_output_path() {
        let req = EvidenceExportRequest {
            workflow_execution_id: "wfx_1".into(),
            output_path: "".into(),
            requested_by: "desktop".into(),
            idempotency_key: "k".into(),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn validate_accepts_valid_request() {
        let req = EvidenceExportRequest {
            workflow_execution_id: "wfx_1".into(),
            output_path: "exports/packet.json".into(),
            requested_by: "desktop".into(),
            idempotency_key: "k".into(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn state_lifecycle_is_honest() {
        assert!(!EvidenceExportState::Idle.is_terminal());
        assert!(!EvidenceExportState::Idle.is_pending());

        assert!(EvidenceExportState::Pending.is_pending());
        assert!(!EvidenceExportState::Pending.is_terminal());

        let exported = EvidenceExportState::Exported {
            artifact_path: "/tmp/out.json".into(),
            record_count: Some(5),
            packet_hash: "abc123".into(),
            certifies_external_truth: false,
            verifies_artifacts: false,
        };
        assert!(exported.is_terminal());
        assert!(!exported.is_pending());

        let failed = EvidenceExportState::Failed { error: "test".into() };
        assert!(failed.is_terminal());

        let unavailable = EvidenceExportState::Unavailable { reason: "none".into() };
        assert!(unavailable.is_terminal());
    }
}
