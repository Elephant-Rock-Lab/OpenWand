//! 02b-5 durability patch tests.
//!
//! Verifies:
//! - Assistant text is durably recorded in trace
//! - Replay shows assistant text after reopen
//! - Scan pagination works for >100 entries
//! - Full user+assistant+tool round-trip replay

use openwand_app::ui::{CreateSessionRequest, UiMessageRole, UiSessionService, UiTimelineItem};
use openwand_core::events::{SessionEvent, ToolEvent, OpenWandTraceEvent};
use openwand_core::ids::ToolCallId;
use openwand_core::tool_vocab::{ToolInvoker, ToolResultStatus};
use openwand_store::backends::sqlite::SqliteStore;
use openwand_trace::actor::Actor;
use openwand_trace::append::AppendTraceEntry;
use openwand_trace::stream::{TraceStreamId, TraceStreamScope};
use openwand_trace::TraceStore;
use std::sync::Arc;

async fn open_svc() -> (UiSessionService, Arc<dyn TraceStore<openwand_store::StoredEvent>>) {
    let store = SqliteStore::open_in_temp_dir().await.unwrap();
    let arc: Arc<SqliteStore> = Arc::new(store);
    let registry: Arc<dyn openwand_store::SessionRegistryStore> = arc.clone();
    let trace: Arc<dyn TraceStore<openwand_store::StoredEvent>> = arc.clone();
    let svc = UiSessionService::new(registry, trace.clone());
    (svc, trace)
}

fn session_stream(session_id: &str) -> TraceStreamId {
    TraceStreamId { scope: TraceStreamScope::Session, id: session_id.to_string() }
}

#[tokio::test]
async fn assistant_message_text_is_recorded_to_trace() {
    let (svc, trace) = open_svc().await;

    let created = svc.create_session(CreateSessionRequest {
        title: None, model: None, base_url: None,
        provider: None, working_directory: None, interaction_mode: "direct".into(),
    }).unwrap();

    let stream_id = session_stream(&created.session_id);

    // Write user message
    trace.append(AppendTraceEntry {
        stream_id: stream_id.clone(),
        actor: Actor::User,
        event: openwand_store::StoredEvent::from(
            OpenWandTraceEvent::Session(SessionEvent::UserMessageInjected {
                text: "What files are here?".into(),
            }),
        ),
        relations: vec![],
        idempotency_key: None,
    }).await.unwrap();

    // Write assistant message
    trace.append(AppendTraceEntry {
        stream_id: stream_id.clone(),
        actor: Actor::Llm { model: "qwen3-4b".into(), provider: "lm-studio".into() },
        event: openwand_store::StoredEvent::from(
            OpenWandTraceEvent::Session(SessionEvent::AssistantMessageGenerated {
                text: "I see the following files: main.rs, lib.rs".into(),
                model: "qwen3-4b".into(),
            }),
        ),
        relations: vec![],
        idempotency_key: None,
    }).await.unwrap();

    // Replay and verify
    let view = svc.open_session(&created.session_id).await.unwrap();
    assert_eq!(2, view.messages.len());

    assert_eq!(UiMessageRole::User, view.messages[0].role);
    assert_eq!("What files are here?", view.messages[0].text);

    assert_eq!(UiMessageRole::Assistant, view.messages[1].role);
    assert_eq!("I see the following files: main.rs, lib.rs", view.messages[1].text);
}

#[tokio::test]
async fn ui_replay_shows_assistant_text_after_reopen() {
    let (svc, trace) = open_svc().await;

    let created = svc.create_session(CreateSessionRequest {
        title: Some("Reopen Test".into()),
        model: None, base_url: None,
        provider: None, working_directory: None, interaction_mode: "direct".into(),
    }).unwrap();

    let stream_id = session_stream(&created.session_id);

    // Simulate a full user→assistant turn
    trace.append(AppendTraceEntry {
        stream_id: stream_id.clone(),
        actor: Actor::User,
        event: openwand_store::StoredEvent::from(
            OpenWandTraceEvent::Session(SessionEvent::UserMessageInjected {
                text: "Hello".into(),
            }),
        ),
        relations: vec![],
        idempotency_key: None,
    }).await.unwrap();

    trace.append(AppendTraceEntry {
        stream_id: stream_id.clone(),
        actor: Actor::Llm { model: "qwen3".into(), provider: "lm-studio".into() },
        event: openwand_store::StoredEvent::from(
            OpenWandTraceEvent::Session(SessionEvent::AssistantMessageGenerated {
                text: "Hi! How can I help?".into(),
                model: "qwen3".into(),
            }),
        ),
        relations: vec![],
        idempotency_key: None,
    }).await.unwrap();

    // "Reopen" — just open the session again
    let view = svc.open_session(&created.session_id).await.unwrap();
    assert_eq!(2, view.messages.len());
    assert_eq!("Hello", view.messages[0].text);
    assert_eq!("Hi! How can I help?", view.messages[1].text);
    assert_eq!(UiMessageRole::Assistant, view.messages[1].role);
}

