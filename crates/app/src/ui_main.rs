//! OpenWand Desktop UI — main entry point.
//!
//! Run with: cargo run --bin openwand-ui --features desktop
//!
//! Ownership:
//! - ui_main.rs owns the desktop runtime loop: send handling, run polling/projection,
//!   cancellation state, active runner state, and signal mutation during a run.
//!   These are not render-shell responsibilities.
//! - ui/desktop_bootstrap.rs owns construction helpers: policy, path, service, memory.
//! - ui/console_shell.rs owns console loading/clearing.
//! - ui/inspector_shell.rs owns inspector loading/clearing.
//! - ui/session_shell.rs owns session/detail/memory/tool-event rendering.

use dioxus::prelude::*;
use dioxus_desktop::{Config, LogicalSize, WindowBuilder};
use openwand_app::ui::memory_dto::UiFilteredMemoryPanel;
use openwand_app::memory_coordinator::PromptInputProductionConfig;
use openwand_app::ui::run_dto::{UiRunEvent, UiRunState, UiRunStatus};
use openwand_app::ui::{CreateSessionRequest, UiSessionService, UiSessionSummary, UiSessionView};
use openwand_app::ui::workflow_run_request::{WorkflowRunRequest, WorkflowRunRequestState};
use openwand_app::ui::approval_resolution_request::{
    ApprovalDecisionDto, ApprovalResolutionRequest, ApprovalResolutionState,
};
use openwand_app::settings;
use openwand_core::SessionId;
use openwand_llm::LlmTarget;
use openwand_memory::prompt_assembly::MemoryPromptAssemblyInputs;
use openwand_memory::{MemoryReadStore, MemoryStore, SqliteMemoryStore};
use openwand_session::runner::SessionRunner;
use openwand_store::backends::sqlite::{SqliteStore, SqliteStoreConfig};
use openwand_store::SessionRegistryStore;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

fn main() {
    let desktop_cfg = Config::new().with_window(
        WindowBuilder::new()
            .with_title("OpenWand")
            .with_inner_size(LogicalSize::new(1100, 700)),
    );

    LaunchBuilder::new().with_cfg(desktop_cfg).launch(App);
}

// ── Shared State ──────────────────────────────────────────

static SESSION_LIST: GlobalSignal<Vec<UiSessionSummary>> = Signal::global(Vec::new);
static SELECTED_SESSION_ID: GlobalSignal<Option<String>> = Signal::global(|| None);
static CURRENT_SESSION: GlobalSignal<Option<UiSessionView>> = Signal::global(|| None);
static RUN_STATE: GlobalSignal<UiRunState> = Signal::global(UiRunState::default);
static STATUS_TEXT: GlobalSignal<String> = Signal::global(|| "Ready".into());
static MEMORY_PANEL: GlobalSignal<UiFilteredMemoryPanel> = Signal::global(UiFilteredMemoryPanel::empty);

/// Cached skills/goals readiness report for the selected session.
static SKILLS_GOALS_REPORT: GlobalSignal<Option<openwand_app::ui::skills_goals_state::SkillGoalReadinessReport>> = Signal::global(|| None);

/// Cached capability context preview state.
static CAPABILITY_PREVIEW: GlobalSignal<Option<openwand_app::ui::skills_goals_state::CapabilityPreviewState>> = Signal::global(|| None);

/// Active runner + handle for the selected session.
static ACTIVE_RUNNER: GlobalSignal<Option<ActiveRun>> = Signal::global(|| None);

/// Session-scoped cache for 02k memory prompt inputs.
/// Produced by coordinator after turn N, consumed by start_run at turn N+1.
/// Scoped by session_id and working_directory to prevent cross-session leakage.
#[derive(Debug, Clone)]
struct CachedMemoryPromptInputs {
    session_id: String,
    working_directory: std::path::PathBuf,
    inputs: MemoryPromptAssemblyInputs,
}

static MEMORY_PROMPT_INPUTS: GlobalSignal<Option<CachedMemoryPromptInputs>> =
    Signal::global(|| None);

pub struct ActiveRun {
    pub runner: Arc<SessionRunner>,
    pub cancellation: CancellationToken,
    pub state: Arc<std::sync::Mutex<UiRunState>>,
}

// ── App Init ──────────────────────────────────────────────

fn build_smoke_policy() -> openwand_policy::BuiltinPolicyEngine {
    openwand_app::ui::desktop_bootstrap::build_smoke_policy()
}

fn db_path() -> std::path::PathBuf {
    openwand_app::ui::desktop_bootstrap::db_path()
}

fn init_service() -> Arc<UiSessionService> {
    openwand_app::ui::desktop_bootstrap::init_service()
}

fn init_memory() -> Arc<SqliteMemoryStore> {
    openwand_app::ui::desktop_bootstrap::init_memory()
}

// ── Root Component ────────────────────────────────────────

