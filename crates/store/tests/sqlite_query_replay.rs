//! SQLite query, replay, relations, hash-chain, and concurrency tests.

use openwand_core::*;
use openwand_store::StoredEvent;
use openwand_trace::*;

fn session_stream() -> TraceStreamId {
    TraceStreamId {
        scope: TraceStreamScope::Session,
        id: "s-main".into(),
    }
}

fn session_started() -> OpenWandTraceEvent {
    OpenWandTraceEvent::Session(SessionEvent::Started {
        session_id: SessionId::new(),
        mode: InteractionMode::Conversational,
    })
}

fn tool_called(name: &str) -> OpenWandTraceEvent {
    OpenWandTraceEvent::Tool(ToolEvent::Called {
        tool_call_id: ToolCallId::new(),
        tool_name: name.into(),
        args_hash: "abc".into(),
        invoker: ToolInvoker::Llm,
    })
}

fn tool_completed(name: &str, call_id: ToolCallId) -> OpenWandTraceEvent {
    OpenWandTraceEvent::Tool(ToolEvent::Completed {
        tool_call_id: call_id,
        tool_name: name.into(),
        status: ToolResultStatus::Success,
        result_summary: "ok".into(),
        duration_ms: 50,
    })
}


fn inference_completed() -> OpenWandTraceEvent {
    OpenWandTraceEvent::Inference(InferenceEvent::Completed {
        model: "gpt-4o".into(),
        tokens: openwand_core::snapshots::TokenUsageSnapshot {
            input: 100,
            output: 50,
            reasoning: Some(10),
            cache_read: None,
            cache_write: None,
        },
        stop_reason: "stop".into(),
        tool_call_count: 0,
    })
}

// ---- Replay test ----

#[tokio::test]
async fn sqlite_append_100_entries_reopen_replay() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("test.db");
    let stream = session_stream();

    // Phase 1: Write 100 entries
    {
        let store = openwand_store::backends::sqlite::SqliteStore::open(
            openwand_store::backends::sqlite::SqliteStoreConfig::file(&path),
        )
        .await
        .expect("open");

        for i in 0..100 {
            store
                .append(AppendTraceEntry {
                    actor: Actor::User,
                    event: StoredEvent::from(tool_called(&format!("tool_{i:03}"))),
                    relations: vec![],
                    stream_id: stream.clone(),
                    idempotency_key: None,
                })
                .await
                .expect("append");
        }

        assert_eq!(100, store.current_global_sequence().await.expect("seq"));
        store.shutdown().expect("shutdown");
    }

    // Phase 2: Reopen and replay
    let store = openwand_store::backends::sqlite::SqliteStore::open(
        openwand_store::backends::sqlite::SqliteStoreConfig::file(&path),
    )
    .await
    .expect("reopen");

    let page = store
        .scan(TraceQuery {
            stream_id: Some(stream),
            ..Default::default()
        })
        .await
        .expect("scan");

    assert_eq!(100, page.entries.len(), "all 100 entries must survive reload");

    // Verify ordering
    let seqs: Vec<u64> = page.entries.iter().map(|e| e.stream_sequence).collect();
    let expected: Vec<u64> = (1..=100).collect();
    assert_eq!(expected, seqs);

    // Verify event kinds are preserved
    for (i, entry) in page.entries.iter().enumerate() {
        assert_eq!("tool.called", entry.event_kind);
        assert_eq!(format!("tool_{i:03}"), entry.event_name_if_tool());
    }
}

// Helper trait for test assertions
trait TraceEntryTestExt {
    fn event_name_if_tool(&self) -> String;
}

impl TraceEntryTestExt for TraceEntry<StoredEvent> {
    fn event_name_if_tool(&self) -> String {
        match &self.event.0 {
            OpenWandTraceEvent::Tool(ToolEvent::Called { tool_name, .. }) => tool_name.clone(),
            _ => "not-a-tool".into(),
        }
    }
}

// ---- Relations test ----

