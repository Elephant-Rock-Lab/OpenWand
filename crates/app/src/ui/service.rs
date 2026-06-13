//! UI session service — the bridge between store and UI.
//!
//! The UI consumes this service, never the raw store.
//! This is the composition boundary where store types become UI types.

use crate::ui::dto::{
    CreateSessionRequest, UiSessionSummary, UiSessionView,
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
        capability_context: Option<openwand_session::config::CapabilityContextBlock>,
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
            capability_context,
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

    /// Request a workflow run initiation from the desktop UI.
    ///
    /// This is the authority boundary for desktop-initiated workflow runs.
    /// The desktop constructs a `WorkflowRunRequest` with only record IDs.
    /// This method loads the full records, extracts hashes, evaluates the
    /// execution gate, advances stages, and saves the run.
    ///
    /// Authority: this method calls `evaluate_workflow_execution()` and
    /// `save_workflow_run()` — the desktop UI code does NOT.
    /// The evaluation gate enforces readiness, hash matching, and predicate
    /// checks. The desktop cannot bypass these.
    pub fn request_workflow_run(
        &self,
        request: &crate::ui::workflow_run_request::WorkflowRunRequest,
        store_root: &std::path::Path,
    ) -> crate::ui::workflow_run_request::WorkflowRunRequestState {
        Self::evaluate_workflow_run_request(request, store_root)
    }

    /// Core evaluation logic — callable without a service instance.
    /// Separated so tests can exercise the authority path without constructing
    /// a full UiSessionService with store connections.
    pub fn evaluate_workflow_run_request(
        request: &crate::ui::workflow_run_request::WorkflowRunRequest,
        store_root: &std::path::Path,
    ) -> crate::ui::workflow_run_request::WorkflowRunRequestState {
        use crate::ui::workflow_run_request::WorkflowRunRequestState;
        use openwand_workflow::workflow_run::WorkflowExecutionRequest;
        use openwand_workflow::workflow_execution_gate::{WorkflowExecutionContext, evaluate_workflow_execution};
        use openwand_workflow::workflow_readiness::WorkflowReadinessId;
        use openwand_workflow::workflow_proposal::WorkflowProposalId;
        use openwand_workflow::workflow_proposal_review::WorkflowProposalReviewId;
        use chrono::Utc;

        // Load readiness record
        let readiness = match crate::workflow_readiness::load_workflow_readiness(
            store_root,
            &WorkflowReadinessId(request.readiness_id.clone()),
        ) {
            Ok(r) => r,
            Err(e) => return WorkflowRunRequestState::Failed {
                error: format!("Failed to load readiness: {e}"),
            },
        };

        // Load proposal record
        let proposal = match crate::workflow_proposal::load_workflow_proposal(
            store_root,
            &WorkflowProposalId(request.proposal_id.clone()),
        ) {
            Ok(p) => p,
            Err(e) => return WorkflowRunRequestState::Failed {
                error: format!("Failed to load proposal: {e}"),
            },
        };

        // Load proposal review record
        let review = match crate::workflow_proposal::load_proposal_review(
            store_root,
            &WorkflowProposalReviewId(request.proposal_review_id.clone()),
        ) {
            Ok(r) => r,
            Err(e) => return WorkflowRunRequestState::Failed {
                error: format!("Failed to load review: {e}"),
            },
        };

        // Load source task plan if available
        let source_plan = crate::task_planning::load_task_plan(
            store_root,
            &proposal.source_task_plan_id,
        ).ok();

        // Load latest proposal review for idempotency check
        let latest_review = crate::workflow_proposal::latest_proposal_review(store_root)
            .ok().flatten()
            .filter(|r| r.proposal_id == proposal.proposal_id);

        // Build the execution request with hashes extracted from loaded records
        let exec_request = WorkflowExecutionRequest {
            readiness_id: readiness.readiness_id.clone(),
            proposal_id: proposal.proposal_id.clone(),
            proposal_review_id: review.review_id.clone(),
            expected_readiness_hash: readiness.proposal_hash.clone(),
            expected_proposal_hash: proposal.proposal_hash.clone(),
            requested_by: request.requested_by.clone(),
            requested_at: Utc::now(),
            idempotency_key: request.idempotency_key.clone(),
        };

        // Build evaluation context from loaded records
        let context = WorkflowExecutionContext {
            readiness: Some(readiness),
            proposal: Some(proposal),
            proposal_review: Some(review.clone()),
            latest_proposal_review: latest_review,
            source_task_plan: source_plan,
            source_task_plan_review: None,
            latest_source_task_plan_review: None,
            provider_config_available: true,
            session_runtime_available: true,
            existing_runs: vec![],
        };

        // Evaluate the execution gate
        let mut record = evaluate_workflow_execution(&exec_request, &context);

        // Advance stages through lifecycle if suspended
        if record.status == openwand_workflow::workflow_run::WorkflowRunStatus::Suspended
            && let Some(ref proposal) = context.proposal {
                let (stages, events, action_requests) =
                    openwand_workflow::workflow_run_lifecycle::advance_stages(proposal);
                record.stages = stages;
                record.lifecycle_events = events;
                record.action_requests = action_requests;
            }

        // Check if blocked
        if matches!(record.status, openwand_workflow::workflow_run::WorkflowRunStatus::Blocked) {
            let failed_predicates: Vec<_> = record.predicates.iter()
                .filter(|p| !p.passed)
                .collect();
            let reason = if failed_predicates.is_empty() {
                "Blocked by execution gate".to_string()
            } else {
                failed_predicates.iter()
                    .map(|p| format!("{:?}: {}", p.predicate, p.reason))
                    .collect::<Vec<_>>()
                    .join("; ")
            };
            return WorkflowRunRequestState::Blocked { reason };
        }

        // Save the run record
        let execution_id = record.execution_id.0.clone();
        let status = format!("{:?}", record.status).to_lowercase();
        let stage_count = record.stages.len();
        let predicates_passed = record.predicates.iter().filter(|p| p.passed).count();
        let predicates_total = record.predicates.len();

        match crate::workflow_execution::save_workflow_run(store_root, &record) {
            Ok(_) => WorkflowRunRequestState::Created {
                execution_id,
                status,
                stage_count,
                predicates_passed,
                predicates_total,
            },
            Err(e) => WorkflowRunRequestState::Failed {
                error: format!("Failed to save workflow run: {e}"),
            },
        }
    }

    /// Submit an approval resolution from the desktop UI.
    ///
    /// This is the authority boundary for desktop approval decisions.
    /// The desktop constructs an `ApprovalResolutionRequest` with an explicit
    /// ARID and a decision. This method maps the DTO to the existing
    /// `SessionRunner::resolve_approval()` API.
    ///
    /// Authority: this method constructs the `ApprovalDecision` and `RunConfig`.
    /// The desktop UI code does NOT.
    /// Tool-name and args-hash enforcement remains in the runner's pending
    /// approval snapshot — the UI's `displayed_tool_name` is display-only.
    ///
    /// Returns `Stale` if the runner is no longer active.
    pub async fn submit_approval_resolution(
        runner: Option<&SessionRunner>,
        request: &crate::ui::approval_resolution_request::ApprovalResolutionRequest,
    ) -> crate::ui::approval_resolution_request::ApprovalResolutionState {
        use crate::ui::approval_resolution_request::{
            ApprovalDecisionDto, ApprovalResolutionState,
        };
        use openwand_session::runner::{ApprovalDecision, ApprovalResolution};

        // Validate the DTO before any delegation
        if let Err(e) = request.validate() {
            return ApprovalResolutionState::Failed { error: e };
        }

        // Require an active runner — no silent success on stale state
        let runner = match runner {
            Some(r) => r,
            None => {
                return ApprovalResolutionState::Stale {
                    reason: "No active session runner".into(),
                };
            }
        };

        // Map DTO decision to backend ApprovalDecision
        let arid = openwand_core::ApprovalRequestId(
            request.approval_request_id.clone(),
        );
        let resolution = match request.decision {
            ApprovalDecisionDto::Approve => ApprovalResolution::Approve,
            ApprovalDecisionDto::Reject => ApprovalResolution::Reject {
                reason: request.rationale.clone(),
            },
        };
        let decision = ApprovalDecision::for_approval(arid, resolution);

        // The service boundary constructs RunConfig — never the UI
        let config = RunConfig {
            max_steps: 25,
            mode: InteractionMode::Conversational,
            working_directory: ".".into(),
            system_prompt: None,
            llm_target: None,
            memory_prompt_inputs: None,
            output_guard: None,
            capability_context: None,
        };

        // Delegate through the existing approval governance path
        match runner.resolve_approval(decision, config).await {
            Ok(result) => {
                let tool_status = result.tool_result.as_ref().map(|_| "completed");
                let source = format!("{:?}", result.source).to_lowercase();
                ApprovalResolutionState::Resolved {
                    decision: request.decision,
                    approval_request_id: result.approval_request_id.0.clone(),
                    tool_name: Some(result.tool_name.clone()),
                    tool_status: tool_status.map(|s| s.to_string()),
                    source,
                }
            }
            Err(e) => {
                let msg = e.to_string();
                // Distinguish stale (no pending approval) from real errors
                if msg.contains("no pending") || msg.contains("not found") {
                    ApprovalResolutionState::Stale {
                        reason: format!("Approval no longer pending: {msg}"),
                    }
                } else {
                    ApprovalResolutionState::Failed {
                        error: format!("Approval resolution failed: {msg}"),
                    }
                }
            }
        }
    }

    /// Export evidence audit packet from the desktop UI.
    ///
    /// This is the authority boundary for desktop-requested evidence export.
    /// The desktop constructs an `EvidenceExportRequest` with a workflow
    /// execution ID and desired output path. This method:
    /// 1. Validates the DTO
    /// 2. Resolves the output path against an allowed export root
    /// 3. Delegates to `export_audit_packet()` (existing exporter)
    /// 4. Computes SHA-256 checksum ONLY on the returned artifact path
    /// 5. Parses the exported JSON for record count and honesty flags
    ///
    /// The service may read ONLY the artifact path produced by the delegated
    /// export operation. It does not expose general file-read authority.
    /// The desktop UI does not read evidence files, assemble packets, or
    /// write export artifacts.
    pub fn export_evidence(
        request: &crate::ui::evidence_export_request::EvidenceExportRequest,
        store_root: &std::path::Path,
        export_root: &std::path::Path,
    ) -> crate::ui::evidence_export_request::EvidenceExportState {
        use crate::ui::evidence_export_request::EvidenceExportState;

        // Validate DTO before any delegation
        if let Err(e) = request.validate() {
            return EvidenceExportState::Failed { error: e };
        }

        // Resolve and validate output path against export_root
        let output_path = std::path::Path::new(&request.output_path);

        // If output_path is relative, resolve it under export_root
        let resolved_output = if output_path.is_absolute() {
            output_path.to_path_buf()
        } else {
            export_root.join(output_path)
        };

        // Create export root if it doesn't exist (before canonicalization)
        if !export_root.exists() {
            if let Err(e) = std::fs::create_dir_all(export_root) {
                return EvidenceExportState::Failed {
                    error: format!("Failed to create export root: {e}"),
                };
            }
        }

        // Canonicalize export_root after ensuring it exists
        let export_root_canon = export_root.canonicalize()
            .unwrap_or_else(|_| export_root.to_path_buf());

        // Canonicalize the resolved output parent
        let resolved_canon = match resolved_output.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // File doesn't exist yet — canonicalize parent and append filename
                match resolved_output.parent() {
                    Some(parent) => match parent.canonicalize() {
                        Ok(canon_parent) => canon_parent.join(
                            resolved_output.file_name().unwrap_or_default(),
                        ),
                        Err(_) => resolved_output.clone(),
                    },
                    None => resolved_output.clone(),
                }
            }
        };

        // Path containment check: reject traversal/symlink escape
        if !resolved_canon.starts_with(&export_root_canon) {
            return EvidenceExportState::Failed {
                error: format!(
                    "Output path escapes export root: {} not within {}",
                    resolved_canon.display(),
                    export_root_canon.display()
                ),
            };
        }

        // Delegate to existing exporter
        let workflow_execution_id = openwand_workflow::workflow_run::WorkflowExecutionId(
            request.workflow_execution_id.clone(),
        );

        let artifact_path = match crate::workflow_evidence_chain_inspector::export_audit_packet(
            store_root,
            &workflow_execution_id,
            &resolved_canon,
        ) {
            Ok(path) => path,
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("not found") || msg.contains("no workflow") || msg.contains("No such") {
                    return EvidenceExportState::Unavailable {
                        reason: format!("No evidence found for workflow run: {msg}"),
                    };
                }
                return EvidenceExportState::Failed {
                    error: format!("Export failed: {msg}"),
                };
            }
        };

        // Compute SHA-256 checksum ONLY on the returned artifact path
        let packet_hash = match std::fs::read(&artifact_path) {
            Ok(bytes) => {
                // Use blake3 which is already a dependency
                let hash = blake3::hash(&bytes);
                hash.to_hex()[..16].to_string()
            }
            Err(e) => {
                return EvidenceExportState::Failed {
                    error: format!("Failed to read exported artifact for checksum: {e}"),
                };
            }
        };

        // Parse exported JSON for record count and honesty flags
        let (record_count, certifies_external_truth, verifies_artifacts) =
            match std::fs::read_to_string(&artifact_path) {
                Ok(json) => {
                    let parsed: serde_json::Value = serde_json::from_str(&json)
                        .unwrap_or(serde_json::json!({}));
                    let count = parsed.get("records")
                        .and_then(|r| r.as_array())
                        .map(|r| r.len());
                    let certifies = parsed.get("certifies_external_truth")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let verifies = parsed.get("verifies_artifacts")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    (count, certifies, verifies)
                }
                Err(_) => (None, false, false),
            };

        EvidenceExportState::Exported {
            artifact_path: artifact_path.display().to_string(),
            record_count,
            packet_hash,
            certifies_external_truth,
            verifies_artifacts,
        }
    }
}