fn App() -> Element {
    let service: Arc<UiSessionService> = use_hook(|| {
        let svc = init_service();
        if let Ok(sessions) = svc.list_sessions() {
            *SESSION_LIST.write() = sessions;
        }
        svc
    });
    let memory: Arc<SqliteMemoryStore> = use_hook(init_memory);

    rsx! {
        div { style: "display: flex; height: 100vh; font-family: system-ui; margin: 0;",

            // Left sidebar — sessions
            div {
                style: "width: 260px; min-width: 260px; background: #f7f7f7;
                        border-right: 1px solid #ddd; display: flex; flex-direction: column;",

                div { style: "padding: 12px 16px; border-bottom: 1px solid #ddd;
                              display: flex; justify-content: space-between; align-items: center;",
                    span { style: "font-weight: 600; font-size: 14px;", "Sessions" }
                    button {
                        style: "padding: 4px 10px; font-size: 12px; background: #4a90d9;
                                color: white; border: none; border-radius: 3px; cursor: pointer;",
                        onclick: {
                            let svc = service.clone();
                            move |_| {
                                let svc = svc.clone();
                                spawn(async move {
                                    let s = settings::load_settings();
                                    match svc.create_session(CreateSessionRequest {
                                        title: Some("New Session".into()),
                                        model: Some(settings::resolve_model(&s)),
                                        base_url: Some(settings::resolve_base_url(&s)),
                                        provider: Some(settings::resolve_provider(&s)),
                                        working_directory: Some(".".into()),
                                        interaction_mode: "direct".into(),
                                    }) {
                                        Ok(summary) => {
                                            let id = summary.session_id.clone();
                                            if let Ok(sessions) = svc.list_sessions() {
                                                *SESSION_LIST.write() = sessions;
                                            }
                                            if let Ok(view) = svc.open_session(&id).await {
                                                *CURRENT_SESSION.write() = Some(view);
                                            }
                                            *SELECTED_SESSION_ID.write() = Some(id);
                                            *STATUS_TEXT.write() = "Session created".into();
                                        }
                                        Err(e) => {
                                            *STATUS_TEXT.write() = format!("Error: {e}");
                                        }
                                    }
                                });
                            }
                        },
                        "+ New"
                    }
                }

                div { style: "flex: 1; overflow-y: auto;",
                    for session in SESSION_LIST.read().iter() {
                        { render_session_item(session, service.clone()) }
                    }
                    if SESSION_LIST.read().is_empty() {
                        div { style: "padding: 24px 16px; color: #999; font-size: 13px; text-align: center;",
                            "No sessions yet. Click \"+ New\" to create one."
                        }
                    }
                }
            }

            // Center — main content with tabs
            div { style: "flex: 1; display: flex; flex-direction: column; min-width: 0;",
                // Tab bar
                div { style: "display: flex; border-bottom: 1px solid #ddd; background: #f7f7f7;",
                    {{
                        let tab_session_active = (*ACTIVE_TAB.read()) == "session";
                        let tab_console_active = (*ACTIVE_TAB.read()) == "console";
                        let tab_inspector_active = (*ACTIVE_TAB.read()) == "inspector";
                        let session_bg = if tab_session_active { "#fff" } else { "#f7f7f7" };
                        let session_border = if tab_session_active { "#ddd #ddd #fff #ddd" } else { "transparent" };
                        let console_bg = if tab_console_active { "#fff" } else { "#f7f7f7" };
                        let console_border = if tab_console_active { "#ddd #ddd #fff #ddd" } else { "transparent" };
                        let inspector_bg = if tab_inspector_active { "#fff" } else { "#f7f7f7" };
                        let inspector_border = if tab_inspector_active { "#ddd #ddd #fff #ddd" } else { "transparent" };
                        rsx! {
                            button {
                                style: "padding: 8px 16px; font-size: 13px; font-weight: 600; border: 1px solid; \
                                         border-color: {session_border}; background: {session_bg}; cursor: pointer; \
                                         border-bottom: none; font-family: system-ui;",
                                onclick: move |_| {
                                    *ACTIVE_TAB.write() = "session".into();
                                },
                                "Session"
                            }
                            button {
                                style: "padding: 8px 16px; font-size: 13px; font-weight: 600; border: 1px solid; \
                                         border-color: {console_border}; background: {console_bg}; cursor: pointer; \
                                         border-bottom: none; font-family: system-ui;",
                                onclick: move |_| {
                                    *ACTIVE_TAB.write() = "console".into();
                                    // Load console state via shell
                                    if let Some(ref view) = *CURRENT_SESSION.read() {
                                        let session_id = view.summary.session_id.clone();
                                        let path = db_path();
                                        spawn(async move {
                                            openwand_app::ui::console_shell::load_console_shell(&CONSOLE_STATE, &path, &session_id);
                                            *STATUS_TEXT.write() = "Console loaded".into();
                                        });
                                    }
                                },
                                "Console"
                            }
                            button {
                                style: "padding: 8px 16px; font-size: 13px; font-weight: 600; border: 1px solid; \
                                         border-color: {inspector_border}; background: {inspector_bg}; cursor: pointer; \
                                         border-bottom: none; font-family: system-ui;",
                                onclick: move |_| {
                                    *ACTIVE_TAB.write() = "inspector".into();
                                    // Load inspector state via shell
                                    if let Some(ref view) = *CURRENT_SESSION.read() {
                                        let session_id = view.summary.session_id.clone();
                                        let path = db_path();
                                        spawn(async move {
                                            use openwand_workflow::workflow_run::WorkflowExecutionId;
                                            let wfx_id = WorkflowExecutionId(session_id.clone());
                                            let sigs = openwand_app::ui::inspector_shell::InspectorSignals {
                                                inspector_state: &INSPECTOR_STATE,
                                                review_rows: &REVIEW_ROWS,
                                                distribution_rows: &DISTRIBUTION_ROWS,
                                                ladder_result_rows: &LADDER_RESULT_ROWS,
                                                ladder_review_rows: &LADDER_REVIEW_ROWS,
                                                ladder_readiness_rows: &LADDER_READINESS_ROWS,
                                                ladder_gate_rows: &LADDER_GATE_ROWS,
                                                ladder_predicates: &LADDER_PREDICATES,
                                                routing_route_row: &ROUTING_ROUTE_ROW,
                                                routing_session_row: &ROUTING_SESSION_ROW,
                                                routing_route_predicates: &ROUTING_ROUTE_PREDICATES,
                                                routing_route_prompt: &ROUTING_ROUTE_PROMPT,
                                                routing_readiness_state: &ROUTING_READINESS_STATE,
                                                routing_next_action_state: &ROUTING_NEXT_ACTION_ROUTING_STATE,
                                                routing_review_row: &ROUTING_REVIEW_ROW,
                                                execution_timeline_state: &EXECUTION_TIMELINE_STATE,
                                                proposal_state: &PROPOSAL_STATE,
                                                readiness_state: &READINESS_STATE,
                                                outcome_state: &OUTCOME_STATE,
                                                reconciliation_state: &RECONCILIATION_STATE,
                                                loop_controller_state: &LOOP_CONTROLLER_STATE,
                                            };
                                            sigs.load_inspector_shell(&path, &wfx_id);
                                            *STATUS_TEXT.write() = "Inspector loaded".into();
                                        });
                                    }
                                },
                                "Inspector"
                            }
                        }
                    }}
                }
                // Tab content
                if (*ACTIVE_TAB.read()) == "console" {
                    { render_console_pane() }
                } else if (*ACTIVE_TAB.read()) == "inspector" {
                    { render_inspector_pane() }
                } else {
                    { render_detail_pane(service.clone(), memory.clone()) }
                }
            }

            // Right sidebar — memory panel
            div {
                style: "width: 240px; min-width: 240px; background: #fafafa;
                        border-left: 1px solid #ddd; display: flex; flex-direction: column;",

                div { style: "padding: 12px 16px; border-bottom: 1px solid #ddd;",
                    span { style: "font-weight: 600; font-size: 14px;", "Memory" }
                    span { style: "font-size: 11px; color: #888; margin-left: 8px;",
                        "{MEMORY_PANEL.read().summary.prompt_included} trusted"
                    }
                }

                div { style: "flex: 1; overflow-y: auto;",
                    { render_memory_buckets(&MEMORY_PANEL.read()) }
                    // Skills & Goals readiness panel (Patch 6: reachable UI)
                    if let Some(ref report) = *SKILLS_GOALS_REPORT.read() {
                        { openwand_app::ui::skills_goals_components::render_skills_goals_readiness_panel(report) }
                    }
                    // Capability context preview (Wave 64A)
                    if let Some(ref preview) = *CAPABILITY_PREVIEW.read() {
                        { openwand_app::ui::skills_goals_components::render_capability_context_preview(preview) }
                    }
                }
            }
        }
    }
}

