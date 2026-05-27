//! Wave 01a Conformance Tests
//!
//! Proves the generic trace substrate works with real domain events.
//! The seam: `TraceStore<StoredEvent>` where `StoredEvent` wraps `OpenWandTraceEvent`.
//!
//! These tests live in openwand-store because:
//! - Store depends on both openwand-core and openwand-trace
//! - StoredEvent (the newtype bridge) is defined here
//! - Orphan rules prevent the impl from living in core or trace

use openwand_core::*;
use openwand_store::StoredEvent;
use openwand_trace::testing::InMemoryTraceStore;
use openwand_trace::*;

/// Helper: build a session-started event.
fn session_started() -> OpenWandTraceEvent {
    OpenWandTraceEvent::Session(SessionEvent::Started {
        session_id: SessionId::new(),
        mode: InteractionMode::Conversational,
    })
}

/// Helper: build a tool-called event.
fn tool_called(name: &str) -> OpenWandTraceEvent {
    OpenWandTraceEvent::Tool(ToolEvent::Called {
        tool_call_id: ToolCallId::new(),
        tool_name: name.into(),
        args_hash: "abc".into(),
        invoker: ToolInvoker::Llm,
    })
}

/// Helper: build a tool-completed event.
fn tool_completed(name: &str, call_id: ToolCallId) -> OpenWandTraceEvent {
    OpenWandTraceEvent::Tool(ToolEvent::Completed {
        tool_call_id: call_id,
        tool_name: name.into(),
        status: ToolResultStatus::Success,
        result_summary: "ok".into(),
        duration_ms: 50,
    })
}

/// Helper: build a memory fact-extracted event.
fn fact_extracted() -> OpenWandTraceEvent {
    OpenWandTraceEvent::Memory(MemoryEvent::FactExtracted {
        claim_id: ClaimId::new(),
        statement: "Rust is fast".into(),
        confidence: 0.95,
        predicate: "is".into(),
    })
}

/// Helper: build an inference-completed event.
fn inference_completed() -> OpenWandTraceEvent {
    OpenWandTraceEvent::Inference(InferenceEvent::Completed {
        model: "gpt-4o".into(),
        tokens: snapshots::TokenUsageSnapshot {
            input: 1000,
            output: 500,
            reasoning: Some(200),
            cache_read: None,
            cache_write: None,
        },
        stop_reason: "tool_call".into(),
        tool_call_count: 2,
    })
}

