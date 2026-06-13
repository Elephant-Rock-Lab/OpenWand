//! Rebuild verification tests.
//!
//! Tests that session state can be rebuilt from the trace stream.

use openwand_session::rebuild::rebuild_session;
use openwand_session::loro_state::LoroSessionState;
use openwand_trace::testing::InMemoryTraceStore;
use openwand_trace::store::TraceStore;
use openwand_trace::append::AppendTraceEntry;
use openwand_trace::stream::{TraceStreamId, TraceStreamScope};
use openwand_trace::actor::Actor;
use openwand_store::StoredEvent;
use openwand_core::OpenWandTraceEvent;
use openwand_core::events::SessionEvent;

fn make_stream_id(session_id: &str) -> TraceStreamId {
    TraceStreamId {
        scope: TraceStreamScope::Session,
        id: session_id.to_string(),
    }
}

fn user_msg(text: &str) -> StoredEvent {
    StoredEvent::from(OpenWandTraceEvent::Session(
        SessionEvent::UserMessageInjected { text: text.to_string() },
    ))
}

fn make_entry(stream_id: TraceStreamId, event: StoredEvent) -> AppendTraceEntry<StoredEvent> {
    AppendTraceEntry {
        actor: Actor::System { component: "rebuild_test".to_string() },
        event,
        relations: vec![],
        stream_id,
        idempotency_key: None,
    }
}

fn to_trace_event(e: &StoredEvent) -> OpenWandTraceEvent {
    e.clone().into()
}

#[tokio::test]
async fn rebuild_empty_session_has_zero_events() {
    let store = InMemoryTraceStore::<StoredEvent>::new();
    let result = rebuild_session(&store, "empty", None, to_trace_event).await.unwrap();
    assert_eq!(0, result.events_replayed);
    assert!(result.state_matches);
}

#[tokio::test]
async fn rebuild_replays_single_event() {
    let store = InMemoryTraceStore::<StoredEvent>::new();
    let sid = make_stream_id("s1");
    store.append(make_entry(sid, user_msg("hello"))).await.unwrap();

    let result = rebuild_session(&store, "s1", None, to_trace_event).await.unwrap();
    assert_eq!(1, result.events_replayed);
    assert!(result.state_matches);
}

#[tokio::test]
async fn rebuild_detects_message_count_divergence() {
    let store = InMemoryTraceStore::<StoredEvent>::new();
    let sid = make_stream_id("s2");
    store.append(make_entry(sid, user_msg("hello"))).await.unwrap();

    let doc = loro::LoroDoc::new();
    let stored = LoroSessionState::new(&doc);
    stored.append_user_message("a", None::<&str>).unwrap();
    stored.append_user_message("b", None::<&str>).unwrap();
    stored.append_user_message("c", None::<&str>).unwrap();

    let result = rebuild_session(&store, "s2", Some(&stored), to_trace_event).await.unwrap();
    assert_eq!(1, result.events_replayed);
    assert!(!result.state_matches);
    assert!(result.divergences[0].contains("Message count"));
}

#[tokio::test]
async fn idempotent_replay_same_result() {
    let store = InMemoryTraceStore::<StoredEvent>::new();
    let sid = make_stream_id("s3");
    store.append(make_entry(sid.clone(), user_msg("once"))).await.unwrap();

    let r1 = rebuild_session(&store, "s3", None, to_trace_event).await.unwrap();
    let r2 = rebuild_session(&store, "s3", None, to_trace_event).await.unwrap();
    assert_eq!(r1.events_replayed, r2.events_replayed);
    assert_eq!(1, r1.events_replayed);
}

#[tokio::test]
async fn rebuild_multiple_events_in_order() {
    let store = InMemoryTraceStore::<StoredEvent>::new();
    let sid = make_stream_id("s4");
    for msg in &["first", "second", "third"] {
        store.append(make_entry(sid.clone(), user_msg(msg))).await.unwrap();
    }

    let result = rebuild_session(&store, "s4", None, to_trace_event).await.unwrap();
    assert_eq!(3, result.events_replayed);
    assert!(result.state_matches);
}
