use openwand_core::mode::InteractionMode;
use serde::{Deserialize, Serialize};

/// Configuration for a single run_turn invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    pub max_steps: u64,
    pub mode: InteractionMode,
    pub working_directory: String,
    pub system_prompt: Option<String>,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            max_steps: 25,
            mode: InteractionMode::Conversational,
            working_directory: ".".into(),
            system_prompt: None,
        }
    }
}

/// Summary of a completed run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub stop_reason: RunStopReason,
    pub steps_completed: u64,
    pub tools_executed: u64,
    pub recoverable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunStopReason {
    Natural,
    MaxStepsReached,
    ToolBlocked,
    Cancelled,
}
