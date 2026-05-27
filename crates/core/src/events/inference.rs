use serde::{Deserialize, Serialize};

use crate::session_vocab::ThinkingBudgetSnapshot;
use crate::snapshots::{PromptAssemblySnapshot, TokenUsageSnapshot};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InferenceEvent {
    Called {
        model: String,
        provider: String,
        prompt_hash: String,
        thinking_budget: Option<ThinkingBudgetSnapshot>,
        prompt_assembly: PromptAssemblySnapshot,
    },
    Completed {
        model: String,
        tokens: TokenUsageSnapshot,
        stop_reason: String,
        tool_call_count: u8,
    },
    Failed {
        model: String,
        error: String,
        retry_count: u8,
    },
}

impl InferenceEvent {
    pub fn event_kind(&self) -> &'static str {
        match self {
            Self::Called { .. } => "inference.called",
            Self::Completed { .. } => "inference.completed",
            Self::Failed { .. } => "inference.failed",
        }
    }
}