fn render_session_item(session: &UiSessionSummary, service: Arc<UiSessionService>) -> Element {
    let id = session.session_id.clone();
    let title = session.title.clone().unwrap_or_else(|| "Untitled".into());
    let model = session.model.clone().unwrap_or_else(|| "No model".into());
    let status = session.status.clone();
    let selected = SELECTED_SESSION_ID.read().as_deref() == Some(id.as_str());
    let bg = if selected { "#e0e8f0" } else { "transparent" };

    rsx! {
        div {
            key: "{id}",
            style: "padding: 10px 16px; cursor: pointer; background: {bg};
                    border-bottom: 1px solid #eee;",
            onclick: {
                let svc = service.clone();
                move |_| {
                    let id = id.clone();
                    let svc = svc.clone();
                    spawn(async move {
                        *SELECTED_SESSION_ID.write() = Some(id.clone());
                        *MEMORY_PROMPT_INPUTS.write() = None; // Clear on session switch
                        // Clear console/inspector shells via shell modules
                        openwand_app::ui::console_shell::clear_console_shell(&CONSOLE_STATE);
                        let sigs = openwand_app::ui::inspector_shell::InspectorSignals {
                            inspector_state: &INSPECTOR_STATE,
                            review_rows: &REVIEW_ROWS,
                            distribution_rows: &DISTRIBUTION_ROWS,
                            ladder_result_rows: &LADDER_RESULT_ROWS,
                            ladder_review_rows: &LADDER_REVIEW_ROWS,
                            ladder_readiness_rows: &LADDER_READINESS_ROWS,
                            ladder_gate_rows: &LADDER_GATE_ROWS,
                            ladder_predicates: &LADDER_PREDICATES,
                            routing_route_row: &ROUTING_ROUTE_ROW,
                            routing_session_row: &ROUTING_SESSION_ROW,
                            routing_route_predicates: &ROUTING_ROUTE_PREDICATES,
                            routing_route_prompt: &ROUTING_ROUTE_PROMPT,
                            routing_readiness_state: &ROUTING_READINESS_STATE,
                            routing_next_action_state: &ROUTING_NEXT_ACTION_ROUTING_STATE,
                            routing_review_row: &ROUTING_REVIEW_ROW,
                            execution_timeline_state: &EXECUTION_TIMELINE_STATE,
                            proposal_state: &PROPOSAL_STATE,
                            readiness_state: &READINESS_STATE,
                            outcome_state: &OUTCOME_STATE,
                            reconciliation_state: &RECONCILIATION_STATE,
                            loop_controller_state: &LOOP_CONTROLLER_STATE,
                        };
                        sigs.clear_inspector_shell();
                        match svc.open_session(&id).await {
                            Ok(view) => {
                                *CURRENT_SESSION.write() = Some(view);
                                // Load skills/goals readiness for the session's working directory
                                let working_dir = CURRENT_SESSION
                                    .read()
                                    .as_ref()
                                    .and_then(|s| s.working_directory.clone())
                                    .unwrap_or_else(|| ".".to_string());
                                let openwand_dir = std::path::Path::new(&working_dir).join(".openwand");
                                let sr = openwand_skills::registry::load_skill_registry(&openwand_dir.join("skills.toml"));
                                let gr = openwand_goals::registry::load_goal_registry(&openwand_dir.join("goals.toml"));
                                let report = openwand_app::ui::skills_goals_state::build_readiness_report(&sr, &gr);
                                *SKILLS_GOALS_REPORT.write() = Some(report.clone());
                                // Build preview of what would be sent (Patch 3: WouldSend mode)
                                let cap_block = openwand_app::session_capability_prompt::build_capability_prompt_inputs(&sr, &gr);
                                let preview = openwand_app::ui::skills_goals_state::build_capability_preview(
                                    &cap_block,
                                    &report,
                                    openwand_app::ui::skills_goals_state::CapabilityPreviewMode::WouldSend,
                                );
                                *CAPABILITY_PREVIEW.write() = Some(preview);
                            }
                            Err(e) => {
                                *STATUS_TEXT.write() = format!("Error: {e}");
                            }
                        }
                    });
                }
            },
            div { style: "font-size: 13px; font-weight: 500; color: #333;",
                "{title}"
            }
            div { style: "font-size: 11px; color: #888; margin-top: 3px;",
                "{model} - {status}"
            }
        }
    }
}

fn render_memory_buckets(panel: &openwand_app::ui::memory_dto::UiFilteredMemoryPanel) -> Element {
    openwand_app::ui::session_shell::render_memory_buckets(panel)
}

fn render_bucket(title: &str, color: &str, rows: &[openwand_app::ui::memory_dto::UiMemoryPanelRow]) -> Element {
    openwand_app::ui::session_shell::render_bucket(title, color, rows)
}

fn render_conflicts(title: &str, color: &str, conflicts: &[openwand_app::ui::memory_dto::UiMemoryPanelConflict]) -> Element {
    openwand_app::ui::session_shell::render_conflicts(title, color, conflicts)
}

// ── Console Pane ──────────────────────────────────────────

fn render_console_pane() -> Element {
    use openwand_app::ui::workflow_operator_console_components::*;

    let console_state = CONSOLE_STATE.read().clone();
    match console_state {
        Some(state) => render_operator_console(&state),
        None => render_operator_console_empty_state(),
    }
}

// ── Inspector Pane ─────────────────────────────────────────