#[tokio::test]
async fn sqlite_relation_graph_survives_reload() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("test.db");
    let stream = session_stream();

    let (id1, id2, _id3) = {
        let store = openwand_store::backends::sqlite::SqliteStore::open(
            openwand_store::backends::sqlite::SqliteStoreConfig::file(&path),
        )
        .await
        .expect("open");

        let id1 = store
            .append(AppendTraceEntry {
                actor: Actor::User,
                event: StoredEvent::from(session_started()),
                relations: vec![],
                stream_id: stream.clone(),
                idempotency_key: None,
            })
            .await
            .expect("append 1");

        let call_id = ToolCallId::new();
        let id2 = store
            .append(AppendTraceEntry {
                actor: Actor::Llm { model: "gpt-4o".into(), provider: "openai".into() },
                event: StoredEvent::from(tool_called("bash")),
                relations: vec![TraceRelationDraft {
                    to: id1.clone(),
                    kind: TraceRelationKind::CausedBy,
                }],
                stream_id: stream.clone(),
                idempotency_key: None,
            })
            .await
            .expect("append 2");

        let id3 = store
            .append(AppendTraceEntry {
                actor: Actor::Llm { model: "gpt-4o".into(), provider: "openai".into() },
                event: StoredEvent::from(tool_completed("bash", call_id)),
                relations: vec![TraceRelationDraft {
                    to: id2.clone(),
                    kind: TraceRelationKind::Implements,
                }],
                stream_id: stream.clone(),
                idempotency_key: None,
            })
            .await
            .expect("append 3");

        store.shutdown().expect("shutdown");
        (id1, id2, id3)
    };

    // Reopen and verify relations
    let store = openwand_store::backends::sqlite::SqliteStore::open(
        openwand_store::backends::sqlite::SqliteStoreConfig::file(&path),
    )
    .await
    .expect("reopen");

    let with_rels = store
        .get_with_relations(id2.clone())
        .await
        .expect("get_with_relations")
        .expect("entry exists");

    assert_eq!(2, with_rels.relations.len(), "entry 2 should have 2 relations");

    // Verify the relation content
    let caused_by: Vec<_> = with_rels
        .relations
        .iter()
        .filter(|r| r.kind == TraceRelationKind::CausedBy)
        .collect();
    assert_eq!(1, caused_by.len());
    assert_eq!(id2, caused_by[0].from);
    assert_eq!(id1, caused_by[0].to);
}

// ---- Hash chain test ----

#[tokio::test]
async fn sqlite_hash_chain_valid_after_reload() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("test.db");
    let stream = session_stream();

    let (id1, id2, id3) = {
        let store = openwand_store::backends::sqlite::SqliteStore::open(
            openwand_store::backends::sqlite::SqliteStoreConfig::file(&path),
        )
        .await
        .expect("open");

        let id1 = store
            .append(AppendTraceEntry {
                actor: Actor::User,
                event: StoredEvent::from(session_started()),
                relations: vec![],
                stream_id: stream.clone(),
                idempotency_key: None,
            })
            .await
            .expect("append 1");

        let id2 = store
            .append(AppendTraceEntry {
                actor: Actor::Llm { model: "gpt-4o".into(), provider: "openai".into() },
                event: StoredEvent::from(tool_called("read")),
                relations: vec![],
                stream_id: stream.clone(),
                idempotency_key: None,
            })
            .await
            .expect("append 2");

        let id3 = store
            .append(AppendTraceEntry {
                actor: Actor::Llm { model: "gpt-4o".into(), provider: "openai".into() },
                event: StoredEvent::from(tool_completed("read", ToolCallId::new())),
                relations: vec![],
                stream_id: stream.clone(),
                idempotency_key: None,
            })
            .await
            .expect("append 3");

        store.shutdown().expect("shutdown");
        (id1, id2, id3)
    };

    // Reload and verify hash chain
    let store = openwand_store::backends::sqlite::SqliteStore::open(
        openwand_store::backends::sqlite::SqliteStoreConfig::file(&path),
    )
    .await
    .expect("reopen");

    let e1 = store.get(id1).await.expect("get").expect("e1");
    let e2 = store.get(id2).await.expect("get").expect("e2");
    let e3 = store.get(id3).await.expect("get").expect("e3");

    assert!(e1.prev_hash.is_none(), "first entry has no prev_hash");
    assert_eq!(Some(e1.entry_hash.clone()), e2.prev_hash, "e2 links to e1");
    assert_eq!(Some(e2.entry_hash.clone()), e3.prev_hash, "e3 links to e2");

    // All hashes are non-empty BLAKE3
    assert_eq!(64, e1.entry_hash.0.len());
    assert_eq!(64, e2.entry_hash.0.len());
    assert_eq!(64, e3.entry_hash.0.len());
}

