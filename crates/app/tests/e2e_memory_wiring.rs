//! End-to-end wiring test.
//!
//! Proves the REAL lock condition:
//! "A user can say 'remember X,' finish the run, see X appear in the Memory
//!  panel automatically, then start a later run where X is retrieved into the
//!  prompt without manual database work."
//!
//! This test uses real SQLite (tempfile), real SqliteMemoryStore,
//! real MemoryCoordinator, real HeuristicExtractor.

use openwand_app::memory_coordinator::MemoryCoordinator;
use openwand_app::ui::memory_service::build_memory_panel;
use openwand_core::events::{InferenceEvent, OpenWandTraceEvent, SessionEvent};
use openwand_core::SessionId;
use openwand_memory::testing::HeuristicExtractor;
use openwand_memory::{MemoryExtractor, MemoryQuery, MemoryReadStore, MemoryStore, SqliteMemoryStore};
use openwand_store::backends::sqlite::{SqliteStore, SqliteStoreConfig};
use openwand_store::StoredEvent;
use openwand_trace::{Actor, TraceEntry, TraceStreamId, TraceStreamScope, TraceStore};
use chrono::Utc;
use std::sync::Arc;

/// Simulates the full lifecycle:
/// 1. User says "remember X"
/// 2. Trace entries are recorded
/// 3. Coordinator projects episodes + extracts
/// 4. Memory panel shows X
/// 5. Next run retrieves X via memory.search()
#[tokio::test]
async fn e2e_remember_x_appears_in_memory_and_is_retrieved_later() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("e2e_test.db");

    // 1. Open real SQLite stores
    let trace_store: Arc<dyn TraceStore<StoredEvent>> = Arc::new(
        SqliteStore::open(SqliteStoreConfig::file(&db_path))
            .await
            .unwrap(),
    );
    let memory_store = Arc::new(
        SqliteMemoryStore::open(&db_path).unwrap(),
    );
    let memory_read: Arc<dyn MemoryReadStore> = memory_store.clone();
    let memory_write: Arc<dyn MemoryStore> = memory_store.clone();

    let session_id = SessionId::new();
    let stream_id = TraceStreamId {
        scope: TraceStreamScope::Session,
        id: session_id.to_string(),
    };

    // 2. Simulate a run that produces trace entries
    //    (In a real run, SessionRunner does this via MutationHelper)
    use openwand_trace::AppendTraceEntry;

    let user_entry = AppendTraceEntry {
        actor: Actor::User,
        event: StoredEvent::from(OpenWandTraceEvent::Session(SessionEvent::UserMessageInjected {
            text: "Remember that I always use Rust for new projects".into(),
        })),
        relations: vec![],
        stream_id: stream_id.clone(),
        idempotency_key: None,
    };
    trace_store.append(user_entry).await.unwrap();

    let assistant_entry = AppendTraceEntry {
        actor: Actor::Llm { model: "test".into(), provider: "test".into() },
        event: StoredEvent::from(OpenWandTraceEvent::Session(SessionEvent::AssistantMessageGenerated {
            text: "I'll remember that you use Rust for new projects.".into(),
            model: "test".into(),
        })),
        relations: vec![],
        stream_id: stream_id.clone(),
        idempotency_key: None,
    };
    trace_store.append(assistant_entry).await.unwrap();

    // 3. Run memory coordinator (automatic after-run projection)
    let trace_for_coordinator: Arc<dyn TraceStore<StoredEvent>> = Arc::new(
        SqliteStore::open(SqliteStoreConfig::file(&db_path))
            .await
            .unwrap(),
    );
    let extractor: Arc<dyn MemoryExtractor> = Arc::new(HeuristicExtractor);
    let coordinator = MemoryCoordinator::new(
        memory_write.clone(),
        extractor,
        trace_for_coordinator,
    );

    let projection = coordinator.project_after_run(&session_id).await;

    // Must have projected episodes
    assert!(projection.episodes_projected >= 1,
        "Expected >= 1 episode, got {}", projection.episodes_projected);
    assert!(projection.records_accepted >= 1,
        "Expected >= 1 accepted record, got {}", projection.records_accepted);
    assert!(projection.errors.is_empty(),
        "Unexpected errors: {:?}", projection.errors);

    // 4. Memory panel shows the record
    let panel = build_memory_panel(&*memory_store).await.unwrap();
    assert!(panel.active_count >= 1, "Memory panel should show >= 1 records");
    assert!(
        panel.records.iter().any(|r| r.claim.contains("Rust")),
        "Expected a record mentioning Rust, got: {:?}",
        panel.records.iter().map(|r| &r.claim).collect::<Vec<_>>()
    );
    assert!(
        panel.records.iter().any(|r| r.source_trace_ids.len() > 0),
        "Expected records with source trace IDs"
    );

    // 5. Next run: memory retrieval finds "Rust"
    let ctx = memory_read
        .search(MemoryQuery::new("Rust"))
        .await
        .unwrap();

    assert!(!ctx.is_empty(), "Memory search for 'Rust' should return results");
    assert!(
        ctx.facts.iter().any(|f| f.contains("Rust")),
        "Expected fact about Rust, got: {:?}", ctx.facts
    );

    // 6. Memory context can be formatted for prompt injection
    let block = ctx.to_context_block();
    assert!(block.is_some());
    let text = block.unwrap();
    assert!(text.contains("Rust"), "Context block should mention Rust");

    // 7. Verify durability: reopen and search still works
    drop(memory_store);
    let memory_reopened = Arc::new(
        SqliteMemoryStore::open(&db_path).unwrap(),
    );
    let ctx2 = memory_reopened
        .search_records(MemoryQuery::new("Rust"))
        .await
        .unwrap();
    assert!(!ctx2.is_empty(), "Memory should survive reopen");
}

#[tokio::test]
async fn e2e_memory_coordinator_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("e2e_idem.db");

    let trace_store: Arc<dyn TraceStore<StoredEvent>> = Arc::new(
        SqliteStore::open(SqliteStoreConfig::file(&db_path))
            .await
            .unwrap(),
    );
    let memory_store = Arc::new(SqliteMemoryStore::open(&db_path).unwrap());
    let memory_write: Arc<dyn MemoryStore> = memory_store.clone();

    let session_id = SessionId::new();
    let stream_id = TraceStreamId {
        scope: TraceStreamScope::Session,
        id: session_id.to_string(),
    };

    // Record a trace entry
    use openwand_trace::AppendTraceEntry;

    trace_store
        .append(AppendTraceEntry {
            actor: Actor::User,
            event: StoredEvent::from(OpenWandTraceEvent::Session(SessionEvent::UserMessageInjected {
                text: "Remember I prefer dark mode".into(),
            })),
            relations: vec![],
            stream_id,
            idempotency_key: None,
        })
        .await
        .unwrap();

    let trace_for_coord: Arc<dyn TraceStore<StoredEvent>> = Arc::new(
        SqliteStore::open(SqliteStoreConfig::file(&db_path))
            .await
            .unwrap(),
    );
    let extractor: Arc<dyn MemoryExtractor> = Arc::new(HeuristicExtractor);
    let coord = MemoryCoordinator::new(memory_write, extractor, trace_for_coord);

    // Project twice
    let r1 = coord.project_after_run(&session_id).await;
    let r2 = coord.project_after_run(&session_id).await;

    // First projection creates records
    assert!(r1.records_accepted >= 1, "First projection should accept >= 1");

    // Idempotency: still only 1 record in the store (duplicate attached, not created)
    let active = memory_store.list_active_records().await.unwrap();
    assert_eq!(1, active.len(), "Should have exactly 1 record after 2 projections");
}
