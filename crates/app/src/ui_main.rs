//! OpenWand Desktop UI — Wave 02b-3 Live Run View.
//!
//! Run with: cargo run --bin openwand-ui --features desktop

use dioxus::prelude::*;
use dioxus_desktop::{Config, LogicalSize, WindowBuilder};
use openwand_app::ui::run_dto::{UiRunEvent, UiRunState, UiRunStatus};
use openwand_app::ui::{CreateSessionRequest, UiSessionSummary, UiSessionView, UiSessionService};
use openwand_core::SessionId;
use openwand_llm::LlmTarget;
use openwand_session::runner::SessionRunner;
use openwand_store::backends::sqlite::{SqliteStore, SqliteStoreConfig};
use openwand_store::SessionRegistryStore;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

fn main() {
    let desktop_cfg = Config::new().with_window(
        WindowBuilder::new()
            .with_title("OpenWand")
            .with_inner_size(LogicalSize::new(960, 640)),
    );

    LaunchBuilder::new().with_cfg(desktop_cfg).launch(App);
}

// ── Shared State ──────────────────────────────────────────

static SESSION_LIST: GlobalSignal<Vec<UiSessionSummary>> = Signal::global(Vec::new);
static SELECTED_SESSION_ID: GlobalSignal<Option<String>> = Signal::global(|| None);
static CURRENT_SESSION: GlobalSignal<Option<UiSessionView>> = Signal::global(|| None);
static RUN_STATE: GlobalSignal<UiRunState> = Signal::global(UiRunState::default);
static STATUS_TEXT: GlobalSignal<String> = Signal::global(|| "Ready".into());

/// Active runner + handle for the selected session.
static ACTIVE_RUNNER: GlobalSignal<Option<ActiveRun>> = Signal::global(|| None);

/// Tracks the active run (runner + cancellation + bridge state).
pub struct ActiveRun {
    pub runner: Arc<SessionRunner>,
    pub cancellation: CancellationToken,
    pub state: Arc<std::sync::Mutex<UiRunState>>,
}

// ── App Init ──────────────────────────────────────────────

/// Stub memory store — returns empty context.
struct StubMemoryStore;

#[async_trait::async_trait]
impl openwand_memory::MemoryReadStore for StubMemoryStore {
    async fn search(
        &self,
        _query: openwand_memory::MemoryQuery,
    ) -> std::result::Result<openwand_memory::RetrievalContext, openwand_memory::MemoryError> {
        Ok(openwand_memory::RetrievalContext::empty())
    }
}

/// Build the smoke policy (Read + Search only).
fn build_smoke_policy() -> openwand_policy::BuiltinPolicyEngine {
    let rules = vec![
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
    ];
    openwand_policy::BuiltinPolicyEngine::new(rules)
}

