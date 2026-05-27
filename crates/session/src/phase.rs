use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phase {
    RunStart,
    StepStart,
    BeforeInference,
    Inference,
    AfterInference,
    ToolGate,
    BeforeToolExecute,
    AfterToolExecute,
    StepEnd,
    RunEnd,
}

impl Phase {
    pub fn name(&self) -> &'static str {
        match self {
            Phase::RunStart => "run_start",
            Phase::StepStart => "step_start",
            Phase::BeforeInference => "before_inference",
            Phase::Inference => "inference",
            Phase::AfterInference => "after_inference",
            Phase::ToolGate => "tool_gate",
            Phase::BeforeToolExecute => "before_tool_execute",
            Phase::AfterToolExecute => "after_tool_execute",
            Phase::StepEnd => "step_end",
            Phase::RunEnd => "run_end",
        }
    }
}
