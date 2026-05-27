//! Session vocabulary — end reasons and thinking budget snapshots.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SessionEndReason {
    Natural,
    UserStopped,
    TokenBudgetExhausted,
    MaxStepsReached,
    Error,
    Cancelled,
}

/// Snapshot version of ThinkingBudget for trace events.
/// Runtime version lives in openwand-session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThinkingBudgetSnapshot {
    Off,
    Low,
    Medium,
    High,
    Max,
    Tokens(u32),
}
