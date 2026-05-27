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
use openwand_core::events::{
    GateEvent, InferenceEvent, OpenWandTraceEvent, SessionEvent, ToolEvent,
};
use openwand_store::StoredEvent;
use openwand_core::mode::InteractionMode;
use openwand_core::tool_vocab::ToolResultStatus;
use openwand_core::SessionId;
use openwand_core::ToolCallId;
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
    /// Tools that hit RequireConfirmation (may be resumable in Conversational mode).
    pending_confirmation: Vec<PendingTool>,
    /// Tools that were hard-blocked by policy (Block decision).
    hard_blocked: Vec<ToolCall>,
    /// True if any tool was blocked (either hard or pending).
    any_blocked: bool,
}

/// A tool that requires confirmation before execution.
#[derive(Debug, Clone)]
pub struct PendingTool {
    pub tool_call: ToolCall,
    pub gate_evaluation: openwand_policy::PolicyEvaluation,
}

/// User's decision on a pending tool approval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalDecision {
    /// User approved — execute the tool.
    Approved,
    /// User rejected — do not execute.
    Rejected,
}

/// Result of resuming a pending approval.
#[derive(Debug, Clone)]
pub struct ApprovalResult {
    pub decision: ApprovalDecision,
    pub tool_name: String,
    pub tool_call_id: ToolCallId,
    /// If approved and executed: the tool result.
    pub tool_result: Option<crate::tool::ToolResult>,
}

pub struct SessionRunner {
    pub session_id: SessionId,
    stream_id: TraceStreamId,

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

    /// Pending tool awaiting user approval. Set when runner suspends for confirmation.
    pending_approval: Mutex<Option<PendingTool>>,
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
            pending_approval: Mutex::new(None),
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

