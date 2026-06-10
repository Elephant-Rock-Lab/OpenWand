//! OpenWand Desktop UI — Wave 02g Real Memory Wiring.
//!
//! Run with: cargo run --bin openwand-ui --features desktop

use dioxus::prelude::*;
use dioxus_desktop::{Config, LogicalSize, WindowBuilder};
use openwand_app::ui::memory_dto::UiFilteredMemoryPanel;
use openwand_app::memory_coordinator::PromptInputProductionConfig;
use openwand_app::ui::run_dto::{UiRunEvent, UiRunState, UiRunStatus};
use openwand_app::ui::{CreateSessionRequest, UiSessionService, UiSessionSummary, UiSessionView};
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
    openwand_policy::BuiltinPolicyEngine::new(vec![
        openwand_policy::PolicyRule {
            id: openwand_policy::PolicyRuleId("smoke-allow-read".into()),
            name: "Allow read-effect tools (smoke)".into(),
            enabled: true,
            priority: 0,
            class: openwand_policy::RuleClass::BuiltinDefault,
            matcher: openwand_policy::ToolMatcher::ToolEffect {
                effect: openwand_core::tool_vocab::ToolEffect::Read,
            },
            effect: openwand_policy::PolicyEffect::Allow {
                risk: openwand_core::risk::RiskLevelSnapshot::Low,
                confirmation: openwand_core::mode::ConfirmationLevel::Auto,
            },
            reason_code: "smoke_allow_read".into(),
            summary: "Allow read-effect tools.".into(),
        },
        openwand_policy::PolicyRule {
            id: openwand_policy::PolicyRuleId("smoke-allow-search".into()),
            name: "Allow search-effect tools (smoke)".into(),
            enabled: true,
            priority: 0,
            class: openwand_policy::RuleClass::BuiltinDefault,
            matcher: openwand_policy::ToolMatcher::ToolEffect {
                effect: openwand_core::tool_vocab::ToolEffect::Search,
            },
            effect: openwand_policy::PolicyEffect::Allow {
                risk: openwand_core::risk::RiskLevelSnapshot::Low,
                confirmation: openwand_core::mode::ConfirmationLevel::Auto,
            },
            reason_code: "smoke_allow_search".into(),
            summary: "Allow search-effect tools.".into(),
        },
    ])
}

fn db_path() -> std::path::PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("openwand")
        .join("openwand.db")
}

fn init_service() -> Arc<UiSessionService> {
    let path = db_path();

    let store_registry = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(
            SqliteStore::open(SqliteStoreConfig::file(&path))
        )
    }).expect("Failed to open store");

    let store_trace = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(
            SqliteStore::open(SqliteStoreConfig::file(&path))
        )
    }).expect("Failed to open trace store");

    let registry: Arc<dyn SessionRegistryStore> = Arc::new(store_registry);
    let trace: Arc<dyn openwand_trace::TraceStore<openwand_store::StoredEvent>> = Arc::new(store_trace);
    Arc::new(UiSessionService::new(registry, trace))
}