fn render_inspector_pane() -> Element {
    use openwand_app::ui::workflow_evidence_chain_inspector_components::*;
    
    use openwand_app::ui::workflow_audit_packet_distribution_components::*;
    use openwand_app::ui::workflow_manual_result_components::render_manual_result_ladder_panel;
    use openwand_app::ui::workflow_action_routing_components::*;
    use openwand_app::ui::workflow_routing_readiness_components::*;
    use openwand_app::ui::workflow_next_action_routing_components::*;
    use openwand_app::ui::workflow_next_action_review_components::*;
    use openwand_app::ui::workflow_execution_components::*;
    use openwand_app::ui::workflow_proposal_components::*;
    use openwand_app::ui::workflow_readiness_components::*;
    use openwand_app::ui::workflow_action_outcome_components::*;
    use openwand_app::ui::workflow_reconciliation_components::*;
    use openwand_app::ui::workflow_loop_controller_components::*;

    let inspector_state = INSPECTOR_STATE.read().clone();
    let reviews = REVIEW_ROWS.read().clone();
    let distributions = DISTRIBUTION_ROWS.read().clone();
    let ladder_results = LADDER_RESULT_ROWS.read().clone();
    let ladder_reviews = LADDER_REVIEW_ROWS.read().clone();
    let ladder_readiness = LADDER_READINESS_ROWS.read().clone();
    let ladder_gates = LADDER_GATE_ROWS.read().clone();
    let ladder_preds = LADDER_PREDICATES.read().clone();
    let route_row = ROUTING_ROUTE_ROW.read().clone();
    let session_row = ROUTING_SESSION_ROW.read().clone();
    let route_preds = ROUTING_ROUTE_PREDICATES.read().clone();
    let route_prompt = ROUTING_ROUTE_PROMPT.read().clone();
    let routing_readiness = ROUTING_READINESS_STATE.read().clone();
    let routing_next_action = ROUTING_NEXT_ACTION_ROUTING_STATE.read().clone();
    let routing_review = ROUTING_REVIEW_ROW.read().clone();
    let execution_timeline = EXECUTION_TIMELINE_STATE.read().clone();
    let proposal_state = PROPOSAL_STATE.read().clone();
    let readiness_state = READINESS_STATE.read().clone();
    let outcome_state = OUTCOME_STATE.read().clone();
    let reconciliation_state = RECONCILIATION_STATE.read().clone();
    let loop_controller_state = LOOP_CONTROLLER_STATE.read().clone();
    let wfx_id = CURRENT_SESSION.read().as_ref().map(|v| v.summary.session_id.clone()).unwrap_or_default();

    match inspector_state {
        Some(state) => rsx! {
            div { style: "flex: 1; display: flex; flex-direction: column; min-width: 0; overflow-y: auto;",
                { render_evidence_chain_inspector(&state) }
                { render_audit_packet_review_distribution_panel(&reviews, &distributions) }
                { render_manual_result_ladder_panel(&ladder_results, &ladder_reviews, &ladder_readiness, &ladder_gates, &ladder_preds, &wfx_id) }
                // Routing ladder
                if let Some(ref route) = route_row {
                    { render_route_summary(route) }
                }
                if let Some(ref session) = session_row {
                    { render_session_route(session) }
                }
                if !route_preds.is_empty() {
                    { render_route_predicate_rows(&route_preds) }
                }
                if let Some(ref prompt) = route_prompt {
                    { render_route_prompt(prompt) }
                }
                if let Some(ref rdy) = routing_readiness {
                    { render_routing_readiness_panel(rdy) }
                }
                if let Some(ref nar) = routing_next_action {
                    { render_next_action_routing_panel(nar) }
                }
                if let Some(ref rev) = routing_review {
                    { render_next_action_review_summary(rev) }
                }
                // Execution timeline
                if let Some(ref timeline) = execution_timeline {
                    { render_workflow_execution_timeline(timeline) }
                }
                // Workflow proposal (live data — Wave 84A)
                if let Some(ref proposal) = proposal_state {
                    { render_proposal_panel(proposal) }
                }
                // Workflow readiness (live data — Wave 84A)
                if let Some(ref readiness) = readiness_state {
                    { render_workflow_readiness_panel(readiness) }
                }
                // Workflow action outcome (live data — Wave 84B)
                if let Some(ref outcome) = outcome_state {
                    { render_action_outcome_panel(outcome) }
                }
                // Workflow reconciliation (live data — Wave 84C)
                if let Some(ref recon) = reconciliation_state {
                    { render_reconciliation_panel(recon) }
                }
                // Workflow loop controller (live data — Wave 84C)
                if let Some(ref loop_ctrl) = loop_controller_state {
                    { render_loop_controller_panel(loop_ctrl) }
                }
                // Workflow run initiation (Wave 88A — delegated authority)
                { render_workflow_run_initiation(service.clone(), &proposal_state, &readiness_state) }
            }
        },
        None => render_inspector_empty_state(),
    }
}

// ── Workflow Run Initiation (Wave 88A) ─────────────────────────────────
/// Renders the workflow run initiation button and request state.
/// The button emits a request through UiSessionService::request_workflow_run().
/// The UI never imports backend execution gates. Authority is delegated.
#[cfg(feature = "desktop")]
fn render_workflow_run_initiation(
    service: Arc<UiSessionService>,
    proposal_state: &Option<openwand_app::ui::workflow_proposal_state::WorkflowProposalUiState>,
    readiness_state: &Option<openwand_app::ui::workflow_readiness_state::WorkflowReadinessUiState>,
) -> Element {
    use openwand_app::ui::design_tokens::*;
    use dioxus::prelude::*;

    let req_state = WORKFLOW_RUN_REQUEST_STATE.read().clone();

    // Only show if both proposal and readiness data are loaded
    let can_request = proposal_state.is_some() && readiness_state.is_some() && !req_state.is_terminal();

    // Extract IDs for the request
    let proposal_id = proposal_state.as_ref()
        .and_then(|p| p.latest_proposal.as_ref())
        .map(|p| p.proposal_id.clone());
    let readiness_id = readiness_state.as_ref()
        .and_then(|r| r.latest_readiness.as_ref())
        .map(|r| r.readiness_id.clone());
    let review_id = proposal_state.as_ref()
        .and_then(|p| p.latest_review.as_ref())
        .map(|r| r.review_id.clone());

    let has_ids = proposal_id.is_some() && readiness_id.is_some() && review_id.is_some();

    let section_style = format!(
        "padding: {} {}; border-top: 1px solid {}; margin-top: {};",
        spacing::SPACE_MD, spacing::SPACE_SM, colors::BORDER_LIGHT, spacing::SPACE_MD,
    );
    let label_style = format!(
        "font-size: {}; font-weight: 600; color: {}; margin-bottom: {};",
        typo::TEXT_BASE, colors::TEXT_STRONG, spacing::SPACE_SM,
    );

    // Status text
    let status_text = req_state.status_label().to_string();
    let status_color = match &*req_state {
        WorkflowRunRequestState::Created { .. } => colors::STATUS_SUCCESS,
        WorkflowRunRequestState::Blocked { .. } => colors::STATUS_WARN,
        WorkflowRunRequestState::Failed { .. } => colors::STATUS_ERROR,
        WorkflowRunRequestState::Pending => colors::TEXT_MUTED,
        WorkflowRunRequestState::Idle => colors::TEXT_FAINT,
    };

    let btn_style = format!(
        "padding: {} {}; background: {}; color: white; border: none; border-radius: {}; cursor: {}; font-size: {};",
        spacing::SPACE_SM, spacing::SPACE_LG, colors::PRIMARY, radius::RADIUS_SM, typo::TEXT_SM,
    );
    let btn_disabled_style = format!(
        "padding: {} {}; background: {}; color: white; border: none; border-radius: {}; cursor: not-allowed; font-size: {};",
        spacing::SPACE_SM, spacing::SPACE_LG, colors::DISABLED_BG, radius::RADIUS_SM, typo::TEXT_SM,
    );

    // Detail for terminal states
    let detail_text = match &*req_state {
        WorkflowRunRequestState::Created { execution_id, status, stage_count, predicates_passed, predicates_total } => {
            format!("Run {} — {} — {} stages — {}/{} predicates passed", execution_id, status, stage_count, predicates_passed, predicates_total)
        }
        WorkflowRunRequestState::Blocked { reason } => format!("Blocked: {}", reason),
        WorkflowRunRequestState::Failed { error } => format!("Error: {}", error),
        _ => String::new(),
    };

    rsx! {
        div { style: "{section_style}",
            div { style: "{label_style}", "Workflow Run" }

            if can_request && has_ids {
                button {
                    style: "{btn_style}",
                    onclick: move |_| {
                        *WORKFLOW_RUN_REQUEST_STATE.write() = WorkflowRunRequestState::Pending;
                        let pid = proposal_id.clone().unwrap();
                        let rid = readiness_id.clone().unwrap();
                        let rvid = review_id.clone().unwrap();
                        let svc = service.clone();
                        spawn(async move {
                            let req = WorkflowRunRequest {
                                readiness_id: rid,
                                proposal_id: pid,
                                proposal_review_id: rvid,
                                idempotency_key: format!("desktop_{}", chrono::Utc::now().timestamp()),
                                requested_by: "desktop".into(),
                            };
                            let working_dir = CURRENT_SESSION
                                .read()
                                .as_ref()
                                .and_then(|s| s.working_directory.clone())
                                .unwrap_or_else(|| ".".to_string());
                            let store_root = std::path::PathBuf::from(&working_dir);
                            let result = svc.request_workflow_run(&req, &store_root);
                            *WORKFLOW_RUN_REQUEST_STATE.write() = result;
                        });
                    },
                    "Initiate Workflow Run"
                }
            } else if !has_ids && req_state.is_terminal() == false {
                div { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_FAINT};",
                    "Load a workflow run with proposal and readiness data to initiate"
                }
            } else {
                button {
                    style: "{btn_disabled_style}",
                    disabled: true,
                    "Initiate Workflow Run"
                }
            }

            // Status display
                        div { style: "margin-top: {spacing::SPACE_SM}; font-size: {typo::TEXT_SM}; color: {status_color};",
                "{status_text}"
            }
            if !detail_text.is_empty() {
                div { style: "margin-top: {spacing::SPACE_XS}; font-size: {typo::TEXT_XS}; color: {colors::TEXT_MUTED};",
                    "{detail_text}"
                }
            }
        }
    }
}

