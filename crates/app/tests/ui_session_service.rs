//! UI session service acceptance tests.
//!
//! Proves the service layer correctly wraps the store registry
//! and produces UI-friendly DTOs, including trace replay.

use openwand_app::ui::{CreateSessionRequest, UiSessionService};
use openwand_store::backends::sqlite::SqliteStore;
use std::sync::Arc;

/// Helper: create service from a single shared store (Arc clone for both traits).
async fn open_service_async() -> (UiSessionService, Arc<dyn openwand_trace::TraceStore<openwand_store::StoredEvent>>) {
    let store = SqliteStore::open_in_temp_dir().await.unwrap();
    let arc: Arc<SqliteStore> = Arc::new(store);
    let registry: Arc<dyn openwand_store::SessionRegistryStore> = arc.clone();
    let trace: Arc<dyn openwand_trace::TraceStore<openwand_store::StoredEvent>> = arc.clone();
    let svc = UiSessionService::new(registry, trace.clone());
    (svc, trace)
}

fn open_service_sync() -> UiSessionService {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store = rt.block_on(SqliteStore::open_in_temp_dir()).unwrap();
    let arc: Arc<SqliteStore> = Arc::new(store);
    let registry: Arc<dyn openwand_store::SessionRegistryStore> = arc.clone();
    let trace: Arc<dyn openwand_trace::TraceStore<openwand_store::StoredEvent>> = arc.clone();
    UiSessionService::new(registry, trace)
}

#[test]
fn ui_session_service_lists_registry_sessions() {
    let svc = open_service_sync();

    let sessions = svc.list_sessions().unwrap();
    assert!(sessions.is_empty());

    svc.create_session(CreateSessionRequest {
        title: Some("Test".into()),
        model: Some("qwen3-4b".into()),
        base_url: None,
        provider: None,
        working_directory: None,
        interaction_mode: "direct".into(),
    })
    .unwrap();

    let sessions = svc.list_sessions().unwrap();
    assert_eq!(1, sessions.len());
    assert_eq!(Some("Test".into()), sessions[0].title);
    assert_eq!(Some("qwen3-4b".into()), sessions[0].model);
}

#[test]
fn ui_session_service_create_session_adds_registry_row() {
    let svc = open_service_sync();

    let summary = svc
        .create_session(CreateSessionRequest {
            title: Some("My Session".into()),
            model: None,
            base_url: Some("http://localhost:1234/v1".into()),
            provider: Some("lm-studio".into()),
            working_directory: Some("/tmp".into()),
            interaction_mode: "conversational".into(),
        })
        .unwrap();

    assert_eq!("My Session", summary.title.unwrap());
    assert_eq!("active", summary.status);
    assert!(!summary.session_id.is_empty());
}

#[tokio::test]
async fn ui_session_service_open_empty_session_returns_empty_messages() {
    let (svc, _trace) = open_service_async().await;

    let created = svc
        .create_session(CreateSessionRequest {
            title: None,
            model: None,
            base_url: None,
            provider: None,
            working_directory: None,
            interaction_mode: "direct".into(),
        })
        .unwrap();

    let view = svc.open_session(&created.session_id).await.unwrap();
    assert!(view.messages.is_empty());
    assert_eq!("direct", view.interaction_mode);
}

#[tokio::test]
async fn ui_session_service_open_session_with_metadata() {
    let (svc, _trace) = open_service_async().await;

    let created = svc
        .create_session(CreateSessionRequest {
            title: Some("Meta Test".into()),
            model: Some("qwen3-4b".into()),
            base_url: Some("http://gpu:1234/v1".into()),
            provider: Some("lm-studio".into()),
            working_directory: Some("/home/user".into()),
            interaction_mode: "direct".into(),
        })
        .unwrap();

    let view = svc.open_session(&created.session_id).await.unwrap();
    assert_eq!(Some("Meta Test".into()), view.summary.title);
    assert_eq!(Some("qwen3-4b".into()), view.summary.model);
    assert_eq!(Some("http://gpu:1234/v1".into()), view.base_url);
    assert_eq!(Some("lm-studio".into()), view.provider);
}

#[tokio::test]
async fn ui_session_service_missing_session_returns_not_found() {
    let (svc, _trace) = open_service_async().await;

    let result = svc.open_session("nonexistent_id").await;
    assert!(result.is_err());
    match result.unwrap_err() {
        openwand_app::ui::UiServiceError::NotFound(id) => {
            assert_eq!("nonexistent_id", id);
        }
        other => panic!("Expected NotFound, got: {other}"),
    }
}

