//! Approval resolution request DTO for desktop-initiated approval decisions.
//!
//! This module is a pure data type — it imports NO backend authority types.
//! It represents the desktop's INTENT to resolve a pending approval,
//! not the authority to do so. The UiSessionService boundary converts
//! this DTO into the existing approval governance path.
//!
//! Wave 88B patches applied:
//! - approval_request_id is mandatory and non-empty
//! - tool_name is display-only (displayed_tool_name), NOT authoritative
//! - tool-name/args-hash enforcement remains in backend approval path
//! - rationale is Optional (not required for approval)
//! - idempotency_key is request metadata only

use serde::{Deserialize, Serialize};

/// Desktop-initiated approval resolution request.
///
/// The desktop constructs this DTO with:
/// - An explicit, non-empty approval_request_id (ARID)
/// - A decision (Approve or Reject)
/// - Display-only context (tool name from the pending approval state)
///
/// The UiSessionService converts this into the existing approval governance path.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApprovalResolutionRequest {
    /// The explicit approval_request_id (ARID) to resolve.
    /// Must be non-empty. No implicit "resolve whatever is pending."
    pub approval_request_id: String,

    /// Tool name for display/binding context only.
    /// The UI is NOT the source of truth — tool-name and args-hash
    /// enforcement remains in the existing backend approval path.
    pub displayed_tool_name: Option<String>,

    /// The user's decision: Approve or Reject.
    pub decision: ApprovalDecisionDto,

    /// Optional rationale. Required for meaningful rejections;
    /// may be empty for approvals.
    pub rationale: Option<String>,

    /// Who resolved this (always "desktop" from the UI layer).
    pub resolved_by: String,

    /// Deterministic idempotency key for this request.
    /// Request metadata only — backend idempotency depends on the
    /// existing approval path's behavior.
    pub idempotency_key: String,
}

/// The user's approval decision.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApprovalDecisionDto {
    Approve,
    Reject,
}

/// State of an approval resolution request lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ApprovalResolutionState {
    /// No request made.
    Idle,

    /// Request submitted, awaiting governance path.
    Pending,

    /// Governance path resolved the approval.
    Resolved {
        decision: ApprovalDecisionDto,
        approval_request_id: String,
        tool_name: Option<String>,
        tool_status: Option<String>,
        source: String,
    },

    /// Governance path returned an error.
    Failed {
        error: String,
    },

    /// The approval is no longer pending (stale, already resolved, or runner gone).
    Stale {
        reason: String,
    },
}

impl Default for ApprovalResolutionState {
    fn default() -> Self {
        Self::Idle
    }
}

impl ApprovalResolutionRequest {
    /// Validate the DTO before service delegation.
    /// Returns Err if approval_request_id is empty.
    pub fn validate(&self) -> Result<(), String> {
        if self.approval_request_id.trim().is_empty() {
            return Err("approval_request_id must be non-empty".into());
        }
        if self.idempotency_key.trim().is_empty() {
            return Err("idempotency_key must be non-empty".into());
        }
        Ok(())
    }
}

impl ApprovalResolutionState {
    /// Human-readable status label for UI display.
    pub fn status_label(&self) -> &'static str {
        match self {
            Self::Idle => "No approval pending",
            Self::Pending => "Resolving...",
            Self::Resolved { .. } => "Resolved",
            Self::Failed { .. } => "Failed",
            Self::Stale { .. } => "Stale",
        }
    }

    /// Whether this state is terminal (no further transitions expected).
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Resolved { .. } | Self::Failed { .. } | Self::Stale { .. })
    }

    /// Whether this state is pending (request in flight).
    pub fn is_pending(&self) -> bool {
        matches!(self, Self::Pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_empty_arid() {
        let req = ApprovalResolutionRequest {
            approval_request_id: "".into(),
            displayed_tool_name: None,
            decision: ApprovalDecisionDto::Approve,
            rationale: None,
            resolved_by: "desktop".into(),
            idempotency_key: "k".into(),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn validate_rejects_whitespace_only_arid() {
        let req = ApprovalResolutionRequest {
            approval_request_id: "   ".into(),
            displayed_tool_name: None,
            decision: ApprovalDecisionDto::Approve,
            rationale: None,
            resolved_by: "desktop".into(),
            idempotency_key: "k".into(),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn validate_rejects_empty_idempotency_key() {
        let req = ApprovalResolutionRequest {
            approval_request_id: "arid_1".into(),
            displayed_tool_name: None,
            decision: ApprovalDecisionDto::Approve,
            rationale: None,
            resolved_by: "desktop".into(),
            idempotency_key: "".into(),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn validate_accepts_valid_request() {
        let req = ApprovalResolutionRequest {
            approval_request_id: "arid_1".into(),
            displayed_tool_name: Some("local__file_write".into()),
            decision: ApprovalDecisionDto::Approve,
            rationale: None,
            resolved_by: "desktop".into(),
            idempotency_key: "k1".into(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn validate_accepts_reject_with_rationale() {
        let req = ApprovalResolutionRequest {
            approval_request_id: "arid_2".into(),
            displayed_tool_name: Some("local__file_write".into()),
            decision: ApprovalDecisionDto::Reject,
            rationale: Some("Too risky".into()),
            resolved_by: "desktop".into(),
            idempotency_key: "k2".into(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn state_lifecycle_is_honest() {
        assert!(!ApprovalResolutionState::Idle.is_terminal());
        assert!(!ApprovalResolutionState::Idle.is_pending());

        assert!(ApprovalResolutionState::Pending.is_pending());
        assert!(!ApprovalResolutionState::Pending.is_terminal());

        let resolved = ApprovalResolutionState::Resolved {
            decision: ApprovalDecisionDto::Approve,
            approval_request_id: "arid".into(),
            tool_name: Some("local__file_write".into()),
            tool_status: Some("completed".into()),
            source: "recovered".into(),
        };
        assert!(resolved.is_terminal());
        assert!(!resolved.is_pending());

        let failed = ApprovalResolutionState::Failed { error: "test".into() };
        assert!(failed.is_terminal());

        let stale = ApprovalResolutionState::Stale { reason: "gone".into() };
        assert!(stale.is_terminal());
    }

    #[test]
    fn status_labels_are_descriptive() {
        assert_eq!(ApprovalResolutionState::Idle.status_label(), "No approval pending");
        assert_eq!(ApprovalResolutionState::Pending.status_label(), "Resolving...");
        assert_eq!(
            ApprovalResolutionState::Resolved {
                decision: ApprovalDecisionDto::Approve,
                approval_request_id: "a".into(),
                tool_name: None,
                tool_status: None,
                source: "live".into(),
            }.status_label(),
            "Resolved"
        );
    }
}