fn init_memory() -> Arc<SqliteMemoryStore> {
    let path = db_path();
    Arc::new(
        SqliteMemoryStore::open(&path).expect("Failed to open memory store")
    )
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
                        let tab_session_active = ACTIVE_TAB.read() == "session";
                        let tab_console_active = ACTIVE_TAB.read() == "console";
                        let tab_inspector_active = ACTIVE_TAB.read() == "inspector";
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
                                    // Load console state if we have a session
                                    if let Some(ref view) = *CURRENT_SESSION.read() {
                                        let session_id = view.summary.session_id.clone();
                                        let path = db_path();
                                        spawn(async move {
                                            use openwand_workflow::workflow_run::WorkflowExecutionId;
                                            let wfx_id = WorkflowExecutionId(session_id.clone());
                                            match openwand_app::workflow_operator_console::assemble_console_state(&path, &wfx_id) {
                                                Ok(state) => {
                                                    *CONSOLE_STATE.write() = Some(state);
                                                    *STATUS_TEXT.write() = "Console loaded".into();
                                                }
                                                Err(e) => {
                                                    *CONSOLE_STATE.write() = None;
                                                    *STATUS_TEXT.write() = format!("Console: {e}");
                                                }
                                            }
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
                                    // Load inspector state if we have a session
                                    if let Some(ref view) = *CURRENT_SESSION.read() {
                                        let session_id = view.summary.session_id.clone();
                                        let path = db_path();
                                        spawn(async move {
                                            use openwand_workflow::workflow_run::WorkflowExecutionId;
                                            let wfx_id = WorkflowExecutionId(session_id.clone());
                                            // Read-only inspection only — no export
                                            match openwand_app::workflow_evidence_chain_inspector::assemble_evidence_chain(&path, &wfx_id, false) {
                                                Ok(state) => {
                                                    *INSPECTOR_STATE.write() = Some(state);
                                                    *STATUS_TEXT.write() = "Inspector loaded".into();
                                                }
                                                Err(e) => {
                                                    *INSPECTOR_STATE.write() = None;
                                                    *STATUS_TEXT.write() = format!("Inspector: {e}");
                                                }
                                            }
                                        });
                                    }
                                },
                                "Inspector"
                            }
                        }
                    }}
                }
                // Tab content
                if ACTIVE_TAB.read() == "console" {
                    { render_console_pane() }
                } else if ACTIVE_TAB.read() == "inspector" {
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
                        match svc.open_session(&id).await {
                            Ok(view) => {
                                *CURRENT_SESSION.write() = Some(view);
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
    if panel.is_empty() {
        return rsx! {
            div { style: "padding: 24px 16px; color: #999; font-size: 12px; text-align: center;",
                "No memory analysis yet."
                br {}
                "Run a turn to populate."
            }
        };
    }

    rsx! {
        div {
            { render_bucket("✓ Trusted", "#4caf50", &panel.prompt_included) }
            { render_bucket("⚠ Stale", "#ff9800", &panel.stale) }
            { render_bucket("✗ Missing in repo", "#f44336", &panel.missing_in_repo) }
            { render_bucket("? Missing in memory", "#9e9e9e", &panel.missing_in_memory) }
            { render_conflicts("⚡ Conflicts", "#e91e63", &panel.conflicts) }
            { render_bucket("○ Unverifiable", "#9e9e9e", &panel.unverifiable) }
            { render_bucket("⊘ Superseded", "#bdbdbd", &panel.superseded_ignored) }
        }
    }
}

fn render_bucket(title: &str, color: &str, rows: &[openwand_app::ui::memory_dto::UiMemoryPanelRow]) -> Element {
    if rows.is_empty() {
        return rsx! { div {} };
    }

    rsx! {
        div { style: "border-bottom: 1px solid #eee;",
            div { style: "padding: 8px 16px 4px; font-size: 11px; font-weight: 600; color: {color};",
                "{title} ({rows.len()})"
            }
            for row in rows.iter() {
                div { style: "padding: 4px 16px 2px; font-size: 11px; color: #333; line-height: 1.3;",
                    "{row.claim}"
                }
                if !row.provenance_label.is_empty() {
                    div { style: "padding: 0 16px 4px; font-size: 10px; color: #888; line-height: 1.2;",
                        "{row.provenance_label}"
                    }
                }
            }
        }
    }
}

fn render_conflicts(title: &str, color: &str, conflicts: &[openwand_app::ui::memory_dto::UiMemoryPanelConflict]) -> Element {
    if conflicts.is_empty() {
        return rsx! { div {} };
    }
    let total_claims: usize = conflicts.iter().map(|g| g.claims.len()).sum();

    rsx! {
        div { style: "border-bottom: 1px solid #eee;",
            div { style: "padding: 8px 16px 4px; font-size: 11px; font-weight: 600; color: {color};",
                "{title} ({total_claims})"
            }
            for group in conflicts.iter() {
                for claim in group.claims.iter() {
                    div { style: "padding: 4px 16px 6px; font-size: 11px; color: #333; line-height: 1.3;",
                        "{claim.claim}"
                    }
                }
            }
        }
    }
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

    let inspector_state = INSPECTOR_STATE.read().clone();
    match inspector_state {
        Some(state) => render_evidence_chain_inspector(&state),
        None => render_inspector_empty_state(),
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
    match event {
        UiRunEvent::ToolCallStarted { id, name } => rsx! {
            div { style: "margin-bottom: 8px; padding: 8px 12px; background: #f0f8e8;
                         border: 1px solid #c8e0b0; border-radius: 6px;
                         display: flex; align-items: center; gap: 8px;",
                div { style: "width: 8px; height: 8px; background: #f0c040; border-radius: 50%;" }
                div {
                    div { style: "font-size: 11px; font-weight: 600; color: #888;", "Tool Call" }
                    div { style: "font-size: 12px; color: #555;", "{name}" }
                }
            }
        },
        UiRunEvent::ToolCallCompleted { id, name, output, is_error } => {
            let bg = if is_error { "#fde8e8" } else { "#e8f4e8" };
            let border = if is_error { "#e8a0a0" } else { "#a0c8a0" };
            let dot = if is_error { "#cc3333" } else { "#33aa33" };
            rsx! {
                div { style: "margin-bottom: 8px; padding: 8px 12px; background: {bg};
                             border: 1px solid {border}; border-radius: 6px;
                             display: flex; align-items: flex-start; gap: 8px;",
                    div { style: "width: 8px; height: 8px; background: {dot}; border-radius: 50%; margin-top: 4px;" }
                    div { style: "flex: 1;",
                        div { style: "font-size: 11px; font-weight: 600; color: #888;",
                            if is_error { "Tool Error" } else { "Tool Result" }
                        }
                        div { style: "font-size: 12px; color: #555;", "{name}" }
                        if !output.is_empty() {
                            div { style: "font-size: 11px; color: #777; margin-top: 4px;
                                         max-height: 80px; overflow-y: auto; white-space: pre-wrap;",
                                "{output}"
                            }
                        }
                    }
                }
            }
        }
        _ => rsx! { div {} },
    }
}

// ── Input Text ────────────────────────────────────────────

static INPUT_TEXT: GlobalSignal<String> = Signal::global(String::new);

/// Active view tab: "session" or "console"
static ACTIVE_TAB: GlobalSignal<String> = Signal::global(|| "session".into());

/// Cached operator console state for the selected workflow run.
static CONSOLE_STATE: GlobalSignal<Option<openwand_workflow::workflow_operator_console::WorkflowOperatorConsoleState>> = Signal::global(|| None);

/// Cached evidence chain inspector state for the selected workflow run.
static INSPECTOR_STATE: GlobalSignal<Option<openwand_workflow::workflow_evidence_chain_inspector::EvidenceChainInspectionState>> = Signal::global(|| None);

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
        .filter(|cached| cached.working_directory == std::path::PathBuf::from(&working_dir))
        .map(|cached| cached.inputs.clone());

    match service.start_run(
        &session_id,
        text.clone(),
        llm_target,
        runner.clone(),
        std::path::PathBuf::from(&working_dir),
        cached_inputs,
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