// ── Approval Resolution (Wave 88B) ────────────────────────
/// Renders the approval resolution UI when a tool approval is pending.
/// Shows approve/reject buttons that submit decisions through the
/// existing UiSessionService approval governance path.
/// The UI never directly calls resolve_approval, resumes execution,
/// or mutates approval records.
#[cfg(feature = "desktop")]
fn render_approval_resolution(
    run_state: &UiRunState,
    active_runner: &Option<ActiveRun>,
) -> Element {
    use openwand_app::ui::design_tokens::*;

    // Only render when waiting for approval and have pending approval info
    if run_state.status != UiRunStatus::WaitingForApproval {
        return rsx! {};
    }
    let pending = match &run_state.pending_approval {
        Some(p) => p,
        None => return rsx! {},
    };

    let resolution_state = APPROVAL_RESOLUTION_STATE.read().clone();
    let tool_name = pending.tool_name.clone();
    let tool_call_id = pending.tool_call_id.clone();
    let reason = pending.reason.clone();

    // Status display
    let status_label = resolution_state.status_label();
    let status_color = match &resolution_state {
        ApprovalResolutionState::Resolved { decision, .. } => {
            if *decision == ApprovalDecisionDto::Approve { colors::STATUS_SUCCESS }
            else { colors::STATUS_WARN }
        }
        ApprovalResolutionState::Failed { .. } => colors::STATUS_ERROR,
        ApprovalResolutionState::Stale { .. } => colors::TEXT_MUTED,
        ApprovalResolutionState::Pending => colors::TEXT_MUTED,
        ApprovalResolutionState::Idle => colors::TEXT_STRONG,
    };

    // Detail for resolved state
    let detail = match &resolution_state {
        ApprovalResolutionState::Resolved { tool_name, tool_status, source, .. } => {
            format!("{} — {} — via {}",
                tool_name.as_deref().unwrap_or("unknown"),
                tool_status.as_deref().unwrap_or("no status"),
                source,
            )
        }
        ApprovalResolutionState::Failed { error } => format!("Error: {}", error),
        ApprovalResolutionState::Stale { reason } => format!("Stale: {}", reason),
        _ => String::new(),
    };

    let is_resolving = resolution_state.is_pending();
    let card_bg = format!("padding: {} {}; background: {}; border: 1px solid {}; border-radius: {}; margin: {} {};",
        spacing::SPACE_MD, spacing::SPACE_MD,
        colors::BG_WARN, colors::BORDER_WARN, radius::RADIUS_MD,
        spacing::SPACE_SM, spacing::SPACE_MD,
    );
    let title_style = format!("font-size: {}; font-weight: 600; color: {}; margin-bottom: {};",
        typo::TEXT_BASE, colors::TEXT_STRONG, spacing::SPACE_XS);
    let info_style = format!("font-size: {}; color: {}; margin-bottom: {};",
        typo::TEXT_SM, colors::TEXT_BODY, spacing::SPACE_SM);
    let reason_style = format!("font-size: {}; color: {}; font-style: italic; margin-bottom: {};",
        typo::TEXT_XS, colors::TEXT_MUTED, spacing::SPACE_SM);
    let btn_approve_style = format!("padding: {} {}; background: {}; color: white; border: none; border-radius: {}; cursor: {}; font-size: {};",
        spacing::SPACE_SM, spacing::SPACE_LG, colors::STATUS_SUCCESS,
        radius::RADIUS_SM, if is_resolving { "not-allowed" } else { "pointer" }, typo::TEXT_SM);
    let btn_reject_style = format!("padding: {} {}; background: {}; color: white; border: none; border-radius: {}; cursor: {}; font-size: {};",
        spacing::SPACE_SM, spacing::SPACE_LG, colors::STATUS_ERROR,
        radius::RADIUS_SM, if is_resolving { "not-allowed" } else { "pointer" }, typo::TEXT_SM);
    let status_style = format!("margin-top: {}; font-size: {}; color: {};",
        spacing::SPACE_SM, typo::TEXT_SM, status_color);
    let detail_style = format!("margin-top: {}; font-size: {}; color: {};",
        spacing::SPACE_XS, typo::TEXT_XS, colors::TEXT_MUTED);

    // Build ARID from tool_call_id for the explicit binding
    // The pending approval state has tool_call_id; the runner's approval
    // recovery index maps this to the ARID.
    let arid = tool_call_id.clone();

    rsx! {
        div { style: "{card_bg}",
            div { style: "{title_style}", "Approval Required" }
            div { style: "{info_style}",
                "Tool: {tool_name}"
            }
            if !reason.is_empty() {
                div { style: "{reason_style}", "{reason}" }
            }

                        if !resolution_state.is_terminal() {
                div { style: "display: flex; gap: {spacing::SPACE_SM};",
                    button {
                        style: "{btn_approve_style}",
                        disabled: "{is_resolving}",
                        onclick: move |_| {
                            *APPROVAL_RESOLUTION_STATE.write() = ApprovalResolutionState::Pending;
                            let req = ApprovalResolutionRequest {
                                approval_request_id: arid.clone(),
                                displayed_tool_name: Some(tool_name.clone()),
                                decision: ApprovalDecisionDto::Approve,
                                rationale: None,
                                resolved_by: "desktop".into(),
                                idempotency_key: format!("desktop_approve_{}", chrono::Utc::now().timestamp()),
                            };
                            let runner_opt = active_runner.as_ref().map(|r| r.runner.clone());
                            spawn(async move {
                                let result = UiSessionService::submit_approval_resolution(
                                    runner_opt.as_deref(),
                                    &req,
                                ).await;
                                *APPROVAL_RESOLUTION_STATE.write() = result;
                            });
                        },
                        "Approve"
                    }
                    button {
                        style: "{btn_reject_style}",
                        disabled: "{is_resolving}",
                        onclick: move |_| {
                            *APPROVAL_RESOLUTION_STATE.write() = ApprovalResolutionState::Pending;
                            let req = ApprovalResolutionRequest {
                                approval_request_id: arid.clone(),
                                displayed_tool_name: Some(tool_name.clone()),
                                decision: ApprovalDecisionDto::Reject,
                                rationale: Some("Rejected via desktop".into()),
                                resolved_by: "desktop".into(),
                                idempotency_key: format!("desktop_reject_{}", chrono::Utc::now().timestamp()),
                            };
                            let runner_opt = active_runner.as_ref().map(|r| r.runner.clone());
                            spawn(async move {
                                let result = UiSessionService::submit_approval_resolution(
                                    runner_opt.as_deref(),
                                    &req,
                                ).await;
                                *APPROVAL_RESOLUTION_STATE.write() = result;
                            });
                        },
                        "Reject"
                    }
                }
            }

            // Status display
            div { style: "{status_style}", "{status_label}" }
            if !detail.is_empty() {
                div { style: "{detail_style}", "{detail}" }
            }
        }
    }
}