fn session_stream() -> TraceStreamId {
    TraceStreamId {
        scope: TraceStreamScope::Session,
        id: "s-main".into(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn trace_append_assigns_ids() {
    let store: InMemoryTraceStore<StoredEvent> = InMemoryTraceStore::new();

    let id = store
        .append(AppendTraceEntry {
            actor: Actor::User,
            event: StoredEvent::from(session_started()),
            relations: vec![],
            stream_id: session_stream(),
            idempotency_key: None,
        })
        .await
        .unwrap();

    assert_eq!(26, id.0.len(), "TraceId should be a 26-char ULID");
}

#[tokio::test]
async fn trace_append_1000_entries() {
    let store: InMemoryTraceStore<StoredEvent> = InMemoryTraceStore::new();
    let stream = session_stream();

    for i in 0..1000 {
        store
            .append(AppendTraceEntry {
                actor: Actor::Llm {
                    model: "gpt-4o".into(),
                    provider: "openai".into(),
                },
                event: StoredEvent::from(tool_called(&format!("tool_{i}"))),
                relations: vec![],
                stream_id: stream.clone(),
                idempotency_key: None,
            })
            .await
            .unwrap();
    }

    assert_eq!(1000, store.current_global_sequence().await.unwrap());
    assert_eq!(1000, store.current_stream_sequence(&stream).await.unwrap());
}

#[tokio::test]
async fn trace_query_by_stream() {
    let store: InMemoryTraceStore<StoredEvent> = InMemoryTraceStore::new();
    let s1 = session_stream();
    let s2 = TraceStreamId {
        scope: TraceStreamScope::Session,
        id: "s-other".into(),
    };

    // Append to s1
    store
        .append(AppendTraceEntry {
            actor: Actor::User,
            event: StoredEvent::from(session_started()),
            relations: vec![],
            stream_id: s1.clone(),
            idempotency_key: None,
        })
        .await
        .unwrap();

    // Append to s2
    store
        .append(AppendTraceEntry {
            actor: Actor::User,
            event: StoredEvent::from(session_started()),
            relations: vec![],
            stream_id: s2.clone(),
            idempotency_key: None,
        })
        .await
        .unwrap();

    // Append another to s1
    store
        .append(AppendTraceEntry {
            actor: Actor::User,
            event: StoredEvent::from(fact_extracted()),
            relations: vec![],
            stream_id: s1.clone(),
            idempotency_key: None,
        })
        .await
        .unwrap();

    let page = store
        .scan(TraceQuery {
            stream_id: Some(s1),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(2, page.entries.len());
    // All should be from s1 stream — verify via event kinds
    let kinds: Vec<&str> = page.entries.iter().map(|e| e.event_kind.as_str()).collect();
    assert_eq!(vec!["session.started", "memory.fact_extracted"], kinds);
}

#[tokio::test]
async fn trace_query_by_event_kind() {
    let store: InMemoryTraceStore<StoredEvent> = InMemoryTraceStore::new();
    let stream = session_stream();

    store
        .append(AppendTraceEntry {
            actor: Actor::User,
            event: StoredEvent::from(session_started()),
            relations: vec![],
            stream_id: stream.clone(),
            idempotency_key: None,
        })
        .await
        .unwrap();

    store
        .append(AppendTraceEntry {
            actor: Actor::Llm {
                model: "gpt-4o".into(),
                provider: "openai".into(),
            },
            event: StoredEvent::from(tool_called("read_file")),
            relations: vec![],
            stream_id: stream.clone(),
            idempotency_key: None,
        })
        .await
        .unwrap();

    store
        .append(AppendTraceEntry {
            actor: Actor::Llm {
                model: "gpt-4o".into(),
                provider: "openai".into(),
            },
            event: StoredEvent::from(inference_completed()),
            relations: vec![],
            stream_id: stream.clone(),
            idempotency_key: None,
        })
        .await
        .unwrap();

    let page = store
        .scan(TraceQuery {
            event_kind: Some("tool.called".into()),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(1, page.entries.len());
    assert_eq!("tool.called", page.entries[0].event_kind);
}

#[tokio::test]
async fn trace_relations_roundtrip() {
    let store: InMemoryTraceStore<StoredEvent> = InMemoryTraceStore::new();
    let stream = session_stream();

    let first_id = store
        .append(AppendTraceEntry {
            actor: Actor::User,
            event: StoredEvent::from(session_started()),
            relations: vec![],
            stream_id: stream.clone(),
            idempotency_key: None,
        })
        .await
        .unwrap();

    let call_id = ToolCallId::new();
    let second_id = store
        .append(AppendTraceEntry {
            actor: Actor::Llm {
                model: "gpt-4o".into(),
                provider: "openai".into(),
            },
            event: StoredEvent::from(tool_called("bash")),
            relations: vec![TraceRelationDraft {
                to: first_id.clone(),
                kind: TraceRelationKind::CausedBy,
            }],
            stream_id: stream.clone(),
            idempotency_key: None,
        })
        .await
        .unwrap();

    let third_id = store
        .append(AppendTraceEntry {
            actor: Actor::Llm {
                model: "gpt-4o".into(),
                provider: "openai".into(),
            },
            event: StoredEvent::from(tool_completed("bash", call_id)),
            relations: vec![TraceRelationDraft {
                to: second_id.clone(),
                kind: TraceRelationKind::Implements,
            }],
            stream_id: stream.clone(),
            idempotency_key: None,
        })
        .await
        .unwrap();

    // Query relations from second entry
    let rels = store
        .scan_relations(RelationQuery {
            from: Some(second_id.clone()),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(1, rels.len());
    assert_eq!(second_id, rels[0].from);
    assert_eq!(first_id, rels[0].to);
    assert_eq!(TraceRelationKind::CausedBy, rels[0].kind);

    // Query relations from third entry (it points to second)
    let rels = store
        .scan_relations(RelationQuery {
            from: Some(third_id.clone()),
            ..Default::default()
        })
        .await
        .unwrap();

    assert_eq!(1, rels.len());
    assert_eq!(third_id, rels[0].from);
    assert_eq!(second_id, rels[0].to);
    assert_eq!(TraceRelationKind::Implements, rels[0].kind);

    // get_with_relations for second_id finds:
    //   - CausedBy (from second -> first)  [from match]
    //   - Implements (from third -> second) [to match]
    let with_rels = store
        .get_with_relations(second_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(2, with_rels.relations.len());
}

#[tokio::test]
async fn trace_idempotency_key() {
    let store: InMemoryTraceStore<StoredEvent> = InMemoryTraceStore::new();
    let stream = session_stream();
    let key = IdempotencyKey("op-unique-42".into());

    let id1 = store
        .append(AppendTraceEntry {
            actor: Actor::User,
            event: StoredEvent::from(session_started()),
            relations: vec![],
            stream_id: stream.clone(),
            idempotency_key: Some(key.clone()),
        })
        .await
        .unwrap();

    let id2 = store
        .append(AppendTraceEntry {
            actor: Actor::User,
            event: StoredEvent::from(fact_extracted()), // different event!
            relations: vec![],
            stream_id: stream.clone(),
            idempotency_key: Some(key.clone()),
        })
        .await
        .unwrap();

    assert_eq!(id1, id2, "idempotency key must return same TraceId");
    assert_eq!(1, store.current_global_sequence().await.unwrap());
}

#[tokio::test]
async fn trace_hash_chain_valid() {
    let store: InMemoryTraceStore<StoredEvent> = InMemoryTraceStore::new();
    let stream = session_stream();

    let id1 = store
        .append(AppendTraceEntry {
            actor: Actor::User,
            event: StoredEvent::from(session_started()),
            relations: vec![],
            stream_id: stream.clone(),
            idempotency_key: None,
        })
        .await
        .unwrap();

    let id2 = store
        .append(AppendTraceEntry {
            actor: Actor::Llm {
                model: "gpt-4o".into(),
                provider: "openai".into(),
            },
            event: StoredEvent::from(tool_called("read")),
            relations: vec![],
            stream_id: stream.clone(),
            idempotency_key: None,
        })
        .await
        .unwrap();

    let id3 = store
        .append(AppendTraceEntry {
            actor: Actor::Llm {
                model: "gpt-4o".into(),
                provider: "openai".into(),
            },
            event: StoredEvent::from(tool_completed("read", ToolCallId::new())),
            relations: vec![],
            stream_id: stream.clone(),
            idempotency_key: None,
        })
        .await
        .unwrap();

    let e1 = store.get(id1).await.unwrap().unwrap();
    let e2 = store.get(id2).await.unwrap().unwrap();
    let e3 = store.get(id3).await.unwrap().unwrap();

    // First entry: no previous hash
    assert!(e1.prev_hash.is_none());

    // Second entry: links to first
    assert_eq!(Some(e1.entry_hash.clone()), e2.prev_hash);

    // Third entry: links to second
    assert_eq!(Some(e2.entry_hash.clone()), e3.prev_hash);
}

#[tokio::test]
async fn stored_event_deref_exposes_core_methods() {
    let event = OpenWandTraceEvent::Tool(ToolEvent::Denied {
        tool_call_id: ToolCallId::new(),
        tool_name: "rm_rf".into(),
    });
    let stored = StoredEvent::from(event);

    // TraceEventEnvelope trait method
    assert_eq!("tool.denied", stored.event_kind());
    assert_eq!(1, stored.schema_version());

    // Deref to OpenWandTraceEvent
    assert_eq!("tool", stored.0.event_family());
}

#[tokio::test]
async fn trace_event_serde_roundtrip_through_store() {
    let store: InMemoryTraceStore<StoredEvent> = InMemoryTraceStore::new();
    let stream = session_stream();

    let event = OpenWandTraceEvent::Memory(MemoryEvent::FactAccepted {
        claim_id: ClaimId::new(),
        gate_summary: vec![snapshots::GateResultSnapshot {
            gate_kind: "risk_assessment".into(),
            passed: true,
            risk_level: Some(RiskLevelSnapshot::Low),
            reason_code: Some("effect.read".into()),
            summary: "Read-only, auto-allowed".into(),
        }],
    });

    let id = store
        .append(AppendTraceEntry {
            actor: Actor::MemoryPipeline,
            event: StoredEvent::from(event),
            relations: vec![],
            stream_id: stream.clone(),
            idempotency_key: None,
        })
        .await
        .unwrap();

    let entry = store.get(id).await.unwrap().unwrap();

    // Verify the event survived the store round-trip
    assert_eq!("memory.fact_accepted", entry.event_kind);
    assert_eq!(1, entry.global_sequence);

    // Verify we can access the inner event
    match &entry.event.0 {
        OpenWandTraceEvent::Memory(MemoryEvent::FactAccepted { claim_id, gate_summary }) => {
            assert_eq!(1, gate_summary.len());
            assert!(gate_summary[0].passed);
            let _ = claim_id; // just verifying destructuring works
        }
        other => panic!("expected Memory::FactAccepted, got {:?}", other.event_family()),
    }
}