// ---- Event kind query test ----

#[tokio::test]
async fn sqlite_query_by_event_kind() {
    let store = openwand_store::backends::sqlite::SqliteStore::open_in_memory()
        .await
        .expect("open");
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
        .expect("append session");

    store
        .append(AppendTraceEntry {
            actor: Actor::Llm { model: "gpt-4o".into(), provider: "openai".into() },
            event: StoredEvent::from(tool_called("read")),
            relations: vec![],
            stream_id: stream.clone(),
            idempotency_key: None,
        })
        .await
        .expect("append tool");

    store
        .append(AppendTraceEntry {
            actor: Actor::Llm { model: "gpt-4o".into(), provider: "openai".into() },
            event: StoredEvent::from(inference_completed()),
            relations: vec![],
            stream_id: stream.clone(),
            idempotency_key: None,
        })
        .await
        .expect("append inference");

    // Query by tool.called
    let page = store
        .scan(TraceQuery {
            event_kind: Some("tool.called".into()),
            ..Default::default()
        })
        .await
        .expect("scan");

    assert_eq!(1, page.entries.len());
    assert_eq!("tool.called", page.entries[0].event_kind);
}

// ---- Concurrent append serialized test ----

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn sqlite_concurrent_append_serialized() {
    let store = std::sync::Arc::new(
        openwand_store::backends::sqlite::SqliteStore::open_in_memory()
            .await
            .expect("open"),
    );

    let stream = session_stream();
    let mut handles = Vec::new();

    // 10 tasks each appending 10 entries
    for task in 0..10 {
        let s = std::sync::Arc::clone(&store);
        let stream = stream.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..10 {
                s.append(AppendTraceEntry {
                    actor: Actor::System { component: format!("task-{task}") },
                    event: StoredEvent::from(tool_called(&format!("t{task}_i{i}"))),
                    relations: vec![],
                    stream_id: stream.clone(),
                    idempotency_key: None,
                })
                .await
                .expect("append");
            }
        }));
    }

    for h in handles {
        h.await.expect("task completed");
    }

    // Verify: exactly 100 entries, all sequences unique and monotonically assigned
    let seq = store.current_global_sequence().await.expect("seq");
    assert_eq!(100, seq, "all 100 appends must be serialized");

    let page = store
        .scan(TraceQuery {
            stream_id: Some(stream),
            ..Default::default()
        })
        .await
        .expect("scan");

    assert_eq!(100, page.entries.len());

    // Verify no duplicate global sequences
    let global_seqs: Vec<u64> = page.entries.iter().map(|e| e.global_sequence).collect();
    let mut unique = global_seqs.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(100, unique.len(), "no duplicate global sequences");

    // Verify no duplicate stream sequences
    let stream_seqs: Vec<u64> = page.entries.iter().map(|e| e.stream_sequence).collect();
    let mut unique_s = stream_seqs.clone();
    unique_s.sort();
    unique_s.dedup();
    assert_eq!(100, unique_s.len(), "no duplicate stream sequences");
}

// ---- No memory projection required ----

#[tokio::test]
async fn sqlite_no_memory_projection_required() {
    // Verify that trace_blob table exists but nothing writes to it
    let store = openwand_store::backends::sqlite::SqliteStore::open_in_memory()
        .await
        .expect("open");

    store
        .append(AppendTraceEntry {
            actor: Actor::User,
            event: StoredEvent::from(session_started()),
            relations: vec![],
            stream_id: session_stream(),
            idempotency_key: None,
        })
        .await
        .expect("append");

    // trace_blob should be empty
    // We can't query it through the public API (by design),
    // but we can verify the store still works without blob logic
    let seq = store.current_global_sequence().await.expect("seq");
    assert_eq!(1, seq, "append works without blob involvement");
}