#[tokio::test]
async fn ui_session_service_replays_user_message_from_trace() {
    use openwand_core::events::SessionEvent;
    use openwand_core::OpenWandTraceEvent;
    use openwand_trace::actor::Actor;
    use openwand_trace::append::AppendTraceEntry;
    use openwand_trace::stream::{TraceStreamId, TraceStreamScope};

    let (svc, trace) = open_service_async().await;

    let created = svc
        .create_session(CreateSessionRequest {
            title: Some("Replay Test".into()),
            model: None,
            base_url: None,
            provider: None,
            working_directory: None,
            interaction_mode: "direct".into(),
        })
        .unwrap();

    // Write a user message trace event
    let stream_id = TraceStreamId {
        scope: TraceStreamScope::Session,
        id: created.session_id.clone(),
    };
    trace
        .append(AppendTraceEntry {
            stream_id,
            actor: Actor::User,
            event: openwand_store::StoredEvent::from(
                OpenWandTraceEvent::Session(SessionEvent::UserMessageInjected {
                    text: "Hello from trace replay".into(),
                }),
            ),
            relations: vec![],
            idempotency_key: None,
        })
        .await
        .unwrap();

    let view = svc.open_session(&created.session_id).await.unwrap();
    assert_eq!(1, view.messages.len());
    assert_eq!("Hello from trace replay", view.messages[0].text);
    assert_eq!(openwand_app::ui::UiMessageRole::User, view.messages[0].role);
}

#[tokio::test]
async fn ui_session_service_replays_tool_call_from_trace() {
    use openwand_core::events::ToolEvent;
    use openwand_core::ids::ToolCallId;
    use openwand_core::OpenWandTraceEvent;
    use openwand_trace::actor::Actor;
    use openwand_trace::append::AppendTraceEntry;
    use openwand_trace::stream::{TraceStreamId, TraceStreamScope};

    let (svc, trace) = open_service_async().await;

    let created = svc
        .create_session(CreateSessionRequest {
            title: None,
            model: None,
            base_url: None,
            provider: None,
            working_directory: None,
            interaction_mode: "direct".into(),
        })
        .unwrap();

    let stream_id = TraceStreamId {
        scope: TraceStreamScope::Session,
        id: created.session_id.clone(),
    };

    // Tool call
    trace
        .append(AppendTraceEntry {
            stream_id: stream_id.clone(),
            actor: Actor::System { component: "test".into() },
            event: openwand_store::StoredEvent::from(OpenWandTraceEvent::Tool(
                ToolEvent::Called {
                    tool_call_id: ToolCallId("tc_1".into()),
                    tool_name: "local__read".into(),
                    args_hash: "hash1".into(),
                    invoker: openwand_core::tool_vocab::ToolInvoker::Llm,
                },
            )),
            relations: vec![],
            idempotency_key: None,
        })
        .await
        .unwrap();

    // Tool completed
    trace
        .append(AppendTraceEntry {
            stream_id: stream_id.clone(),
            actor: Actor::System { component: "test".into() },
            event: openwand_store::StoredEvent::from(OpenWandTraceEvent::Tool(
                ToolEvent::Completed {
                    tool_call_id: ToolCallId("tc_1".into()),
                    tool_name: "local__read".into(),
                    status: openwand_core::tool_vocab::ToolResultStatus::Success,
                    result_summary: "file contents here".into(),
                    duration_ms: 42,
                },
            )),
            relations: vec![],
            idempotency_key: None,
        })
        .await
        .unwrap();

    let timeline = openwand_app::ui::replay::replay_timeline(trace.as_ref(), &created.session_id)
        .await
        .unwrap();

    assert_eq!(2, timeline.len());
    match &timeline[0] {
        openwand_app::ui::UiTimelineItem::ToolCall { tool_name, .. } => {
            assert_eq!("local__read", tool_name);
        }
        other => panic!("Expected ToolCall, got: {other:?}"),
    }
    match &timeline[1] {
        openwand_app::ui::UiTimelineItem::ToolResult {
            tool_name, output_preview, is_error, ..
        } => {
            assert_eq!("local__read", tool_name);
            assert_eq!("file contents here", output_preview);
            assert!(!is_error);
        }
        other => panic!("Expected ToolResult, got: {other:?}"),
    }
}

#[test]
fn desktop_feature_does_not_affect_cli_build() {
    let _ = std::mem::size_of::<openwand_app::ui::UiSessionSummary>();
    let _ = std::mem::size_of::<openwand_app::ui::UiSessionView>();
    let _ = std::mem::size_of::<openwand_app::ui::UiMessage>();
    let _ = std::mem::size_of::<openwand_app::ui::UiTimelineItem>();
}
