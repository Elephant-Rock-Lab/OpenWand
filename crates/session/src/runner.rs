use crate::adapters::llm::{message_to_llm_message, tool_def_to_llm_tool};
use crate::adapters::policy::session_tool_call_to_policy;
use crate::adapters::tools::build_tool_context;
use crate::agent_event::AgentEvent;
use crate::config::{RunConfig, RunStopReason, RunSummary};
use crate::loro_state::LoroSessionState;
use crate::message::Message;
use crate::mutation::MutationHelper;
use crate::phase::Phase;
use crate::projector::LoroProjector;
use crate::tool::ToolCall;
use crate::SessionError;
use openwand_core::events::{InferenceEvent, OpenWandTraceEvent, SessionEvent};
use openwand_store::StoredEvent;
use openwand_core::mode::InteractionMode;
use openwand_core::SessionId;
use openwand_llm::{LlmClient, LlmDelta, LlmRequest, LlmTarget};
use openwand_memory::{MemoryQuery, MemoryReadStore};
use openwand_policy::{GateDecision, PolicyEngine};
use openwand_tools::executor::ToolExecutor;
use openwand_trace::{Actor, TraceStore, TraceStreamId, TraceStreamScope};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tokio_util::sync::CancellationToken;

struct InferenceOutput {
    text: String,
    tool_calls: Vec<ToolCall>,
}

struct GatedTools {
    allowed: Vec<ToolCall>,
    blocked: Vec<ToolCall>,
    blocked_any: bool,
}

pub struct SessionRunner {
    pub session_id: SessionId,
    stream_id: TraceStreamId,

    #[allow(dead_code)] // Cloned for MutationHelper
    trace: Arc<dyn TraceStore<StoredEvent>>,
    llm: Arc<dyn LlmClient>,
    tools: Arc<dyn ToolExecutor>,
    policy: Arc<dyn PolicyEngine>,
    memory: Arc<dyn MemoryReadStore>,

    loro_state: LoroSessionState,
    mutation: MutationHelper,

    agent_event_tx: broadcast::Sender<AgentEvent>,

    run_lock: Mutex<()>,
    cancellation: CancellationToken,

    working_directory: String,
}

impl SessionRunner {
    pub fn new(
        session_id: SessionId,
        trace: Arc<dyn TraceStore<StoredEvent>>,
        llm: Arc<dyn LlmClient>,
        tools: Arc<dyn ToolExecutor>,
        policy: Arc<dyn PolicyEngine>,
        memory: Arc<dyn MemoryReadStore>,
        working_directory: String,
    ) -> Self {
        let doc = loro::LoroDoc::new();
        let loro_state = LoroSessionState::new(&doc);
        let projector = LoroProjector::new(LoroSessionState::new(&doc));
        let (agent_event_tx, _) = broadcast::channel(256);
        let stream_id = TraceStreamId {
            scope: TraceStreamScope::Session,
            id: session_id.to_string(),
        };

        let mutation = MutationHelper::new(
            Arc::clone(&trace),
            projector,
            LoroSessionState::new(&doc),
            agent_event_tx.clone(),
        );

        Self {
            session_id,
            stream_id,
            trace,
            llm,
            tools,
            policy,
            memory,
            loro_state,
            mutation,
            agent_event_tx,
            run_lock: Mutex::new(()),
            cancellation: CancellationToken::new(),
            working_directory,
        }
    }

    /// Get session messages from Loro projection.
    pub fn messages(&self) -> Result<Vec<Message>, String> {
        self.loro_state.messages()
    }

    /// Get the LoroSessionState.
    pub fn loro_state(&self) -> &LoroSessionState {
        &self.loro_state
    }

    /// Get a receiver for AgentEvents.
    pub fn subscribe(&self) -> broadcast::Receiver<AgentEvent> {
        self.agent_event_tx.subscribe()
    }

