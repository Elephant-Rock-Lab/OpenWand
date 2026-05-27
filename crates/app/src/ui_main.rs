//! OpenWand Desktop UI — Wave 02b-2 Static Session View.
//!
//! Run with: cargo run --bin openwand-ui --features desktop

use dioxus::prelude::*;
use dioxus_desktop::{Config, LogicalSize, WindowBuilder};
use openwand_app::ui::{CreateSessionRequest, UiSessionService, UiSessionView};
use openwand_store::backends::sqlite::{SqliteStore, SqliteStoreConfig};
use std::sync::Arc;

fn main() {
    let desktop_cfg = Config::new().with_window(
        WindowBuilder::new()
            .with_title("OpenWand")
            .with_inner_size(LogicalSize::new(900, 600)),
    );

    LaunchBuilder::new().with_cfg(desktop_cfg).launch(App);
}

static SELECTED_SESSION_ID: GlobalSignal<Option<String>> = Signal::global(|| None);
static SESSION_LIST: GlobalSignal<Vec<openwand_app::ui::UiSessionSummary>> =
    Signal::global(Vec::new);
static CURRENT_SESSION: GlobalSignal<Option<UiSessionView>> = Signal::global(|| None);
static STATUS_TEXT: GlobalSignal<String> = Signal::global(|| "Ready".into());

fn init_service() -> Arc<UiSessionService> {
    let db_path = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("openwand")
        .join("openwand.db");

    // Open store synchronously (SqliteStore::open is async but the writer
    // does blocking work internally; we use tokio::task::block_in_place
    // since we're inside the Dioxus/tokio runtime)
    let store = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(
            SqliteStore::open(SqliteStoreConfig::file(&db_path))
        )
    }).expect("Failed to open store. Ensure OpenWand is initialized.");

    let registry: Arc<dyn openwand_store::SessionRegistryStore> = Arc::new(store);
    Arc::new(UiSessionService::new(registry))
}

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

                // Header with New button
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
                                    *STATUS_TEXT.write() = "Creating session...".into();
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
                            let preview = session.last_message_preview.clone();
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
                                    if let Some(p) = preview {
                                        div { style: "font-size: 11px; color: #aaa; margin-top: 2px;",
                                            "{p}"
                                        }
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
                {match CURRENT_SESSION.read().clone() {
                    Some(view) => rsx! {
                        // Header
                        div { style: "padding: 20px 24px; border-bottom: 1px solid #eee;",
                            h2 { style: "margin: 0 0 4px 0; font-size: 18px;",
                                {view.summary.title.as_deref().unwrap_or("Untitled")}
                            }
                            div { style: "font-size: 12px; color: #888;",
                                "ID: {view.summary.session_id}"
                                " | {view.summary.status}"
                                " | {view.interaction_mode}"
                            }
                        }
                        // Metadata
                        div { style: "padding: 16px 24px; background: #fafafa; border-bottom: 1px solid #eee;
                                      font-size: 13px; color: #555;",
                            if let Some(m) = &view.summary.model {
                                div { "Model: {m}" }
                            }
                            if let Some(p) = &view.provider {
                                div { "Provider: {p}" }
                            }
                            if let Some(u) = &view.base_url {
                                div { "Base URL: {u}" }
                            }
                            if let Some(w) = &view.working_directory {
                                div { "Working Dir: {w}" }
                            }
                            if let Some(ph) = &view.summary.current_phase {
                                div { "Phase: {ph} | Step: {view.current_step}" }
                            }
                        }
                        // Messages / empty state
                        div { style: "flex: 1; padding: 16px 24px; overflow-y: auto;",
                            if view.messages.is_empty() {
                                div { style: "color: #999; font-size: 14px; text-align: center; margin-top: 60px;",
                                    "No messages yet."
                                    br {}
                                    "Live messaging will be available in a future update."
                                }
                            }
                        }
                    },
                    None => rsx! {
                        div { style: "flex: 1; display: flex; align-items: center; justify-content: center;
                                     color: #bbb; font-size: 15px;",
                            "Select a session or create a new one"
                        }
                    }
                }}

                // Status bar
                div { style: "padding: 6px 16px; background: #f0f0f0; border-top: 1px solid #ddd;
                              font-size: 11px; color: #888;",
                    "{STATUS_TEXT}"
                }
            }
        }
    }
}
