use serde::{Deserialize, Serialize};

use crate::ids::SessionId;
use crate::mode::InteractionMode;
use crate::session_vocab::SessionEndReason;
use crate::snapshots::TokenUsageSnapshot;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionEvent {
    Started {
        session_id: SessionId,
        mode: InteractionMode,
    },
    Ended {
        session_id: SessionId,
        reason: SessionEndReason,
        total_steps: u64,
        total_tokens: TokenUsageSnapshot,
    },
    StepStarted {
        step: u64,
    },
    StepCompleted {
        step: u64,
        stop_reason: String,
    },
    UserMessageInjected {
        text: String,
    },
}

impl SessionEvent {
    pub fn event_kind(&self) -> &'static str {
        match self {
            Self::Started { .. } => "session.started",
            Self::Ended { .. } => "session.ended",
            Self::StepStarted { .. } => "session.step_started",
            Self::StepCompleted { .. } => "session.step_completed",
            Self::UserMessageInjected { .. } => "session.user_message_injected",
        }
    }
}