fn init_service() -> Arc<UiSessionService> {
    let db_path = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("openwand")
        .join("openwand.db");

    let store = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(
            SqliteStore::open(SqliteStoreConfig::file(&db_path))
        )
    }).expect("Failed to open store");

    let registry: Arc<dyn SessionRegistryStore> = Arc::new(store);
    Arc::new(UiSessionService::new(registry))
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

    rsx! {
        div { style: "display: flex; height: 100vh; font-family: system-ui; margin: 0;",

            // Left sidebar
            div {
                style: "width: 280px; min-width: 280px; background: #f7f7f7;
                        border-right: 1px solid #ddd; display: flex; flex-direction: column;",

                // Header
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
                                    match svc.create_session(CreateSessionRequest {
                                        title: Some("New Session".into()),
                                        model: Some("qwen3-4b".into()),
                                        base_url: Some("http://100.64.0.1:1234/v1".into()),
                                        provider: Some("lm-studio".into()),
                                        working_directory: Some(".".into()),
                                        interaction_mode: "direct".into(),
                                    }) {
                                        Ok(summary) => {
                                            let id = summary.session_id.clone();
                                            if let Ok(sessions) = svc.list_sessions() {
                                                *SESSION_LIST.write() = sessions;
                                            }
                                            if let Ok(view) = svc.open_session(&id) {
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

                // Session list
                div { style: "flex: 1; overflow-y: auto;",
                    for session in SESSION_LIST.read().iter() {
                        {
                            let id = session.session_id.clone();
                            let title = session.title.clone().unwrap_or_else(|| "Untitled".into());
                            let model = session.model.clone().unwrap_or_else(|| "No model".into());
                            let status = session.status.clone();
                            let selected = SELECTED_SESSION_ID.read().as_deref() == Some(id.as_str());
                            let bg = if selected { "#e0e8f0" } else { "transparent" };
                            let svc = service.clone();
                            rsx! {
                                div {
                                    key: "{id}",
                                    style: "padding: 10px 16px; cursor: pointer; background: {bg};
                                            border-bottom: 1px solid #eee;",
                                    onclick: {
                                        let svc = svc.clone();
                                        move |_| {
                                            let id = id.clone();
                                            let svc = svc.clone();
                                            spawn(async move {
                                                *SELECTED_SESSION_ID.write() = Some(id.clone());
                                                match svc.open_session(&id) {
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
                    }
                    if SESSION_LIST.read().is_empty() {
                        div { style: "padding: 24px 16px; color: #999; font-size: 13px; text-align: center;",
                            "No sessions yet."
                            br {}
                            "Click \"+ New\" to create one."
                        }
                    }
                }
            }

            // Right pane
            div { style: "flex: 1; display: flex; flex-direction: column;",
                {render_detail_pane(service.clone())}
            }
        }
    }
}

// ── Detail Pane ───────────────────────────────────────────

fn render_detail_pane(service: Arc<UiSessionService>) -> Element {
    let current = CURRENT_SESSION.read().clone();
    let run_state = RUN_STATE.read().clone();
    let has_runner = ACTIVE_RUNNER.read().is_some();

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
                    // Phase / run status badge
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
                    // Streaming text
                    if !run_state.streamed_text.is_empty() {
                        div { style: "margin-bottom: 12px; padding: 10px 14px; background: #f0f0f0;
                                     border: 1px solid #ddd; border-radius: 6px;",
                            div { style: "font-size: 11px; font-weight: 600; color: #888; margin-bottom: 4px;",
                                "Assistant"
                            }
                            div { style: "font-size: 13px; color: #333; white-space: pre-wrap;",
                                "{run_state.streamed_text}"
                                if is_running {
                                    span { style: "color: #4a90d9; animation: blink 1s infinite;",
                                        "▍"
                                    }
                                }
                            }
                        }
                    }

                    // Tool events
                    for event in run_state.tool_events.iter() {
                        {render_tool_event(event.clone())}
                    }

                    // Empty state
                    if run_state.streamed_text.is_empty() && run_state.tool_events.is_empty() && !is_running {
                        div { style: "color: #999; font-size: 14px; text-align: center; margin-top: 40px;",
                            "Type a message below to start"
                        }
                    }

                    // Error display
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
                        rsx! {
                            textarea {
                                style: "flex: 1; padding: 8px 12px; font-size: 13px; border: 1px solid #ddd;
                                        border-radius: 4px; resize: none; font-family: system-ui;
                                        min-height: 36px; max-height: 120px;",
                                rows: "1",
                                placeholder: if is_running { "Running..." } else { "Type a message..." },
                                disabled: is_running,
                                onkeydown: {
                                    let svc = svc.clone();
                                    move |e: KeyboardEvent| {
                                        if e.key() == Key::Enter && !e.modifiers().shift() {
                                            e.prevent_default();
                                            // Trigger send via the send button's logic
                                            // We'll use a GlobalSignal for input text
                                        }
                                    }
                                },
                                onchange: {
                                    move |e: FormEvent| {
                                        *INPUT_TEXT.write() = e.value().clone();
                                    }
                                }
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
                            let svc = service.clone();
                            let sid = session_id.clone();
                            move |_| {
                                let svc = svc.clone();
                                let sid = sid.clone();
                                let text = INPUT_TEXT.read().clone();
                                if text.is_empty() { return; }
                                *INPUT_TEXT.write() = String::new();
                                spawn(async move {
                                    handle_send(svc, sid, text).await;
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

// ── Tool Event Card ───────────────────────────────────────

fn render_tool_event(event: UiRunEvent) -> Element {
    match event {
        UiRunEvent::ToolCallStarted { id, name } => rsx! {
            div { style: "margin-bottom: 8px; padding: 8px 12px; background: #f0f8e8;
                         border: 1px solid #c8e0b0; border-radius: 6px;
                         display: flex; align-items: center; gap: 8px;",
                div { style: "width: 8px; height: 8px; background: #f0c040; border-radius: 50%;" }
                div {
                    div { style: "font-size: 11px; font-weight: 600; color: #888;",
                        "Tool Call"
                    }
                    div { style: "font-size: 12px; color: #555;",
                        "{name}"
                    }
                }
            }
        },
        UiRunEvent::ToolCallCompleted { id, name, output, is_error } => {
            let bg = if is_error { "#fde8e8" } else { "#e8f4e8" };
            let border = if is_error { "#e8a0a0" } else { "#a0c8a0" };
            let dot_color = if is_error { "#cc3333" } else { "#33aa33" };
            rsx! {
                div { style: "margin-bottom: 8px; padding: 8px 12px; background: {bg};
                             border: 1px solid {border}; border-radius: 6px;
                             display: flex; align-items: flex-start; gap: 8px;",
                    div { style: "width: 8px; height: 8px; background: {dot_color}; border-radius: 50%; margin-top: 4px;" }
                    div { style: "flex: 1;",
                        div { style: "font-size: 11px; font-weight: 600; color: #888;",
                            if is_error { "Tool Error" } else { "Tool Result" }
                        }
                        div { style: "font-size: 12px; color: #555;",
                            "{name}"
                        }
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

// ── Send Handler ──────────────────────────────────────────

async fn handle_send(
    service: Arc<UiSessionService>,
    session_id: String,
    text: String,
) {
    *STATUS_TEXT.write() = "Starting run...".into();

    // Reset run state
    *RUN_STATE.write() = UiRunState::new_running();

    // Build LLM target from session or defaults
    let llm_target = LlmTarget {
        provider: openwand_llm::LlmProvider::Custom { name: "lm-studio".into() },
        model: "qwen/qwen3-4b-2507".into(),
        base_url: Some("http://100.64.0.1:1234/v1".into()),
        api_key: Some("lm-studio".into()),
    };

    // Build the runner
    let db_path = dirs::data_dir()
        .unwrap()
        .join("openwand")
        .join("openwand.db");

    // Open a second store connection for the runner's trace
    let trace_store: Arc<dyn openwand_trace::TraceStore<openwand_store::StoredEvent>> =
        Arc::new(
            openwand_store::backends::sqlite::SqliteStore::open(
                openwand_store::backends::sqlite::SqliteStoreConfig::file(&db_path)
            )
            .await
            .expect("Failed to open trace store")
        );
    let llm: Arc<dyn openwand_llm::LlmClient> = Arc::new(
        openwand_llm::adapters::openai_compatible::OpenAiCompatibleClient::new()
    );
    let tools: Arc<dyn openwand_tools::executor::ToolExecutor> = Arc::new(
        openwand_tools::composite::CompositeToolExecutor::local_only(
            openwand_tools::local::batch1_local_tools()
        )
    );
    let policy: Arc<dyn openwand_policy::PolicyEngine> = Arc::new(
        build_smoke_policy()
    );
    let memory: Arc<dyn openwand_memory::MemoryReadStore> = Arc::new(
        StubMemoryStore
    );

    let runner = Arc::new(SessionRunner::new(
        SessionId(session_id.clone()),
        trace_store,
        llm,
        tools,
        policy,
        memory,
        ".".into(),
    ));

    // Start run via service
    match service.start_run(&session_id, text, llm_target, runner.clone()).await {
        Ok(handle) => {
            let state = handle.state.clone();
            *ACTIVE_RUNNER.write() = Some(ActiveRun {
                runner,
                cancellation: handle.cancellation,
                state: handle.state,
            });
            *STATUS_TEXT.write() = "Run started".into();

            // Poll the bridge state into RUN_STATE GlobalSignal
            poll_run_state(state).await;
        }
        Err(e) => {
            *STATUS_TEXT.write() = format!("Run error: {e}");
            RUN_STATE.write().status = UiRunStatus::Failed;
            RUN_STATE.write().error = Some(e.to_string());
        }
    }
}

/// Poll the shared run state and sync to the GlobalSignal.
/// Runs until the run completes or is cancelled.
async fn poll_run_state(state: Arc<std::sync::Mutex<UiRunState>>) {
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        let snapshot = {
            let s = state.lock().unwrap();
            s.clone()
        };
        *RUN_STATE.write() = snapshot;

        match RUN_STATE.read().status {
            UiRunStatus::Completed | UiRunStatus::Failed | UiRunStatus::Cancelled => {
                let status_str = format!("{:?}", RUN_STATE.read().status);
                *STATUS_TEXT.write() = format!("Run {}", status_str);
                // Clear active runner
                *ACTIVE_RUNNER.write() = None;
                break;
            }
            _ => {}
        }
    }
}
