//! UI session action adapters.
//!
//! Typed actions the UI can request, routed through existing
//! SessionRunner and UiSessionService APIs.
//!
//! These are thin adapters. They do NOT:
//! - Call LLM providers directly
//! - Execute tools directly
//! - Evaluate policy directly
//! - Write memory directly
//! - Append trace directly
//! - Mutate pending approval state directly

use crate::ui::run_dto::{UiRunState, UiRunStatus};
use crate::ui::service::UiSessionService;
use openwand_core::ApprovalRequestId;
use openwand_session::runner::{ApprovalDecision, ApprovalResolution, SessionRunner};
use openwand_session::config::RunConfig;
use openwand_core::mode::InteractionMode;
use openwand_llm::LlmTarget;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Typed action the UI can request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UiSessionAction {
    StartSession {
        provider: String,
        model: String,
        mode: String,
        working_directory: String,
    },
    SendUserMessage {
        text: String,
    },
    StopRun,
    ApprovePendingTool {
        approval_request_id: Option<String>,
        rationale: Option<String>,
    },
    RejectPendingTool {
        approval_request_id: Option<String>,
        rationale: Option<String>,
    },
    RefreshSession,
}

/// Result of a UI session action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UiSessionActionResult {
    Started {
        session_id: String,
    },
    MessageSent,
    RunStopped,
    ApprovalResolved {
        approved: bool,
        tool_name: String,
    },
    Refreshed,
    Error {
        message: String,
    },
}

/// Execute a session action.
///
/// Routes through existing SessionRunner/UiSessionService APIs only.
/// Never calls LLM, tools, policy, memory writers, or trace append directly.
pub async fn execute_session_action(
    action: UiSessionAction,
    runner: Option<Arc<SessionRunner>>,
    service: Option<&UiSessionService>,
    run_state: Option<Arc<std::sync::Mutex<UiRunState>>>,
    session_id: Option<&str>,
) -> UiSessionActionResult {
    match action {
        UiSessionAction::StartSession {
            provider,
            model,
            mode,
            working_directory,
        } => {
            // Start requires a runner. The UI layer assembles it externally
            // via session_runtime::build_session_runtime().
            // This action records the start intent.
            if let Some(state) = &run_state {
                let mut s = state.lock().unwrap();
                s.status = UiRunStatus::Starting;
            }
            UiSessionActionResult::Started {
                session_id: session_id.unwrap_or("unknown").to_string(),
            }
        }

        UiSessionAction::SendUserMessage { text } => {
            // Record user message in run state for transcript
            if let Some(state) = &run_state {
                let mut s = state.lock().unwrap();
                s.record_user_message(text.clone());
                s.status = UiRunStatus::Running;
            }
            // The actual run_turn call happens externally via UiSessionService::start_run
            UiSessionActionResult::MessageSent
        }

        UiSessionAction::StopRun => {
            // Cancellation is handled via the RunHandle's CancellationToken
            if let Some(state) = &run_state {
                let mut s = state.lock().unwrap();
                s.status = UiRunStatus::Cancelled;
            }
            UiSessionActionResult::RunStopped
        }

        UiSessionAction::ApprovePendingTool { approval_request_id, rationale: _ } => {
            let r = runner.as_ref();
            if r.is_none() {
                return UiSessionActionResult::Error {
                    message: "No active runner".into(),
                };
            }
            let r = r.unwrap();

            let decision = match approval_request_id {
                Some(arid) => ApprovalDecision::for_approval(
                    ApprovalRequestId(arid),
                    ApprovalResolution::Approve,
                ),
                None => ApprovalDecision::approve(),
            };

            let config = RunConfig {
                max_steps: 25,
                mode: InteractionMode::Conversational,
                working_directory: r.working_directory().to_string(),
                system_prompt: None,
                llm_target: None,
                memory_prompt_inputs: None,
                output_guard: None,
                capability_context: None,
            };

            match UiSessionService::resolve_approval(r, decision, config).await {
                Ok(result) => {
                    if let Some(state) = &run_state {
                        let mut s = state.lock().unwrap();
                        s.status = UiRunStatus::Running;
                        s.pending_approval = None;
                    }
                    UiSessionActionResult::ApprovalResolved {
                        approved: true,
                        tool_name: result.tool_name,
                    }
                }
                Err(e) => UiSessionActionResult::Error {
                    message: e.to_string(),
                },
            }
        }

        UiSessionAction::RejectPendingTool { approval_request_id, rationale } => {
            let r = runner.as_ref();
            if r.is_none() {
                return UiSessionActionResult::Error {
                    message: "No active runner".into(),
                };
            }
            let r = r.unwrap();

            let decision = match approval_request_id {
                Some(arid) => ApprovalDecision::for_approval(
                    ApprovalRequestId(arid),
                    ApprovalResolution::Reject { reason: rationale.clone() },
                ),
                None => ApprovalDecision::reject_with_reason(
                    rationale.unwrap_or_else(|| "User rejected".into()),
                ),
            };

            let config = RunConfig {
                max_steps: 25,
                mode: InteractionMode::Conversational,
                working_directory: r.working_directory().to_string(),
                system_prompt: None,
                llm_target: None,
                memory_prompt_inputs: None,
                output_guard: None,
                capability_context: None,
            };

            match UiSessionService::resolve_approval(r, decision, config).await {
                Ok(result) => {
                    if let Some(state) = &run_state {
                        let mut s = state.lock().unwrap();
                        s.pending_approval = None;
                    }
                    UiSessionActionResult::ApprovalResolved {
                        approved: false,
                        tool_name: result.tool_name,
                    }
                }
                Err(e) => UiSessionActionResult::Error {
                    message: e.to_string(),
                },
            }
        }

        UiSessionAction::RefreshSession => {
            // Read-only: refresh session view from trace projection
            if let (Some(svc), Some(sid)) = (service, session_id) {
                match svc.refresh_session(sid).await {
                    Ok(_) => UiSessionActionResult::Refreshed,
                    Err(e) => UiSessionActionResult::Error {
                        message: e.to_string(),
                    },
                }
            } else {
                UiSessionActionResult::Refreshed
            }
        }
    }
}