#[tokio::test]
async fn ui_replay_handles_more_than_100_trace_entries() {
    let store = SqliteStore::open_in_temp_dir().await.unwrap();
    let arc: Arc<SqliteStore> = Arc::new(store);
    let registry: Arc<dyn openwand_store::SessionRegistryStore> = arc.clone();
    let trace: Arc<dyn TraceStore<openwand_store::StoredEvent>> = arc.clone();
    let svc = UiSessionService::new(registry, trace.clone());

    let created = svc.create_session(CreateSessionRequest {
        title: None, model: None, base_url: None,
        provider: None, working_directory: None, interaction_mode: "direct".into(),
    }).unwrap();

    let stream_id = session_stream(&created.session_id);

    // Write 150 entries — exceeds the 100-per-page limit
    for i in 0..150u32 {
        trace.append(AppendTraceEntry {
            stream_id: stream_id.clone(),
            actor: Actor::User,
            event: openwand_store::StoredEvent::from(
                OpenWandTraceEvent::Session(SessionEvent::UserMessageInjected {
                    text: format!("Message {i}"),
                }),
            ),
            relations: vec![],
            idempotency_key: None,
        }).await.unwrap();
    }

    // Replay should get all 150
    let timeline = openwand_app::ui::replay::replay_timeline(
        trace.as_ref(), &created.session_id,
    ).await.unwrap();

    assert_eq!(150, timeline.len());
    // Verify ordering: first and last
    match &timeline[0] {
        UiTimelineItem::Message(msg) => assert_eq!("Message 0", msg.text),
        other => panic!("Expected Message, got: {other:?}"),
    }
    match &timeline[149] {
        UiTimelineItem::Message(msg) => assert_eq!("Message 149", msg.text),
        other => panic!("Expected Message, got: {other:?}"),
    }
}

#[tokio::test]
async fn ui_replay_full_round_trip_user_assistant_tool() {
    let (svc, trace) = open_svc().await;

    let created = svc.create_session(CreateSessionRequest {
        title: None, model: None, base_url: None,
        provider: None, working_directory: None, interaction_mode: "direct".into(),
    }).unwrap();

    let stream_id = session_stream(&created.session_id);

    // 1. User message
    trace.append(AppendTraceEntry {
        stream_id: stream_id.clone(),
        actor: Actor::User,
        event: openwand_store::StoredEvent::from(
            OpenWandTraceEvent::Session(SessionEvent::UserMessageInjected {
                text: "List files in this directory".into(),
            }),
        ),
        relations: vec![],
        idempotency_key: None,
    }).await.unwrap();

    // 2. Tool call
    trace.append(AppendTraceEntry {
        stream_id: stream_id.clone(),
        actor: Actor::System { component: "tool_executor".into() },
        event: openwand_store::StoredEvent::from(
            OpenWandTraceEvent::Tool(ToolEvent::Called {
                tool_call_id: ToolCallId("tc_1".into()),
                tool_name: "local__file_list".into(),
                args_hash: "h1".into(),
                invoker: ToolInvoker::Llm,
            }),
        ),
        relations: vec![],
        idempotency_key: None,
    }).await.unwrap();

    // 3. Tool result
    trace.append(AppendTraceEntry {
        stream_id: stream_id.clone(),
        actor: Actor::System { component: "tool_executor".into() },
        event: openwand_store::StoredEvent::from(
            OpenWandTraceEvent::Tool(ToolEvent::Completed {
                tool_call_id: ToolCallId("tc_1".into()),
                tool_name: "local__file_list".into(),
                status: ToolResultStatus::Success,
                result_summary: "main.rs, lib.rs, mod.rs".into(),
                duration_ms: 15,
            }),
        ),
        relations: vec![],
        idempotency_key: None,
    }).await.unwrap();

    // 4. Assistant response
    trace.append(AppendTraceEntry {
        stream_id: stream_id.clone(),
        actor: Actor::Llm { model: "qwen3-4b".into(), provider: "lm-studio".into() },
        event: openwand_store::StoredEvent::from(
            OpenWandTraceEvent::Session(SessionEvent::AssistantMessageGenerated {
                text: "The directory contains: main.rs, lib.rs, mod.rs".into(),
                model: "qwen3-4b".into(),
            }),
        ),
        relations: vec![],
        idempotency_key: None,
    }).await.unwrap();

    // Replay full timeline
    let timeline = openwand_app::ui::replay::replay_timeline(
        trace.as_ref(), &created.session_id,
    ).await.unwrap();

    assert_eq!(4, timeline.len());

    // User message
    match &timeline[0] {
        UiTimelineItem::Message(msg) => {
            assert_eq!(UiMessageRole::User, msg.role);
            assert_eq!("List files in this directory", msg.text);
        }
        other => panic!("Expected Message, got: {other:?}"),
    }
    // Tool call
    match &timeline[1] {
        UiTimelineItem::ToolCall { tool_name, .. } => {
            assert_eq!("local__file_list", tool_name);
        }
        other => panic!("Expected ToolCall, got: {other:?}"),
    }
    // Tool result
    match &timeline[2] {
        UiTimelineItem::ToolResult { tool_name, output_preview, is_error, .. } => {
            assert_eq!("local__file_list", tool_name);
            assert_eq!("main.rs, lib.rs, mod.rs", output_preview);
            assert!(!is_error);
        }
        other => panic!("Expected ToolResult, got: {other:?}"),
    }
    // Assistant response
    match &timeline[3] {
        UiTimelineItem::Message(msg) => {
            assert_eq!(UiMessageRole::Assistant, msg.role);
            assert_eq!("The directory contains: main.rs, lib.rs, mod.rs", msg.text);
        }
        other => panic!("Expected Message, got: {other:?}"),
    }
}
