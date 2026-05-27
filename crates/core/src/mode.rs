//! Interaction modes and confirmation levels.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InteractionMode {
    Direct,
    Conversational,
    AutoRouting,
    Custom { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConfirmationLevel {
    /// Auto-accept after deterministic gates pass
    Auto,
    /// Show diff + explanation, accept on ack
    Inform,
    /// Require explicit approval
    Approve,
    /// Approval + rollback plan + optional second review
    Escalate,
}
