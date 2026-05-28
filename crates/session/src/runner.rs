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
use openwand_core::SessionId;
use openwand_core::ToolCallId;
use openwand_core::ids::ApprovalRequestId;
use openwand_core::snapshots::ApprovalContextSnapshot;
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

/// Cache entry linking a live pending tool to its approval_request_id.
struct CachedApproval {
    approval_request_id: openwand_core::ApprovalRequestId,
    pending: PendingTool,
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
    pub declared_effect: openwand_core::ToolEffect,
}

/// How the user resolved a pending approval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalResolution {
    /// User approved — execute the tool.
    Approve,
    /// User rejected — do not execute.
    Reject { reason: Option<String> },
}

/// A governance decision resolving a pending approval.
#[derive(Debug, Clone)]
pub struct ApprovalDecision {
    /// Which approval to resolve. None = "resolve the single pending one."
    pub approval_request_id: Option<openwand_core::ApprovalRequestId>,
    /// The resolution itself.
    pub resolution: ApprovalResolution,
}

impl ApprovalDecision {
    /// Approve the single pending approval (no explicit ID).
    pub fn approve() -> Self {
        Self { approval_request_id: None, resolution: ApprovalResolution::Approve }
    }

    /// Reject the single pending approval (no explicit ID).
    pub fn reject() -> Self {
        Self { approval_request_id: None, resolution: ApprovalResolution::Reject { reason: None } }
    }

    /// Reject with an explicit reason.
    pub fn reject_with_reason(reason: impl Into<String>) -> Self {
        Self { approval_request_id: None, resolution: ApprovalResolution::Reject { reason: Some(reason.into()) } }
    }

    /// Resolve a specific approval by ID.
    pub fn for_approval(arid: openwand_core::ApprovalRequestId, resolution: ApprovalResolution) -> Self {
        Self { approval_request_id: Some(arid), resolution }
    }
}

/// How the resolver found the approval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalSource {
    /// Cache hit: pending_approval pointed to the resolved approval.
    Live,
    /// Cache miss or no cache: found by scanning trace.
    Recovered,
    /// Cache existed but pointed to a different approval than the one resolved.
    StaleCache,
}

/// UX-facing view of a pending approval.
///
/// Returned by `pending_approval()`. Contains only the fields callers need
/// for display and governance — not the full internal `PendingTool`.
#[derive(Debug, Clone)]
pub struct PendingApprovalView {
    pub approval_request_id: openwand_core::ApprovalRequestId,
    pub tool_call_id: ToolCallId,
    pub tool_name: String,
    pub risk_level: openwand_core::RiskLevelSnapshot,
    pub confirmation_level: openwand_core::ConfirmationLevel,
    pub policy_summary: String,
    pub requested_action_summary: String,
}

/// Result of resuming a pending approval.
#[derive(Debug, Clone)]
pub struct ApprovalResult {
    pub resolution: ApprovalResolution,
    pub tool_name: String,
    pub tool_call_id: ToolCallId,
    pub approval_request_id: openwand_core::ApprovalRequestId,
    /// If approved and executed: the tool result.
    pub tool_result: Option<crate::tool::ToolResult>,
    /// How the resolver found the approval.
    pub source: ApprovalSource,
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
    pending_approval: Mutex<Option<CachedApproval>>,
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
    pub async fn pending_approval(&self) -> Option<PendingApprovalView> {
        self.pending_approval.lock().await.as_ref().map(|c| PendingApprovalView {
            approval_request_id: c.approval_request_id.clone(),
            tool_call_id: c.pending.tool_call.id.clone(),
            tool_name: c.pending.tool_call.name.clone(),
            risk_level: c.pending.gate_evaluation.risk_level.clone(),
            confirmation_level: c.pending.gate_evaluation.confirmation_level.clone(),
            policy_summary: c.pending.gate_evaluation.summary.clone(),
            requested_action_summary: format!("Execute '{}' with provided arguments", c.pending.tool_call.name),
        })
    }