// ── Detail Pane ───────────────────────────────────────────

fn render_detail_pane(service: Arc<UiSessionService>, memory: Arc<SqliteMemoryStore>) -> Element {
    let current = CURRENT_SESSION.read().clone();
    let run_state = RUN_STATE.read().clone();

    match current {
        Some(view) => {
            let is_running = run_state.status == UiRunStatus::Running;
            let session_id = view.summary.session_id.clone();

            rsx! {
                // Header
                div { style: "padding: 16px 20px; border-bottom: 1px solid #eee;
                              display: flex; justify-content: space-between; align-items: center;",
                    div {
                        h2 { style: "margin: 0 0 2px 0; font-size: 16px;",
                            {view.summary.title.as_deref().unwrap_or("Untitled")}
                        }
                        div { style: "font-size: 11px; color: #888;",
                            "{session_id}"
                        }
                    }
                    if is_running {
                        div { style: "padding: 4px 10px; background: #e8f4e8; border: 1px solid #a0c8a0;
                                      border-radius: 12px; font-size: 11px; font-weight: 600; color: #2d6a2d;",
                            {run_state.phase.as_deref().unwrap_or("Running")}
                            " (step {run_state.step})"
                        }
                    }
                }

                // Messages area
                div { style: "flex: 1; overflow-y: auto; padding: 16px 20px; background: #fff;",
                    if !run_state.streamed_text.is_empty() {
                        div { style: "margin-bottom: 12px; padding: 10px 14px; background: #f0f0f0;
                                     border: 1px solid #ddd; border-radius: 6px;",
                            div { style: "font-size: 11px; font-weight: 600; color: #888; margin-bottom: 4px;",
                                "Assistant"
                            }
                            div { style: "font-size: 13px; color: #333; white-space: pre-wrap;",
                                "{run_state.streamed_text}"
                                if is_running {
                                    span { style: "color: #4a90d9;", "▍" }
                                }
                            }
                        }
                    }

                    for event in run_state.tool_events.iter() {
                        { render_tool_event(event.clone()) }
                    }

                    if run_state.streamed_text.is_empty() && run_state.tool_events.is_empty() && !is_running {
                        div { style: "color: #999; font-size: 14px; text-align: center; margin-top: 40px;",
                            "Type a message below to start"
                        }
                    }

                    if let Some(ref err) = run_state.error {
                        div { style: "margin-top: 12px; padding: 10px 14px; background: #fde8e8;
                                     border: 1px solid #e8a0a0; border-radius: 6px; color: #cc3333;
                                     font-size: 13px;",
                            "Error: {err}"
                        }
                    }
                }

                // Approval resolution area (only visible when WaitingForApproval)
                { render_approval_resolution(&run_state, &*ACTIVE_RUNNER.read()) }

                // Input area
                div { style: "padding: 12px 20px; border-top: 1px solid #eee; background: #fafafa;
                              display: flex; gap: 8px; align-items: flex-end;",
                    {
                        let svc = service.clone();
                        let mem = memory.clone();
                        let sid = session_id.clone();
                        rsx! {
                            textarea {
                                style: "flex: 1; padding: 8px 12px; font-size: 13px; border: 1px solid #ddd;
                                        border-radius: 4px; resize: none; font-family: system-ui;
                                        min-height: 36px; max-height: 120px;",
                                rows: "1",
                                placeholder: if is_running { "Running..." } else { "Type a message..." },
                                disabled: is_running,
                                onchange: {
                                    move |e: FormEvent| {
                                        *INPUT_TEXT.write() = e.value().clone();
                                    }
                                }
                            }
                            button {
                                style: if is_running {
                                    "padding: 8px 16px; font-size: 13px; background: #ccc; color: white;
                                     border: none; border-radius: 4px; cursor: not-allowed;"
                                } else {
                                    "padding: 8px 16px; font-size: 13px; background: #4a90d9; color: white;
                                     border: none; border-radius: 4px; cursor: pointer;"
                                },
                                disabled: is_running,
                                onclick: {
                                    let svc = svc.clone();
                                    let mem = mem.clone();
                                    let sid = sid.clone();
                                    move |_| {
                                        let svc = svc.clone();
                                        let mem = mem.clone();
                                        let sid = sid.clone();
                                        let text = INPUT_TEXT.read().clone();
                                        if text.is_empty() { return; }
                                        *INPUT_TEXT.write() = String::new();
                                        spawn(async move {
                                            handle_send(svc, mem, sid, text).await;
                                        });
                                    }
                                },
                                if is_running { "Running..." } else { "Send" }
                            }
                            if is_running {
                                button {
                                    style: "padding: 8px 12px; font-size: 13px; background: #d94a4a;
                                            color: white; border: none; border-radius: 4px; cursor: pointer;",
                                    onclick: move |_| {
                                        if let Some(ref run) = *ACTIVE_RUNNER.read() {
                                            run.cancellation.cancel();
                                        }
                                        *STATUS_TEXT.write() = "Run cancelled".into();
                                    },
                                    "Cancel"
                                }
                            }
                        }
                    }
                }

                // Status bar
                div { style: "padding: 4px 16px; background: #f0f0f0; border-top: 1px solid #ddd;
                              font-size: 11px; color: #888;",
                    "{STATUS_TEXT}"
                }
            }
        }
        None => rsx! {
            div { style: "flex: 1; display: flex; align-items: center; justify-content: center;
                         color: #bbb; font-size: 15px;",
                "Select a session or create a new one"
            }
        },
    }
}