    /// Run one user turn through the 10-phase loop.
    pub async fn run_turn(
        &self,
        user_text: String,
        config: RunConfig,
    ) -> Result<RunSummary, SessionError> {
        // Single-writer guard
        let _guard = self
            .run_lock
            .try_lock()
            .map_err(|_| SessionError::RunAlreadyActive)?;

        let mut steps_completed = 0u64;
        let mut tools_executed = 0u64;
        #[allow(unused_assignments)]
        let mut stop_reason = RunStopReason::Natural;

        // Phase: RunStart
        self.emit_phase(Phase::RunStart, 0).await;
        self.record_user_message(&user_text).await?;

        let mut step = 0u64;
        loop {
            if self.cancellation.is_cancelled() {
                stop_reason = RunStopReason::Cancelled;
                break;
            }
            if step >= config.max_steps {
                stop_reason = RunStopReason::MaxStepsReached;
                break;
            }

            self.emit_phase(Phase::StepStart, step).await;

            // BeforeInference — assemble request
            self.emit_phase(Phase::BeforeInference, step).await;
            let llm_request = self.assemble_llm_request(&config).await?;

            // Inference
            self.emit_phase(Phase::Inference, step).await;
            let inference_output = self.run_inference(llm_request).await?;

            // AfterInference
            self.emit_phase(Phase::AfterInference, step).await;
            if !inference_output.text.is_empty() {
                self.record_assistant_message(&inference_output.text).await?;
            }

            if inference_output.tool_calls.is_empty() {
                stop_reason = RunStopReason::Natural;
                break;
            }

            // ToolGate
            self.emit_phase(Phase::ToolGate, step).await;
            let gated = self.gate_tool_calls(&inference_output.tool_calls).await?;

            if gated.blocked_any {
                self.record_blocked_tools(&gated.blocked).await?;
                stop_reason = RunStopReason::ToolBlocked;
                break;
            }

            if gated.allowed.is_empty() {
                stop_reason = RunStopReason::Natural;
                break;
            }

            // BeforeToolExecute
            self.emit_phase(Phase::BeforeToolExecute, step).await;
            let results = self.execute_tools(&gated.allowed, &config).await?;

            // AfterToolExecute
            self.emit_phase(Phase::AfterToolExecute, step).await;
            self.record_tool_results(&results).await?;
            tools_executed += results.len() as u64;

            self.emit_phase(Phase::StepEnd, step).await;
            step += 1;
            steps_completed = step;
        }

        self.emit_phase(Phase::RunEnd, step).await;

        Ok(RunSummary {
            stop_reason,
            steps_completed,
            tools_executed,
            recoverable: true,
        })
    }

    // ---- Internal helpers ----

    async fn emit_phase(&self, phase: Phase, step: u64) {
        let _ = self.agent_event_tx.send(AgentEvent::PhaseEntered {
            session_id: self.session_id.clone(),
            phase: phase.name().to_string(),
            step,
        });
    }

    async fn record_user_message(&self, text: &str) -> Result<(), SessionError> {
        let event = OpenWandTraceEvent::Session(SessionEvent::UserMessageInjected {
            text: text.to_string(),
        });
        self.mutation
            .apply(
                Actor::User,
                event,
                vec![],
                None,
                self.stream_id.clone(),
            )
            .await?;

        self.loro_state
            .append_user_message(text, None::<&str>)
            .map_err(SessionError::Internal)?;

        Ok(())
    }

    async fn record_assistant_message(&self, text: &str) -> Result<(), SessionError> {
        // Record in trace
        let event = OpenWandTraceEvent::Inference(InferenceEvent::Completed {
            model: "mock".into(),
            tokens: openwand_core::snapshots::TokenUsageSnapshot {
                input: 0,
                output: 0,
                reasoning: None,
                cache_read: None,
                cache_write: None,
            },
            stop_reason: "stop".into(),
            tool_call_count: 0,
        });
        self.mutation
            .apply(
                Actor::Llm {
                    model: "mock".into(),
                    provider: "mock".into(),
                },
                event,
                vec![],
                None,
                self.stream_id.clone(),
            )
            .await?;

        self.loro_state
            .append_assistant_message(text, None::<&str>)
            .map_err(SessionError::Internal)?;

        Ok(())
    }

    async fn assemble_llm_request(&self, config: &RunConfig) -> Result<LlmRequest, SessionError> {
        // Memory retrieval
        let _memory_context = self
            .memory
            .search(MemoryQuery::new(""))
            .await
            .unwrap_or_else(|_| openwand_memory::RetrievalContext::empty());

        // Build messages from Loro
        let messages = self.loro_state.messages().map_err(SessionError::Internal)?;
        let llm_messages: Vec<openwand_llm::LlmMessage> = messages
            .iter()
            .filter_map(|m| message_to_llm_message(m))
            .collect();

        // Tools
        let tool_defs = self.tools.available_tools();
        let llm_tools: Vec<openwand_llm::LlmToolDef> =
            tool_defs.iter().map(|t| tool_def_to_llm_tool(t)).collect();

        Ok(LlmRequest {
            target: LlmTarget {
                provider: openwand_llm::LlmProvider::Custom {
                    name: "mock".into(),
                },
                model: "mock".into(),
                base_url: None,
                api_key: None,
            },
            messages: llm_messages,
            system_prompt: config.system_prompt.clone().unwrap_or_default(),
            tools: llm_tools,
            thinking_budget: None,
            max_tokens: Some(4096),
            temperature: Some(0.7),
            tool_choice: None,
            provider_options: serde_json::Value::Null,
        })
    }

