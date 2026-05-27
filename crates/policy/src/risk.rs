//! Risk mapping functions.

use openwand_core::mode::{ConfirmationLevel, InteractionMode};
use openwand_core::risk::RiskLevelSnapshot;

/// Base confirmation level for a given risk level.
/// Policy recommendation before mode adjustment.
pub fn confirmation_for_risk(risk: &RiskLevelSnapshot) -> ConfirmationLevel {
    match risk {
        RiskLevelSnapshot::Low => ConfirmationLevel::Auto,
        RiskLevelSnapshot::Medium => ConfirmationLevel::Inform,
        RiskLevelSnapshot::High => ConfirmationLevel::Approve,
        RiskLevelSnapshot::Critical => ConfirmationLevel::Escalate,
    }
}

/// Apply the mode floor. InteractionMode can only raise confirmation, never lower it.
pub fn apply_mode_floor(
    mode: &InteractionMode,
    risk: &RiskLevelSnapshot,
    base: &ConfirmationLevel,
) -> ConfirmationLevel {
    match mode {
        InteractionMode::Direct => base.clone(),
        InteractionMode::AutoRouting => base.clone(),
        InteractionMode::Conversational => {
            // Conversational mode floors Low to Inform
            match risk {
                RiskLevelSnapshot::Low => ConfirmationLevel::Inform,
                _ => base.clone(),
            }
        }
        InteractionMode::Custom { .. } => base.clone(),
    }
}

/// Risk ordering for comparison. Higher = more dangerous.
pub fn risk_order(risk: &RiskLevelSnapshot) -> u8 {
    match risk {
        RiskLevelSnapshot::Low => 0,
        RiskLevelSnapshot::Medium => 1,
        RiskLevelSnapshot::High => 2,
        RiskLevelSnapshot::Critical => 3,
    }
}