fn render_tool_event(event: UiRunEvent) -> Element {
    openwand_app::ui::session_shell::render_tool_event(event)
}

// ── Input Text ────────────────────────────────────────────

static INPUT_TEXT: GlobalSignal<String> = Signal::global(String::new);

/// Active view tab: "session" or "console"
static ACTIVE_TAB: GlobalSignal<String> = Signal::global(|| "session".into());

/// Cached operator console state for the selected workflow run.
static CONSOLE_STATE: GlobalSignal<Option<openwand_workflow::workflow_operator_console::WorkflowOperatorConsoleState>> = Signal::global(|| None);

/// Cached evidence chain inspector state for the selected workflow run.
static INSPECTOR_STATE: GlobalSignal<Option<openwand_workflow::workflow_evidence_chain_inspector::EvidenceChainInspectionState>> = Signal::global(|| None);

/// Cached audit packet review summaries for the Inspector tab.
static REVIEW_ROWS: GlobalSignal<Vec<openwand_app::ui::workflow_audit_packet_review_state::ReviewSummaryRow>> = Signal::global(Vec::new);

/// Cached audit packet distribution summaries for the Inspector tab.
static DISTRIBUTION_ROWS: GlobalSignal<Vec<openwand_app::ui::workflow_audit_packet_distribution_state::DistributionSummaryRow>> = Signal::global(Vec::new);

/// Cached manual result ladder rows for the Inspector tab.
static LADDER_RESULT_ROWS: GlobalSignal<Vec<openwand_app::ui::workflow_manual_result_state::WorkflowManualResultSummaryRow>> = Signal::global(Vec::new);
static LADDER_REVIEW_ROWS: GlobalSignal<Vec<openwand_app::ui::workflow_manual_result_review_state::WorkflowManualResultReviewSummaryRow>> = Signal::global(Vec::new);
static LADDER_READINESS_ROWS: GlobalSignal<Vec<openwand_app::ui::workflow_manual_result_reconciliation_readiness_state::WorkflowManualResultReconciliationReadinessSummaryRow>> = Signal::global(Vec::new);
static LADDER_GATE_ROWS: GlobalSignal<Vec<openwand_app::ui::workflow_manual_result_reconciliation_gate_state::WorkflowManualResultReconciliationGateSummaryRow>> = Signal::global(Vec::new);
static LADDER_PREDICATES: GlobalSignal<Vec<openwand_app::ui::workflow_manual_result_reconciliation_readiness_state::ReadinessPredicateDisplayRow>> = Signal::global(Vec::new);

/// Cached routing ladder state for the Inspector tab.
static ROUTING_ROUTE_ROW: GlobalSignal<Option<openwand_app::ui::workflow_action_routing_state::WorkflowActionRouteSummaryRow>> = Signal::global(|| None);
static ROUTING_SESSION_ROW: GlobalSignal<Option<openwand_app::ui::workflow_action_routing_state::WorkflowSessionRouteRow>> = Signal::global(|| None);
static ROUTING_ROUTE_PREDICATES: GlobalSignal<Vec<openwand_app::ui::workflow_action_routing_state::WorkflowActionRoutePredicateRow>> = Signal::global(Vec::new);
static ROUTING_ROUTE_PROMPT: GlobalSignal<Option<openwand_app::ui::workflow_action_routing_state::WorkflowActionRoutePromptRow>> = Signal::global(|| None);
static ROUTING_READINESS_STATE: GlobalSignal<Option<openwand_app::ui::workflow_routing_readiness_state::WorkflowRoutingReadinessUiState>> = Signal::global(|| None);
static ROUTING_NEXT_ACTION_ROUTING_STATE: GlobalSignal<Option<openwand_app::ui::workflow_next_action_routing_state::WorkflowNextActionRoutingUiState>> = Signal::global(|| None);
static ROUTING_REVIEW_ROW: GlobalSignal<Option<openwand_app::ui::workflow_next_action_review_state::ReviewSummaryRow>> = Signal::global(|| None);

/// Cached workflow execution timeline state for the Inspector tab.
static EXECUTION_TIMELINE_STATE: GlobalSignal<Option<openwand_app::ui::workflow_execution_state::WorkflowExecutionUiState>> = Signal::global(|| None);

// Workflow proposal + readiness states (Wave 84A — live wiring)
static PROPOSAL_STATE: GlobalSignal<Option<openwand_app::ui::workflow_proposal_state::WorkflowProposalUiState>> = Signal::global(|| None);
static READINESS_STATE: GlobalSignal<Option<openwand_app::ui::workflow_readiness_state::WorkflowReadinessUiState>> = Signal::global(|| None);
static OUTCOME_STATE: GlobalSignal<Option<openwand_app::ui::workflow_action_outcome_state::WorkflowActionOutcomeUiState>> = Signal::global(|| None);
static RECONCILIATION_STATE: GlobalSignal<Option<openwand_app::ui::workflow_reconciliation_state::WorkflowReconciliationUiState>> = Signal::global(|| None);
static LOOP_CONTROLLER_STATE: GlobalSignal<Option<openwand_app::ui::workflow_loop_controller_state::WorkflowLoopControllerUiState>> = Signal::global(|| None);
static WORKFLOW_RUN_REQUEST_STATE: GlobalSignal<openwand_app::ui::workflow_run_request::WorkflowRunRequestState> = Signal::global(WorkflowRunRequestState::default);
static APPROVAL_RESOLUTION_STATE: GlobalSignal<openwand_app::ui::approval_resolution_request::ApprovalResolutionState> = Signal::global(ApprovalResolutionState::default);

// ── Send Handler ──────────────────────────────────────────

