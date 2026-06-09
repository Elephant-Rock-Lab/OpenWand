//! UI session service — the bridge between store and UI.
//!
//! The UI consumes this service, never the raw store.
//! This is the composition boundary where store types become UI types.

use crate::ui::dto::{
    CreateSessionRequest, UiMessageRole, UiSessionSummary, UiSessionView,
};
use crate::ui::replay::{self, UiTimelineItem};
use crate::ui::run_bridge;
use crate::ui::run_dto::{UiRunState, UiRunStatus};
use openwand_core::mode::InteractionMode;
use openwand_core::SessionId;
use openwand_llm::LlmTarget;
use openwand_store::{
    NewSessionRecord, SessionListFilter, SessionRegistryStore, SessionRegistryUpdate,
};
use openwand_session::config::RunConfig;
use openwand_session::runner::SessionRunner;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// Error type for UI session operations.
#[derive(Debug, thiserror::Error)]
pub enum UiServiceError {
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("Run already active for session: {0}")]
    RunAlreadyActive(String),
    #[error("Store error: {0}")]
    Store(#[from] openwand_store::StoreError),
    #[error("Session error: {0}")]
    Session(#[from] openwand_session::SessionError),
    #[error("Internal: {0}")]
    Internal(String),
}

/// Handle to an active run. UI polls the shared state.
pub struct RunHandle {
    pub state: Arc<std::sync::Mutex<UiRunState>>,
    pub cancellation: CancellationToken,
}

/// The UI session service. Wraps the store registry and coordinates runs.
pub struct UiSessionService {
    registry: Arc<dyn SessionRegistryStore>,
    trace: Arc<dyn openwand_trace::TraceStore<openwand_store::StoredEvent>>,
}

impl UiSessionService {
    pub fn new(
        registry: Arc<dyn SessionRegistryStore>,
        trace: Arc<dyn openwand_trace::TraceStore<openwand_store::StoredEvent>>,
    ) -> Self {
        Self { registry, trace }
    }

    /// List all non-archived sessions, most recently updated first.
    pub fn list_sessions(&self) -> Result<Vec<UiSessionSummary>, UiServiceError> {
        let summaries = self
            .registry
            .list_sessions(SessionListFilter {
                include_archived: false,
                limit: Some(50),
            })
            .map_err(UiServiceError::Store)?;

        Ok(summaries
            .into_iter()
            .map(|s| UiSessionSummary {
                session_id: s.session_id,
                title: s.title,
                status: s.status,
                updated_at: s.updated_at,
                last_message_preview: s.last_message_preview,
                model: s.model,
                current_phase: s.current_phase,
            })
            .collect())
    }

    /// Create a new session. Returns the summary for immediate display.
    pub fn create_session(
        &self,
        request: CreateSessionRequest,
    ) -> Result<UiSessionSummary, UiServiceError> {
        let session_id = SessionId::new().to_string();

        let record = self
            .registry
            .create_session(NewSessionRecord {
                session_id: session_id.clone(),
                title: request.title,
                provider: request.provider,
                model: request.model,
                base_url: request.base_url,
                working_directory: request.working_directory,
                interaction_mode: request.interaction_mode,
            })
            .map_err(UiServiceError::Store)?;

        Ok(UiSessionSummary {
            session_id: record.session_id,
            title: record.title,
            status: record.status,
            updated_at: record.updated_at,
            last_message_preview: record.last_message_preview,
            model: record.model,
            current_phase: record.current_phase,
        })
    }

    /// Open a session for viewing. Returns the full view including messages.
    ///
    /// Open a session for viewing. Rebuilds the timeline from trace.
    pub async fn open_session(&self, session_id: &str) -> Result<UiSessionView, UiServiceError> {
        let record = self
            .registry
            .get_session(session_id)
            .map_err(UiServiceError::Store)?
            .ok_or_else(|| UiServiceError::NotFound(session_id.to_string()))?;

        // Replay timeline from trace
        let timeline = replay::replay_timeline(self.trace.as_ref(), session_id)
            .await
            .unwrap_or_default();

        // Convert timeline items to UI messages
        let messages: Vec<crate::ui::dto::UiMessage> = timeline
            .iter()
            .filter_map(|item| match item {
                UiTimelineItem::Message(msg) => Some(msg.clone()),
                _ => None,
            })
            .collect();

        Ok(UiSessionView {
            summary: UiSessionSummary {
                session_id: record.session_id,
                title: record.title,
                status: record.status,
                updated_at: record.updated_at,
                last_message_preview: record.last_message_preview,
                model: record.model,
                current_phase: record.current_phase,
            },
            messages,
            interaction_mode: record.interaction_mode,
            current_step: record.current_step,
            provider: record.provider,
            base_url: record.base_url,
            working_directory: record.working_directory,
        })
    }

    /// Start a live run for a session. Returns a RunHandle for polling state.
    ///
    /// This creates a SessionRunner, wires the event bridge, and spawns the
    /// run_turn in a background task. The UI polls `handle.state` for updates.
    pub async fn start_run(
        &self,
        session_id: &str,
        user_text: String,
        llm_target: LlmTarget,
        runner: Arc<SessionRunner>,
        working_directory: std::path::PathBuf,
        memory_prompt_inputs: Option<openwand_memory::prompt_assembly::MemoryPromptAssemblyInputs>,
    ) -> Result<RunHandle, UiServiceError> {
        // Check run lock via try_run — if runner has an active run, it fails
        let cancellation = CancellationToken::new();

        // Set up shared state
        let state = Arc::new(std::sync::Mutex::new(UiRunState::new_running()));

        // Subscribe to runner events
        let rx = runner.subscribe();

        // Start bridge
        run_bridge::start_bridge(rx, Arc::clone(&state), cancellation.clone());

        // Build run config
        let config = RunConfig {
            max_steps: 25,
            mode: InteractionMode::Direct,
            working_directory: working_directory.display().to_string(),
            system_prompt: None,
            llm_target: Some(llm_target),
            memory_prompt_inputs,
            output_guard: None,
        };

        // Spawn the run in background
        let runner = Arc::clone(&runner);
        let session_id_owned = session_id.to_string();
        let state_clone = Arc::clone(&state);
        let registry_clone = Arc::clone(&self.registry);
        tokio::spawn(async move {
            match runner.run_turn(user_text, config).await {
                Ok(_summary) => {
                    let mut s = state_clone.lock().unwrap_or_else(|e| e.into_inner());
                    if s.status == UiRunStatus::Running {
                        s.status = UiRunStatus::Completed;
                    }
                }
                Err(e) => {
                    let mut s = state_clone.lock().unwrap_or_else(|e| e.into_inner());
                    s.status = UiRunStatus::Failed;
                    s.error = Some(e.to_string());
                }
            }
            // Update registry — set last_message_preview from streamed text
            let preview = {
                let s = state_clone.lock().unwrap_or_else(|e| e.into_inner());
                s.streamed_text.chars().take(80).collect::<String>()
            };
            let _ = registry_clone.update_session(SessionRegistryUpdate {
                session_id: session_id_owned,
                title: None,
                status: None,
                current_phase: None,
                current_step: None,
                last_message_preview: Some(preview),
                last_trace_id: None,
                last_global_sequence: None,
                snapshot_key: None,
                projection_stale: None,
                metadata_json: None,
            }).ok();
        });

        Ok(RunHandle {
            state,
            cancellation,
        })
    }

    /// Resolve a pending tool approval through the existing SessionRunner API.
    ///
    /// This is a thin adapter over `SessionRunner::resolve_approval()`.
    /// It does not construct LLM requests, execute tools, evaluate policy,
    /// or mutate pending state directly.
    pub async fn resolve_approval(
        runner: &SessionRunner,
        decision: openwand_session::runner::ApprovalDecision,
        config: RunConfig,
    ) -> Result<openwand_session::runner::ApprovalResult, UiServiceError> {
        runner
            .resolve_approval(decision, config)
            .await
            .map_err(UiServiceError::Session)
    }

    /// Refresh session view by reading from trace projection only.
    ///
    /// This is read-only: it opens the session and replays from trace.
    /// It does not call LLM, tools, policy, or memory writers.
    pub async fn refresh_session(
        &self,
        session_id: &str,
    ) -> Result<UiSessionView, UiServiceError> {
        self.open_session(session_id).await
    }
}