    /// Get the approval_request_id from the cache, if any.
    async fn pending_approval_hint(&self) -> Option<openwand_core::ApprovalRequestId> {
        self.pending_approval
            .lock()
            .await
            .as_ref()
            .map(|c| c.approval_request_id.clone())
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
                let mut pending_iter = gated.pending_confirmation.into_iter();
                let pending = pending_iter.next().unwrap();
                let remaining_pending: Vec<PendingTool> = pending_iter.collect();

                // Record tool.suspended in trace BEFORE pausing
                // Returns the approval_request_id if suspension succeeded
                let suspended_arid = self.record_tool_suspended(&pending, step).await?;

                // If oversized args caused blocking instead of suspension, skip
                if let Some(arid) = suspended_arid {
                    // Emit tool.deferred for remaining confirmation-requiring tools
                    for deferred in &remaining_pending {
                        self.record_tool_deferred(
                            &deferred.tool_call,
                            &arid,
                            &pending.tool_call.id,
                        )
                        .await?;
                    }

                    // Emit tool.deferred for allowed tools (batch is frozen on suspension)
                    for allowed in &gated.allowed {
                        self.record_tool_deferred(
                            allowed,
                            &arid,
                            &pending.tool_call.id,
                        )
                        .await?;
                    }

                    // Store pending approval with its arid
                    *self.pending_approval.lock().await = Some(CachedApproval {
                        approval_request_id: arid.clone(),
                        pending: pending.clone(),
                    });

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
                } else {
                    // Oversized: tool was blocked, not suspended
                    stop_reason = RunStopReason::ToolBlocked;
                }
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
    /// Resolve a pending tool approval.
    ///
    /// Single public API for both live and recovered approvals.
    /// Builds recovery index once, uses cache as hint, resolves from trace.
    pub async fn resolve_approval(
        &self,
        decision: ApprovalDecision,
        config: RunConfig,
    ) -> Result<ApprovalResult, SessionError> {
        // Phase 1: Build recovery index (single scan)
        let index = self.approval_recovery_index().await?;

        // Phase 1.5: Idempotency check — if caller specified an arid and it's already resolved
        if let Some(arid) = decision.approval_request_id.as_ref() {
            if let Some(resolved) = index.resolved.iter().find(|r| &r.approval_request_id == arid) {
                return Ok(ApprovalResult {
                    resolution: match resolved.kind {
                        crate::approval_recovery::ResolvedApprovalKind::Approved => ApprovalResolution::Approve,
                        crate::approval_recovery::ResolvedApprovalKind::Denied => ApprovalResolution::Reject { reason: None },
                    },
                    tool_name: resolved.tool_name.clone(),
                    tool_call_id: resolved.tool_call_id.clone(),
                    approval_request_id: resolved.approval_request_id.clone(),
                    tool_result: None,
                    source: ApprovalSource::Recovered,
                });
            }
        }

        // Phase 2: Select target (pure logic)
        let cache_hint = self.pending_approval_hint().await;
        let (target, source) = select_approval_target(&index, cache_hint, &decision)?;

        // Phase 3: Resolve from index (no second scan)
        let mut result = self
            .resolve_from_index(&index, &target, decision, config)
            .await?;

        // Stamp the source determined by the selector
        result.source = source;

        // Clear cache after successful resolution
        self.pending_approval.lock().await.take();

        Ok(result)
    }

    /// Effectful resolver: appends trace events, executes tool, mutates Loro.
    /// Takes a pre-built index — no second scan.
    async fn resolve_from_index(
        &self,
        index: &crate::approval_recovery::ApprovalRecoveryIndex,
        target: &crate::approval_recovery::PendingApprovalRecovery,
        decision: ApprovalDecision,
        config: RunConfig,
    ) -> Result<ApprovalResult, SessionError> {
        // Check for conflicts
        if !index.conflicts.is_empty() {
            return Err(SessionError::Internal(format!(
                "Cannot resolve approval: {} conflict(s) detected",
                index.conflicts.len()
            )));
        }

        let tool_name = target.tool_name.clone();
        let tool_call_id = target.context.tool_call_id.clone();
        let approval_request_id = target.context.approval_request_id.clone();

        match decision.resolution {
            ApprovalResolution::Approve => {
                // 1. Append tool.resumed BEFORE execution (durable approval proof)
                let resumed_event = OpenWandTraceEvent::Tool(ToolEvent::Resumed {
                    tool_call_id: tool_call_id.clone(),
                    tool_name: tool_name.clone(),
                    resolution: "approved".into(),
                    approval_request_id: Some(approval_request_id.clone()),
                });
                self.mutation
                    .apply(
                        Actor::System { component: "gate".into() },
                        resumed_event,
                        vec![],
                        None,
                        self.stream_id.clone(),
                    )
                    .await?;

                // 2. Record tool.called BEFORE execution
                let called_event = OpenWandTraceEvent::Tool(ToolEvent::Called {
                    tool_call_id: tool_call_id.clone(),
                    tool_name: tool_name.clone(),
                    args_hash: target.context.args_hash.clone(),
                    invoker: openwand_core::tool_vocab::ToolInvoker::Llm,
                });
                self.mutation
                    .apply(
                        Actor::System { component: "tool".into() },
                        called_event,
                        vec![],
                        None,
                        self.stream_id.clone(),
                    )
                    .await?;

                // 3. Execute tool using persisted arguments from context
                let tools_call = openwand_tools::executor::ToolCall {
                    id: tool_call_id.clone(),
                    name: tool_name.clone(),
                    arguments: target.context.arguments.clone(),
                };
                let context = build_tool_context(
                    self.session_id.clone(),
                    config.working_directory.clone(),
                    self.cancellation.clone(),
                );
                let result = self.tools.execute(&tools_call, &context).await;
                let tool_result = crate::tool::ToolResult::from(result);

                // 4. Record tool.completed or tool.failed AFTER execution
                let terminal_event = if tool_result.is_error {
                    OpenWandTraceEvent::Tool(ToolEvent::Failed {
                        tool_call_id: tool_call_id.clone(),
                        tool_name: tool_name.clone(),
                        error: tool_result.output.clone(),
                    })
                } else {
                    OpenWandTraceEvent::Tool(ToolEvent::Completed {
                        tool_call_id: tool_call_id.clone(),
                        tool_name: tool_name.clone(),
                        status: openwand_core::tool_vocab::ToolResultStatus::Success,
                        result_summary: tool_result.output.chars().take(200).collect(),
                        duration_ms: tool_result.duration_ms,
                    })
                };
                self.mutation
                    .apply(
                        Actor::System { component: "tool".into() },
                        terminal_event,
                        vec![],
                        None,
                        self.stream_id.clone(),
                    )
                    .await?;

                // 5. Record in Loro state
                self.loro_state
                    .append_tool_result(&tool_result, None::<&str>)
                    .map_err(SessionError::Internal)?;

                // 6. Clear waiting approval in Loro
                self.loro_state
                    .clear_waiting_approval()
                    .map_err(SessionError::Internal)?;

                Ok(ApprovalResult {
                    resolution: ApprovalResolution::Approve,
                    tool_name,
                    tool_call_id,
                    approval_request_id,
                    tool_result: Some(tool_result),
                    source: ApprovalSource::Live, // overwritten by caller
                })
            }
            ApprovalResolution::Reject { ref reason } => {
                // 1. Append tool.denied (no execution)
                let denied_event = OpenWandTraceEvent::Tool(ToolEvent::Denied {
                    tool_call_id: tool_call_id.clone(),
                    tool_name: tool_name.clone(),
                    approval_request_id: Some(approval_request_id.clone()),
                    reason: reason.clone().or_else(|| Some("user_rejected".into())),
                });
                self.mutation
                    .apply(
                        Actor::System { component: "gate".into() },
                        denied_event,
                        vec![],
                        None,
                        self.stream_id.clone(),
                    )
                    .await?;

                // 2. Inject denied result into conversation for LLM continuation
                let denied_result = crate::tool::ToolResult {
                    tool_call_id: tool_call_id.clone(),
                    tool_name: tool_name.clone(),
                    output: format!(
                        "Tool '{}' was denied by user. Do not retry without asking differently.",
                        tool_name
                    ),
                    is_error: true,
                    duration_ms: 0,
                };
                self.loro_state
                    .append_tool_result(&denied_result, None::<&str>)
                    .map_err(SessionError::Internal)?;

                // 3. Clear waiting approval in Loro
                self.loro_state
                    .clear_waiting_approval()
                    .map_err(SessionError::Internal)?;

                Ok(ApprovalResult {
                    resolution: ApprovalResolution::Reject { reason: reason.clone() },
                    tool_name,
                    tool_call_id,
                    approval_request_id,
                    tool_result: None,
                    source: ApprovalSource::Live, // overwritten by caller
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
                        declared_effect: descriptor
                            .as_ref()
                            .map(|d| d.declared_effect.clone())
                            .unwrap_or(openwand_core::ToolEffect::Unknown),
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
    async fn record_tool_suspended(&self, pending: &PendingTool, step: u64) -> Result<Option<ApprovalRequestId>, SessionError> {
        use crate::approval_recovery::{approval_args_hash, validate_approval_context_size};

        // Validate argument size before suspending
        if let Err(size_err) = validate_approval_context_size(&pending.tool_call.arguments) {
            // Oversized: block fail-closed with trace evidence
            tracing::warn!(tool = %pending.tool_call.name, "{}", size_err);

            // Append gate.evaluated with failure reason
            let gate_event = OpenWandTraceEvent::Gate(GateEvent::Evaluated {
                gate_id: pending.gate_evaluation.gate_id.as_str().to_string(),
                gate_kind: "tool_policy".into(),
                passed: false,
                risk_level: Some(openwand_core::RiskLevelSnapshot::Critical),
                reason_code: Some("approval_context_too_large".into()),
                summary: format!("Approval context exceeded size limit: {size_err}"),
            });
            self.mutation
                .apply(
                    Actor::System { component: "gate".into() },
                    gate_event,
                    vec![],
                    None,
                    self.stream_id.clone(),
                )
                .await?;

            // Append tool.denied
            let denied_event = OpenWandTraceEvent::Tool(ToolEvent::Denied {
                tool_call_id: pending.tool_call.id.clone(),
                tool_name: pending.tool_call.name.clone(),
                approval_request_id: None,
                reason: Some("approval_context_too_large".into()),
            });
            self.mutation
                .apply(
                    Actor::System { component: "gate".into() },
                    denied_event,
                    vec![],
                    None,
                    self.stream_id.clone(),
                )
                .await?;

            // Inject blocked result for model
            let blocked_result = crate::tool::ToolResult {
                tool_call_id: pending.tool_call.id.clone(),
                tool_name: pending.tool_call.name.clone(),
                output: format!("Tool '{}' blocked: approval context too large", pending.tool_call.name),
                is_error: true,
                duration_ms: 0,
            };
            self.loro_state
                .append_tool_result(&blocked_result, None::<&str>)
                .map_err(SessionError::Internal)?;

            return Ok(None);
        }

        let args_hash = approval_args_hash(&pending.tool_call.arguments)
            .unwrap_or_else(|_| "hash_error".into());

        let approval_request_id = ApprovalRequestId::new();
        let approval_context = ApprovalContextSnapshot {
            approval_request_id: approval_request_id.clone(),
            gate_id: pending.gate_evaluation.gate_id.clone(),
            step,
            tool_call_id: pending.tool_call.id.clone(),
            tool_name: pending.tool_call.name.clone(),
            arguments: pending.tool_call.arguments.clone(),
            args_hash,
            declared_effect: pending.declared_effect.clone(),
            risk_level: pending.gate_evaluation.risk_level.clone(),
            confirmation_level: pending.gate_evaluation.confirmation_level.clone(),
            reason_code: pending.gate_evaluation.reason_code.clone(),
            policy_summary: pending.gate_evaluation.summary.clone(),
            requested_action_summary: format!("Execute '{}' with provided arguments", pending.tool_call.name),
            rollback_plan: pending.gate_evaluation.rollback_plan.clone(),
            metadata: serde_json::Value::Null,
        };

        let event = OpenWandTraceEvent::Tool(ToolEvent::Suspended {
            tool_call_id: pending.tool_call.id.clone(),
            tool_name: pending.tool_call.name.clone(),
            reason: "awaiting_user_approval".into(),
            approval_context: Some(approval_context),
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
        Ok(Some(approval_request_id))
    }

    /// Record tool.deferred for a tool call that was deferred because another tool suspended the batch.
    async fn record_tool_deferred(
        &self,
        call: &ToolCall,
        blocking_arid: &ApprovalRequestId,
        blocking_tool_call_id: &ToolCallId,
    ) -> Result<(), SessionError> {
        use crate::approval_recovery::approval_args_hash;

        let args_hash = approval_args_hash(&call.arguments).ok();
        let event = OpenWandTraceEvent::Tool(ToolEvent::Deferred {
            tool_call_id: call.id.clone(),
            tool_name: call.name.clone(),
            reason: "deferred: another approval pending".into(),
            blocked_by_tool_call_id: Some(blocking_tool_call_id.clone()),
            blocked_by_approval_request_id: Some(blocking_arid.clone()),
            original_order_index: None, // not tracked in current GatedTools
            args_hash,
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
                approval_request_id: None,
                reason: None,
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

    /// Find tool.suspended events that have no matching tool.resumed or tool.denied.
    /// These represent crash-interrupted approvals that could be recovered.
    /// Build the full recovery index from trace.
    /// Replaces the old diagnostic-only `unresolved_suspensions` method.
    pub async fn approval_recovery_index(
        &self,
    ) -> Result<crate::approval_recovery::ApprovalRecoveryIndex, SessionError> {
        use openwand_trace::TraceQuery;

        // Collect ALL events from the session stream
        let query = TraceQuery {
            ..Default::default()
        };
        let page = self.trace.scan(query).await.map_err(SessionError::Trace)?;

        Ok(crate::approval_recovery::build_recovery_index(&page.entries))
    }

    /// Legacy compatibility: unresolved suspensions as a simple list.
    /// Derives from the full recovery index.
    pub async fn unresolved_suspensions(&self) -> Result<Vec<UnresolvedSuspension>, SessionError> {
        let index = self.approval_recovery_index().await?;
        Ok(index
            .pending
            .into_iter()
            .map(|p| UnresolvedSuspension {
                tool_call_id: p.context.tool_call_id,
                tool_name: p.tool_name,
                suspended_at: chrono::Utc::now(), // approximation — exact time in trace entry
            })
            .collect())
    }
}

// ---- Pure selector (free function) ----

/// Select which pending approval to resolve.
///
/// Pure function over index data + cache hint + decision.
/// Returns the target pending approval and how it was found.
pub fn select_approval_target(
    index: &crate::approval_recovery::ApprovalRecoveryIndex,
    cache_hint: Option<openwand_core::ApprovalRequestId>,
    decision: &ApprovalDecision,
) -> Result<(crate::approval_recovery::PendingApprovalRecovery, ApprovalSource), SessionError> {
    // Case 1: Caller specified an explicit approval_request_id
    if let Some(ref arid) = decision.approval_request_id {
        let matching = index
            .pending
            .iter()
            .find(|p| p.context.approval_request_id == *arid);

        return match matching {
            Some(target) => {
                let source = match cache_hint {
                    Some(hint_arid) if hint_arid == *arid => ApprovalSource::Live,
                    Some(_) => ApprovalSource::StaleCache,
                    None => ApprovalSource::Recovered,
                };
                Ok((target.clone(), source))
            }
            None => Err(SessionError::NoPendingApproval),
        };
    }

    // Case 2: Caller wants "the single pending one" — use cache hint or scan

    // Try cache hint first
    if let Some(cache_arid) = cache_hint {
        if let Some(target) = index
            .pending
            .iter()
            .find(|p| p.context.approval_request_id == cache_arid)
        {
            return Ok((target.clone(), ApprovalSource::Live));
        }
        // Cache was stale — fall through to scan
    }

    // No cache or stale cache — use index
    match index.pending.len() {
        0 => Err(SessionError::NoPendingApproval),
        1 => Ok((index.pending[0].clone(), ApprovalSource::Recovered)),
        n => Err(SessionError::Internal(format!(
            "Cannot resolve: {} pending approvals. Specify an approval_request_id.",
            n
        ))),
    }
}

/// An unresolved tool suspension — tool.suspended with no matching tool.resumed or tool.denied.
#[derive(Debug, Clone)]
pub struct UnresolvedSuspension {
    pub tool_call_id: ToolCallId,
    pub tool_name: String,
    pub suspended_at: chrono::DateTime<chrono::Utc>,
}
