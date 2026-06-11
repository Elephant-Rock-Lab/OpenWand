use openwand_core::mode::InteractionMode;
use openwand_memory::prompt_assembly::MemoryPromptAssemblyInputs;
use openwand_policy::OutputGuardConfig;
use serde::{Deserialize, Serialize};

/// Typed capability context block from skills/goals registries.
/// Carried through RunConfig for deterministic prompt assembly.
/// Text-only, no executable fields.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CapabilityContextBlock {
    /// Manifest state at assembly time.
    pub skills_manifest_state: String,
    pub goals_manifest_state: String,
    /// IDs of skills included in the block.
    pub included_skill_ids: Vec<String>,
    /// IDs of goals included in the block.
    pub included_goal_ids: Vec<String>,
    /// IDs excluded due to readiness gaps.
    pub excluded_item_ids: Vec<String>,
    /// The assembled prompt text (bounded, sanitized).
    pub text: String,
}

/// Configuration for a single run_turn invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    pub max_steps: u64,
    pub mode: InteractionMode,
    pub working_directory: String,
    pub system_prompt: Option<String>,
    /// LLM target — provider, model, base_url, api_key.
    /// Session runner passes this to the LLM client.
    pub llm_target: Option<openwand_llm::LlmTarget>,
    /// Pre-assembled memory prompt inputs from 02k pipeline.
    /// If present, used instead of raw memory search.
    /// The caller (session/app) is responsible for producing the
    /// RepoConsistencyReport and assembling these inputs.
    #[serde(skip)]
    pub memory_prompt_inputs: Option<MemoryPromptAssemblyInputs>,
    /// Post-inference output record guard config.
    ///
    /// When enabled, the durable assistant message is screened for
    /// forbidden action patterns after generation. Streaming remains live.
    ///
    /// This is NOT pre-disclosure safety enforcement.
    /// It is post-hoc durable-record correction.
    /// This does NOT guarantee the user never saw the raw text.
    #[serde(skip)]
    pub output_guard: Option<OutputGuardConfig>,
    /// Pre-assembled capability context from skills/goals registries.
    /// Gated by readiness: only ReadyForContext entries are included.
    /// This is contextual information only — never parsed as executable instructions.
    #[serde(skip)]
    pub capability_context: Option<CapabilityContextBlock>,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            max_steps: 25,
            mode: InteractionMode::Conversational,
            working_directory: ".".into(),
            system_prompt: None,
            llm_target: None,
            memory_prompt_inputs: None,
            output_guard: None,
            capability_context: None,
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
    /// Runner paused waiting for user approval of a gated tool.
    AwaitingApproval,
    /// Tool was denied by user; not executed.
    ToolDenied,
}
