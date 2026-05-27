use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowEvent {
    StateChanged {
        from_state: String,
        to_state: String,
        mod_id: Option<String>,
    },
    GatePassed {
        gate_name: String,
        mod_id: String,
    },
    GateFailed {
        gate_name: String,
        mod_id: String,
        reason: String,
    },
    ActionExecuted {
        action_name: String,
        mod_id: String,
        success: bool,
        duration_ms: u64,
    },
    ModStarted {
        mod_id: String,
        mod_name: String,
    },
    ModCompleted {
        mod_id: String,
        mod_name: String,
        outcome: String,
    },
}

impl WorkflowEvent {
    pub fn event_kind(&self) -> &'static str {
        match self {
            Self::StateChanged { .. } => "workflow.state_changed",
            Self::GatePassed { .. } => "workflow.gate_passed",
            Self::GateFailed { .. } => "workflow.gate_failed",
            Self::ActionExecuted { .. } => "workflow.action_executed",
            Self::ModStarted { .. } => "workflow.mod_started",
            Self::ModCompleted { .. } => "workflow.mod_completed",
        }
    }
}