async fn handle_send(
    service: Arc<UiSessionService>,
    memory: Arc<SqliteMemoryStore>,
    session_id: String,
    text: String,
) {
    *STATUS_TEXT.write() = "Starting run...".into();
    *RUN_STATE.write() = UiRunState::new_running();

    let s = settings::load_settings();
    let llm_target = LlmTarget {
        provider: openwand_llm::LlmProvider::Custom { name: settings::resolve_provider(&s) },
        model: settings::resolve_model(&s),
        base_url: Some(settings::resolve_base_url(&s)),
        api_key: Some(settings::resolve_api_key(&s)),
    };

    let path = db_path();

    // Open store connections
    let trace_store: Arc<dyn openwand_trace::TraceStore<openwand_store::StoredEvent>> =
        match SqliteStore::open(SqliteStoreConfig::file(&path)).await {
            Ok(s) => Arc::new(s),
            Err(e) => {
                *STATUS_TEXT.write() = format!("Failed to open trace store: {e}");
                *RUN_STATE.write() = UiRunState { status: UiRunStatus::Failed, error: Some(format!("Database error: {e}")), ..UiRunState::new_running() };
                return;
            }
        };

    let llm: Arc<dyn openwand_llm::LlmClient> = match
        openwand_llm::adapters::openai_compatible::OpenAiCompatibleClient::try_new()
    {
        Ok(client) => Arc::new(client),
        Err(e) => {
            *STATUS_TEXT.write() = format!("Failed to create LLM client: {e}");
            *RUN_STATE.write() = UiRunState { status: UiRunStatus::Failed, error: Some(format!("LLM client error: {e}")), ..UiRunState::new_running() };
            return;
        }
    };
    let tools: Arc<dyn openwand_tools::executor::ToolExecutor> = Arc::new(
        openwand_tools::composite::CompositeToolExecutor::local_only(
            openwand_tools::local::batch1_local_tools()
        )
    );
    let policy: Arc<dyn openwand_policy::PolicyEngine> = Arc::new(build_smoke_policy());
    let memory_read: Arc<dyn MemoryReadStore> = memory.clone() as Arc<dyn MemoryReadStore>;

    // Get working directory from session view
    let working_dir = CURRENT_SESSION
        .read()
        .as_ref()
        .and_then(|s| s.working_directory.clone())
        .unwrap_or_else(|| ".".to_string());

    let runner = Arc::new(SessionRunner::new(
        SessionId(session_id.clone()),
        trace_store,
        llm,
        tools,
        policy,
        memory_read,
        working_dir.clone(),
    ));

    // Read cached 02k prompt inputs, filtered by session and working directory
    let cached_inputs = MEMORY_PROMPT_INPUTS
        .read()
        .as_ref()
        .filter(|cached| cached.session_id == session_id)
        .filter(|cached| cached.working_directory == working_dir)
        .map(|cached| cached.inputs.clone());

    // Build capability context from current registries (Patch 3: recomputes, not UI signal)
    let openwand_dir = std::path::Path::new(&working_dir).join(".openwand");
    let skill_registry = openwand_skills::registry::load_skill_registry(&openwand_dir.join("skills.toml"));
    let goal_registry = openwand_goals::registry::load_goal_registry(&openwand_dir.join("goals.toml"));
    let cap_block = openwand_app::session_capability_prompt::build_capability_prompt_inputs(
        &skill_registry,
        &goal_registry,
    );
    // Build preview of what was sent (Patch 3: LastSent mode)
    let readiness = openwand_app::ui::skills_goals_state::build_readiness_report(&skill_registry, &goal_registry);
    let preview = openwand_app::ui::skills_goals_state::build_capability_preview(
        &cap_block,
        &readiness,
        openwand_app::ui::skills_goals_state::CapabilityPreviewMode::LastSent,
    );
    *CAPABILITY_PREVIEW.write() = Some(preview);
    let capability_context = if openwand_app::session_capability_prompt::capability_block_has_content(&cap_block) {
        Some(cap_block)
    } else {
        None
    };

    match service.start_run(
        &session_id,
        text.clone(),
        llm_target,
        runner.clone(),
        std::path::PathBuf::from(&working_dir),
        cached_inputs,
        capability_context,
    ).await {
        Ok(handle) => {
            *ACTIVE_RUNNER.write() = Some(ActiveRun {
                runner: runner.clone(),
                cancellation: handle.cancellation,
                state: handle.state.clone(),
            });
            *STATUS_TEXT.write() = "Run started".into();

            // Poll until done, then project memory
            let state = handle.state;
            poll_and_project(state, memory, runner).await;
        }
        Err(e) => {
            *STATUS_TEXT.write() = format!("Run error: {e}");
            RUN_STATE.write().status = UiRunStatus::Failed;
            RUN_STATE.write().error = Some(e.to_string());
        }
    }
}

async fn poll_and_project(
    state: Arc<std::sync::Mutex<UiRunState>>,
    memory: Arc<SqliteMemoryStore>,
    runner: Arc<SessionRunner>,
) {
    // Poll run state
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        let snapshot = state.lock().unwrap().clone();
        *RUN_STATE.write() = snapshot;

        match RUN_STATE.read().status {
            UiRunStatus::Completed | UiRunStatus::Failed | UiRunStatus::Cancelled => {
                break;
            }
            _ => {}
        }
    }

    // Run memory projection
    let path = db_path();
    let trace_for_coordinator: Arc<dyn openwand_trace::TraceStore<openwand_store::StoredEvent>> =
        match SqliteStore::open(SqliteStoreConfig::file(&path)).await {
            Ok(s) => Arc::new(s),
            Err(e) => {
                *STATUS_TEXT.write() = format!("Memory coord error: {e}");
                return;
            }
        };

    let memory_write: Arc<dyn MemoryStore> = memory.clone() as Arc<dyn MemoryStore>;
    let extractor: Arc<dyn openwand_memory::MemoryExtractor> =
        Arc::new(openwand_memory::testing::HeuristicExtractor);
    let coordinator = openwand_app::memory_coordinator::MemoryCoordinator::new(
        memory_write,
        extractor,
        trace_for_coordinator,
    );

    let session_id = runner.session_id.clone();
    let projection = coordinator.project_after_run(&session_id).await;

    // Produce 02k prompt inputs for the next turn
    let working_dir = CURRENT_SESSION
        .read()
        .as_ref()
        .and_then(|s| s.working_directory.clone())
        .unwrap_or_else(|| ".".to_string());
    let prompt_result = coordinator
        .produce_prompt_inputs(
            Some(session_id.clone()),
            std::path::Path::new(&working_dir),
            &PromptInputProductionConfig::default(),
        )
        .await;
    // Refresh memory panel — use filtered panel from coordinator output
    let panel = openwand_app::ui::memory_service::build_filtered_panel(&prompt_result);
    *MEMORY_PANEL.write() = panel;

    // Cache prompt inputs for the next turn
    *MEMORY_PROMPT_INPUTS.write() = if prompt_result.inputs.is_empty() {
        None
    } else {
        Some(CachedMemoryPromptInputs {
            session_id: session_id.to_string(),
            working_directory: std::path::PathBuf::from(&working_dir),
            inputs: prompt_result.inputs,
        })
    };

    *STATUS_TEXT.write() = format!(
        "Run complete. Memory: {} trusted, {} new records.",
        MEMORY_PANEL.read().summary.prompt_included,
        projection.records_accepted
    );

    *ACTIVE_RUNNER.write() = None;
}