    async fn run_inference(
        &self,
        request: LlmRequest,
    ) -> Result<InferenceOutput, SessionError> {
        let mut stream = self
            .llm
            .chat_stream(request)
            .await
            .map_err(SessionError::Llm)?;

        let mut text = String::new();
        let mut tool_calls = Vec::new();

        use futures::StreamExt;
        while let Some(delta_result) = stream.next().await {
            match delta_result {
                Ok(LlmDelta::Text { delta }) => {
                    text.push_str(&delta);
                    let _ = self.agent_event_tx.send(AgentEvent::TextDelta {
                        session_id: self.session_id.clone(),
                        delta,
                    });
                }
                Ok(LlmDelta::ToolCallComplete {
                    id,
                    name,
                    arguments,
                }) => {
                    tool_calls.push(ToolCall {
                        id: openwand_core::ToolCallId(id),
                        name,
                        arguments,
                    });
                }
                Ok(LlmDelta::Done { .. }) => break,
                Ok(_) => {}
                Err(e) => return Err(SessionError::Llm(e)),
            }
        }

        Ok(InferenceOutput { text, tool_calls })
    }

    async fn gate_tool_calls(
        &self,
        calls: &[ToolCall],
    ) -> Result<GatedTools, SessionError> {
        let mut allowed = Vec::new();
        let mut blocked = Vec::new();
        let mut blocked_any = false;

        for call in calls {
            let descriptor = self.tools.get_descriptor(&call.name);

            let policy_request = openwand_policy::PolicyRequest {
                tool_call: if let Some(ref desc) = descriptor {
                    session_tool_call_to_policy(call, desc)
                } else {
                    openwand_policy::PolicyToolCall {
                        id: call.id.clone(),
                        name: call.name.clone(),
                        arguments: call.arguments.clone(),
                        declared_effect: openwand_core::tool_vocab::ToolEffect::Unknown,
                    }
                },
                mode: InteractionMode::Conversational,
                context: openwand_policy::PolicyContext {
                    working_directory: self.working_directory.clone(),
                    model: "mock".into(),
                    session_id: self.session_id.clone(),
                    recent_gate_history: vec![],
                },
            };

            let evaluation = match self.policy.evaluate_tool_call(policy_request).await {
                Ok(eval) => eval,
                Err(_) => {
                    blocked_any = true;
                    blocked.push(call.clone());
                    continue;
                }
            };

            match evaluation.decision {
                GateDecision::Allow => {
                    allowed.push(call.clone());
                }
                GateDecision::RequireConfirmation { .. } => {
                    blocked_any = true;
                    blocked.push(call.clone());
                }
                GateDecision::Block { .. } => {
                    blocked_any = true;
                    blocked.push(call.clone());
                }
            }
        }

        Ok(GatedTools {
            allowed,
            blocked,
            blocked_any,
        })
    }

    async fn execute_tools(
        &self,
        calls: &[ToolCall],
        config: &RunConfig,
    ) -> Result<Vec<crate::tool::ToolResult>, SessionError> {
        let mut results = Vec::new();
        for call in calls {
            let tools_call: openwand_tools::executor::ToolCall = call.into();
            let context = build_tool_context(
                self.session_id.clone(),
                config.working_directory.clone(),
                self.cancellation.clone(),
            );

            let result = self.tools.execute(&tools_call, &context).await;
            results.push(crate::tool::ToolResult::from(result));
        }
        Ok(results)
    }

    async fn record_blocked_tools(&self, _calls: &[ToolCall]) -> Result<(), SessionError> {
        Ok(())
    }

    async fn record_tool_results(
        &self,
        results: &[crate::tool::ToolResult],
    ) -> Result<(), SessionError> {
        for result in results {
            self.loro_state
                .append_tool_result(result, None::<&str>)
                .map_err(SessionError::Internal)?;
        }
        Ok(())
    }
}