    /// Get the current pending approval (if any).
    pub async fn pending_approval(&self) -> Option<PendingTool> {
        self.pending_approval.lock().await.clone()
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
        let _ = self.agent_event_tx.send(AgentEvent::RunStarted {
            session_id: self.session_id.clone(),
        });
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
            let gated = self.gate_tool_calls(&inference_output.tool_calls, config.mode.clone()).await?;

            // Record gate evaluations in trace for ALL decisions
            self.record_gate_evaluations(&gated).await?;

            if !gated.hard_blocked.is_empty() {
                // Hard-blocked tools: record denied, stop
                self.record_denied_tools(&gated.hard_blocked).await?;
                stop_reason = RunStopReason::ToolBlocked;
                break;
            }

            if !gated.pending_confirmation.is_empty() {
                // In Direct mode: treat pending as blocked (no way to get approval)
                if matches!(config.mode, InteractionMode::Direct) {
                    let blocked_calls: Vec<ToolCall> = gated.pending_confirmation.iter().map(|p| p.tool_call.clone()).collect();
                    self.record_denied_tools(&blocked_calls).await?;
                    stop_reason = RunStopReason::ToolBlocked;
                    break;
                }

                // In Conversational/AutoRouting: suspend for approval
                // Only one pending tool at a time (batch 1 simplification)
                let pending = gated.pending_confirmation.into_iter().next().unwrap();

                // Record tool.suspended in trace BEFORE pausing
                self.record_tool_suspended(&pending).await?;

                // Store pending approval
                *self.pending_approval.lock().await = Some(pending.clone());

                // Emit approval requested event
                let _ = self.agent_event_tx.send(AgentEvent::ApprovalRequested {
                    session_id: self.session_id.clone(),
                    tool_name: pending.tool_call.name.clone(),
                    tool_call_id: pending.tool_call.id.clone(),
                    reason: format!(
                        "Tool '{}' requires your approval before execution.",
                        pending.tool_call.name
                    ),
                });

                stop_reason = RunStopReason::AwaitingApproval;
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

        let _ = self.agent_event_tx.send(AgentEvent::RunCompleted {
            session_id: self.session_id.clone(),
            stop_reason: format!("{:?}", stop_reason),
        });

        Ok(RunSummary {
            stop_reason,
            steps_completed,
            tools_executed,
            recoverable: true,
        })
    }

    /// Resume a pending approval with the user's decision.
    ///
    /// On approval: records `tool.resumed` in trace, then executes the tool.
    /// On rejection: records `tool.denied` in trace, feeds rejection to LLM.
    ///
    /// **Critical invariant**: ToolExecutor::execute is only called after
    /// `tool.resumed` is durably recorded in trace.
    pub async fn resume_with_approval(
        &self,
        decision: ApprovalDecision,
        config: RunConfig,
    ) -> Result<ApprovalResult, SessionError> {
        // Take the pending approval (exactly-once consumption)
        let pending = self.pending_approval.lock().await.take();
        let pending = pending.ok_or(SessionError::NoPendingApproval)?;

        let tool_call_id = pending.tool_call.id.clone();
        let tool_name = pending.tool_call.name.clone();

        match decision {
            ApprovalDecision::Approved => {
                // Record tool.resumed in trace BEFORE execution
                // This is the durable approval record
                self.record_tool_resumed(&pending).await?;

                // NOW execute the tool
                let tools_call: openwand_tools::executor::ToolCall = (&pending.tool_call).into();
                let context = build_tool_context(
                    self.session_id.clone(),
                    config.working_directory.clone(),
                    self.cancellation.clone(),
                );

                let result = self.tools.execute(&tools_call, &context).await;
                let tool_result = crate::tool::ToolResult::from(result);

                // Record in Loro state
                self.loro_state
                    .append_tool_result(&tool_result, None::<&str>)
                    .map_err(SessionError::Internal)?;

                Ok(ApprovalResult {
                    decision: ApprovalDecision::Approved,
                    tool_name,
                    tool_call_id,
                    tool_result: Some(tool_result),
                })
            }
            ApprovalDecision::Rejected => {
                // Record tool.denied in trace (no execution)
                self.record_tool_denied_event(&pending).await?;

                // Inject denied result into conversation so LLM can adjust
                let denied_result = crate::tool::ToolResult {
                    tool_call_id: pending.tool_call.id.clone(),
                    tool_name: pending.tool_call.name.clone(),
                    output: format!("Tool '{}' was denied by user. Do not retry without asking differently.", pending.tool_call.name),
                    is_error: true,
                    duration_ms: 0,
                };
                self.loro_state
                    .append_tool_result(&denied_result, None::<&str>)
                    .map_err(SessionError::Internal)?;

                Ok(ApprovalResult {
                    decision: ApprovalDecision::Rejected,
                    tool_name,
                    tool_call_id,
                    tool_result: None,
                })
            }
        }
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

        let assistant_event = OpenWandTraceEvent::Session(
            SessionEvent::AssistantMessageGenerated {
                text: text.to_string(),
                model: "unknown".into(),
            },
        );
        self.mutation
            .apply(
                Actor::Llm {
                    model: "mock".into(),
                    provider: "mock".into(),
                },
                assistant_event,
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
        let last_user_text = self.loro_state.last_user_message_text().unwrap_or_default();
        let memory_context = if !last_user_text.is_empty() {
            self.memory
                .search(MemoryQuery::new(&last_user_text))
                .await
                .unwrap_or_else(|_| openwand_memory::RetrievalContext::empty())
        } else {
            openwand_memory::RetrievalContext::empty()
        };
        let memory_block = memory_context.to_context_block();

        let messages = self.loro_state.messages().map_err(SessionError::Internal)?;
        let llm_messages: Vec<openwand_llm::LlmMessage> = messages
            .iter()
            .filter_map(|m| message_to_llm_message(m))
            .collect();

        let tool_defs = self.tools.available_tools();
        let llm_tools: Vec<openwand_llm::LlmToolDef> =
            tool_defs.iter().map(|t| tool_def_to_llm_tool(t)).collect();

        Ok(LlmRequest {
            target: config.llm_target.clone().unwrap_or(LlmTarget {
                provider: openwand_llm::LlmProvider::Custom {
                    name: "mock".into(),
                },
                model: "mock".into(),
                base_url: None,
                api_key: None,
            }),
            messages: llm_messages,
            system_prompt: config.system_prompt.clone().unwrap_or_else(|| {
                let mut base = if llm_tools.is_empty() {
                    String::new()
                } else {
                    "You are a helpful assistant with access to tools. When the user asks you to perform an action that can be fulfilled by one of your tools, call the tool instead of explaining how to do it manually.".to_string()
                };
                if let Some(ref block) = memory_block {
                    base.push_str(&format!("\n\n## Retrieved Memory Context\n\n{}\n\nUse this context when relevant to the user's request.", block));
                }
                base
            }),
            tools: llm_tools.clone(),
            thinking_budget: None,
            max_tokens: Some(4096),
            temperature: Some(0.7),
            tool_choice: if llm_tools.is_empty() { None } else { Some(openwand_llm::LlmToolChoice::Auto) },
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
        mode: InteractionMode,
    ) -> Result<GatedTools, SessionError> {
        let mut allowed = Vec::new();
        let mut pending_confirmation = Vec::new();
        let mut hard_blocked = Vec::new();
        let mut any_blocked = false;

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
                mode: mode.clone(),
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
                    any_blocked = true;
                    hard_blocked.push(call.clone());
                    continue;
                }
            };

            match evaluation.decision {
                GateDecision::Allow => {
                    allowed.push(call.clone());
                }
                GateDecision::RequireConfirmation { .. } => {
                    any_blocked = true;
                    pending_confirmation.push(PendingTool {
                        tool_call: call.clone(),
                        gate_evaluation: evaluation,
                    });
                }
                GateDecision::Block { .. } => {
                    any_blocked = true;
                    hard_blocked.push(call.clone());
                }
            }
        }

        Ok(GatedTools {
            allowed,
            pending_confirmation,
            hard_blocked,
            any_blocked,
        })
    }

    async fn execute_tools(
        &self,
        calls: &[ToolCall],
        config: &RunConfig,
    ) -> Result<Vec<crate::tool::ToolResult>, SessionError> {
        let mut results = Vec::new();
        for call in calls {
            let _ = self.agent_event_tx.send(AgentEvent::ToolCallStarted {
                session_id: self.session_id.clone(),
                tool_name: call.name.clone(),
                tool_call_id: call.id.clone(),
            });

            let tools_call: openwand_tools::executor::ToolCall = call.into();
            let context = build_tool_context(
                self.session_id.clone(),
                config.working_directory.clone(),
                self.cancellation.clone(),
            );

            let result = self.tools.execute(&tools_call, &context).await;
            let tool_result = crate::tool::ToolResult::from(result);

            let preview = {
                let text = &tool_result.output;
                if text.len() > 200 {
                    format!("{}...", &text[..200])
                } else {
                    text.clone()
                }
            };
            let _ = self.agent_event_tx.send(AgentEvent::ToolCallCompleted {
                session_id: self.session_id.clone(),
                tool_name: call.name.clone(),
                tool_call_id: call.id.clone(),
                result_preview: preview,
                is_error: tool_result.is_error,
            });

            results.push(tool_result);
        }
        Ok(results)
    }

    // ---- Trace recording ----

    /// Record gate.evaluated for every gate decision (allow, confirm, block).
    async fn record_gate_evaluations(&self, gated: &GatedTools) -> Result<(), SessionError> {
        // Record gate events for allowed tools
        for call in &gated.allowed {
            let event = OpenWandTraceEvent::Gate(GateEvent::Evaluated {
                gate_id: call.id.to_string(),
                gate_kind: "policy".into(),
                passed: true,
                risk_level: Some(openwand_core::risk::RiskLevelSnapshot::Low),
                reason_code: Some("allowed".into()),
                summary: format!("Tool '{}' passed policy gate", call.name),
            });
            self.mutation
                .apply(
                    Actor::System { component: "gate".into() },
                    event,
                    vec![],
                    None,
                    self.stream_id.clone(),
                )
                .await?;
        }

        // Record gate events for pending-confirmation tools
        for pending in &gated.pending_confirmation {
            let event = OpenWandTraceEvent::Gate(GateEvent::Evaluated {
                gate_id: pending.tool_call.id.to_string(),
                gate_kind: "policy".into(),
                passed: false,
                risk_level: Some(openwand_core::risk::RiskLevelSnapshot::Medium),
                reason_code: Some("require_confirmation".into()),
                summary: format!("Tool '{}' requires confirmation", pending.tool_call.name),
            });
            self.mutation
                .apply(
                    Actor::System { component: "gate".into() },
                    event,
                    vec![],
                    None,
                    self.stream_id.clone(),
                )
                .await?;
        }

        // Record gate events for hard-blocked tools
        for call in &gated.hard_blocked {
            let event = OpenWandTraceEvent::Gate(GateEvent::Evaluated {
                gate_id: call.id.to_string(),
                gate_kind: "policy".into(),
                passed: false,
                risk_level: Some(openwand_core::risk::RiskLevelSnapshot::High),
                reason_code: Some("blocked".into()),
                summary: format!("Tool '{}' blocked by policy", call.name),
            });
            self.mutation
                .apply(
                    Actor::System { component: "gate".into() },
                    event,
                    vec![],
                    None,
                    self.stream_id.clone(),
                )
                .await?;
        }

        Ok(())
    }

    /// Record tool.suspended in trace (pending approval).
    async fn record_tool_suspended(&self, pending: &PendingTool) -> Result<(), SessionError> {
        let event = OpenWandTraceEvent::Tool(ToolEvent::Suspended {
            tool_call_id: pending.tool_call.id.clone(),
            tool_name: pending.tool_call.name.clone(),
            reason: "awaiting_user_approval".into(),
        });
        self.mutation
            .apply(
                Actor::System { component: "gate".into() },
                event,
                vec![],
                None,
                self.stream_id.clone(),
            )
            .await?;
        Ok(())
    }

    /// Record tool.resumed in trace (approval granted).
    /// This is the durable approval record — must exist before ToolExecutor::execute.
    async fn record_tool_resumed(&self, pending: &PendingTool) -> Result<(), SessionError> {
        let event = OpenWandTraceEvent::Tool(ToolEvent::Resumed {
            tool_call_id: pending.tool_call.id.clone(),
            tool_name: pending.tool_call.name.clone(),
            resolution: "approved".into(),
        });
        self.mutation
            .apply(
                Actor::System { component: "gate".into() },
                event,
                vec![],
                None,
                self.stream_id.clone(),
            )
            .await?;
        Ok(())
    }

    /// Record tool.denied in trace (approval rejected or hard-blocked).
    async fn record_tool_denied_event(&self, pending: &PendingTool) -> Result<(), SessionError> {
        let event = OpenWandTraceEvent::Tool(ToolEvent::Denied {
            tool_call_id: pending.tool_call.id.clone(),
            tool_name: pending.tool_call.name.clone(),
        });
        self.mutation
            .apply(
                Actor::System { component: "gate".into() },
                event,
                vec![],
                None,
                self.stream_id.clone(),
            )
            .await?;
        Ok(())
    }

    /// Record tool.denied for hard-blocked tools (no pending approval context).
    async fn record_denied_tools(&self, calls: &[ToolCall]) -> Result<(), SessionError> {
        for call in calls {
            let event = OpenWandTraceEvent::Tool(ToolEvent::Denied {
                tool_call_id: call.id.clone(),
                tool_name: call.name.clone(),
            });
            self.mutation
                .apply(
                    Actor::System { component: "gate".into() },
                    event,
                    vec![],
                    None,
                    self.stream_id.clone(),
                )
                .await?;
        }
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
