//! Gate decision types.

use openwand_core::mode::ConfirmationLevel;
use openwand_core::risk::RiskLevelSnapshot;
use openwand_core::GateId;
use serde::{Deserialize, Serialize};

use crate::error::PolicyError;
use crate::rule::PolicyRuleId;

/// Three-way decision: execute now, execute after confirmation, or blocked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GateDecision {
    /// Tool call may execute immediately.
    Allow,

    /// Tool call may execute after the user provides confirmation.
    RequireConfirmation { level: ConfirmationLevel },

    /// Tool call is blocked. It will not execute.
    Block { reason: String },
}

impl GateDecision {
    pub fn allows_execution(&self) -> bool {
        matches!(self, Self::Allow)
    }

    pub fn requires_confirmation(&self) -> bool {
        matches!(self, Self::RequireConfirmation { .. })
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Block { .. })
    }
}

/// Individual gate evaluation result. All findings collected before finalization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateFinding {
    pub rule_id: Option<PolicyRuleId>,
    pub result: GateFindingResult,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GateFindingResult {
    Allow,
    Require { risk: RiskLevelSnapshot },
    Block,
}

/// The full evaluation result. Session constructs `GateEvent::Evaluated` from this.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvaluation {
    /// Unique ID for this gate evaluation
    pub gate_id: GateId,

    /// The final decision
    pub decision: GateDecision,

    /// Assessed risk level
    pub risk_level: RiskLevelSnapshot,

    /// Required confirmation level (derived from risk × mode)
    pub confirmation_level: ConfirmationLevel,

    /// All gate findings collected during evaluation
    pub findings: Vec<GateFinding>,

    /// Which rules matched and contributed to the decision
    pub matched_rules: Vec<PolicyRuleId>,

    /// Machine-readable reason code for trace
    pub reason_code: String,

    /// Human-readable summary
    pub summary: String,

    /// Whether a rollback plan is required (for Escalate)
    pub rollback_required: bool,

    /// Suggested rollback plan (if available)
    pub rollback_plan: Option<String>,
}

impl PolicyEvaluation {
    /// Construct a fail-closed evaluation from a policy error.
    /// The tool call is blocked. The session continues.
    pub fn fail_closed(error: PolicyError) -> Self {
        Self {
            gate_id: GateId::new(),
            decision: GateDecision::Block {
                reason: "Policy evaluation failed; tool call blocked.".into(),
            },
            risk_level: RiskLevelSnapshot::Critical,
            confirmation_level: ConfirmationLevel::Escalate,
            findings: vec![],
            matched_rules: vec![],
            reason_code: "policy_evaluation_failed".into(),
            summary: format!("Policy evaluation failed: {}", error.safe_message()),
            rollback_required: false,
            rollback_plan: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_decision_helpers() {
        assert!(GateDecision::Allow.allows_execution());
        assert!(!GateDecision::Allow.requires_confirmation());
        assert!(!GateDecision::Allow.is_blocked());

        assert!(!GateDecision::RequireConfirmation {
            level: ConfirmationLevel::Approve
        }
        .allows_execution());
        assert!(GateDecision::RequireConfirmation {
            level: ConfirmationLevel::Approve
        }
        .requires_confirmation());
        assert!(!GateDecision::RequireConfirmation {
            level: ConfirmationLevel::Approve
        }
        .is_blocked());

        assert!(!GateDecision::Block {
            reason: "nope".into()
        }
        .allows_execution());
        assert!(!GateDecision::Block {
            reason: "nope".into()
        }
        .requires_confirmation());
        assert!(GateDecision::Block {
            reason: "nope".into()
        }
        .is_blocked());
    }

    #[test]
    fn policy_evaluation_fail_closed_blocks() {
        let eval = PolicyEvaluation::fail_closed(PolicyError::Internal("test".into()));

        assert!(eval.decision.is_blocked());
        assert_eq!(RiskLevelSnapshot::Critical, eval.risk_level);
        assert_eq!(ConfirmationLevel::Escalate, eval.confirmation_level);
        assert_eq!("policy_evaluation_failed", eval.reason_code);
        assert!(eval.summary.contains("Internal policy error"));
        assert!(eval.findings.is_empty());
        assert!(eval.matched_rules.is_empty());
    }
}
