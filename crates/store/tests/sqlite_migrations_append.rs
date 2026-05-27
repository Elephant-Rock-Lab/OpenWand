//! SQLite migration and append tests.

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

// ---- Migration tests ----

#[tokio::test]
async fn sqlite_migrations_create_trace_schema() {
    let store = openwand_store::backends::sqlite::SqliteStore::open_in_memory()
        .await
        .expect("open");

    // Initialize should succeed (migrations ran at open)
    store
        .initialize()
        .await
        .expect("initialize after open should succeed");

    // Append should work
    let id = store
        .append(AppendTraceEntry {
            actor: Actor::User,
            event: StoredEvent::from(session_started()),
            relations: vec![],
            stream_id: session_stream(),
            idempotency_key: None,
        })
        .await
        .expect("append");

    assert_eq!(26, id.0.len());
}

#[tokio::test]
async fn sqlite_migrations_are_idempotent() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("test.db");

    // Open twice with same path
    {
        let store = openwand_store::backends::sqlite::SqliteStore::open(
            openwand_store::backends::sqlite::SqliteStoreConfig::file(&path),
        )
        .await
        .expect("first open");
        store
            .append(AppendTraceEntry {
                actor: Actor::User,
                event: StoredEvent::from(session_started()),
                relations: vec![],
                stream_id: session_stream(),
                idempotency_key: None,
            })
            .await
            .expect("append on first open");
        store.shutdown().expect("shutdown");
    }

    {
        let store = openwand_store::backends::sqlite::SqliteStore::open(
            openwand_store::backends::sqlite::SqliteStoreConfig::file(&path),
        )
        .await
        .expect("second open should succeed — migrations are idempotent");

        // Should still see the data
        let seq = store.current_global_sequence().await.expect("seq");
        assert_eq!(1, seq, "data from first open should persist");
        store.shutdown().expect("shutdown");
    }
}

// ---- Append tests ----

#[tokio::test]
async fn sqlite_append_one_entry() {
    let store = openwand_store::backends::sqlite::SqliteStore::open_in_memory()
        .await
        .expect("open");

    let id = store
        .append(AppendTraceEntry {
            actor: Actor::User,
            event: StoredEvent::from(session_started()),
            relations: vec![],
            stream_id: session_stream(),
            idempotency_key: None,
        })
        .await
        .expect("append");

    // Verify we can get it back
    let entry = store.get(id).await.expect("get").expect("entry exists");
    assert_eq!("session.started", entry.event_kind);
    assert_eq!(1, entry.global_sequence);
    assert_eq!(1, entry.stream_sequence);
    assert!(entry.prev_hash.is_none(), "first entry has no prev_hash");
    assert!(!entry.entry_hash.0.is_empty(), "entry_hash must be populated");
}

#[tokio::test]
async fn sqlite_stream_sequence_monotonic() {
    let store = openwand_store::backends::sqlite::SqliteStore::open_in_memory()
        .await
        .expect("open");
    let stream = session_stream();

    // Append 10 entries
    for i in 0..10 {
        store
            .append(AppendTraceEntry {
                actor: Actor::User,
                event: StoredEvent::from(tool_called(&format!("tool_{i}"))),
                relations: vec![],
                stream_id: stream.clone(),
                idempotency_key: None,
            })
            .await
            .expect("append");
    }

    assert_eq!(10, store.current_global_sequence().await.expect("global"));
    assert_eq!(
        10,
        store.current_stream_sequence(&stream).await.expect("stream")
    );

    // Verify monotonicity
    let page = store
        .scan(TraceQuery {
            stream_id: Some(stream),
            ..Default::default()
        })
        .await
        .expect("scan");

    let sequences: Vec<u64> = page.entries.iter().map(|e| e.stream_sequence).collect();
    let expected: Vec<u64> = (1..=10).collect();
    assert_eq!(expected, sequences, "stream sequences must be monotonically increasing from 1");
}

#[tokio::test]
async fn sqlite_idempotency_key_survives_reload() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("test.db");

    let id = {
        let store = openwand_store::backends::sqlite::SqliteStore::open(
            openwand_store::backends::sqlite::SqliteStoreConfig::file(&path),
        )
        .await
        .expect("open");

        let id = store
            .append(AppendTraceEntry {
                actor: Actor::User,
                event: StoredEvent::from(session_started()),
                relations: vec![],
                stream_id: session_stream(),
                idempotency_key: Some(IdempotencyKey("unique-op-42".into())),
            })
            .await
            .expect("append");

        // Same key should return same ID
        let id2 = store
            .append(AppendTraceEntry {
                actor: Actor::User,
                event: StoredEvent::from(tool_called("other")), // different event!
                relations: vec![],
                stream_id: session_stream(),
                idempotency_key: Some(IdempotencyKey("unique-op-42".into())),
            })
            .await
            .expect("append idempotent");

        assert_eq!(id, id2, "same idempotency key → same TraceId");
        assert_eq!(1, store.current_global_sequence().await.expect("seq"));

        store.shutdown().expect("shutdown");
        id
    };

    // Reload and verify
    let store = openwand_store::backends::sqlite::SqliteStore::open(
        openwand_store::backends::sqlite::SqliteStoreConfig::file(&path),
    )
    .await
    .expect("reopen");

    let id3 = store
        .append(AppendTraceEntry {
            actor: Actor::User,
            event: StoredEvent::from(tool_called("yet_another")),
            relations: vec![],
            stream_id: session_stream(),
            idempotency_key: Some(IdempotencyKey("unique-op-42".into())),
        })
        .await
        .expect("append after reload");

    assert_eq!(id, id3, "idempotency key survives reload");
    assert_eq!(1, store.current_global_sequence().await.expect("seq"));
}
