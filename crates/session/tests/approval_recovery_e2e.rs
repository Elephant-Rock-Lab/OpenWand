//! Wave 03d Batch D — SQLite recovery E2E tests.
//!
//! Tests crash-recoverable approval governance using real SQLite trace store.
//! Pattern: create session → suspend → drop runner → reopen → rebuild → resolve.

use openwand_core::mode::InteractionMode;
use openwand_core::SessionId;
use openwand_llm::LlmClient;
use openwand_memory::MemoryReadStore;
use openwand_session::approval_recovery::ApprovalRecoveryConflict;
use openwand_session::config::RunConfig;
use openwand_session::persistence::rebuild_approval_state;
use openwand_session::runner::SessionRunner;
use openwand_session::testing::mock_llm::MockLlmClient;
use openwand_session::testing::mock_memory::MockMemoryReadStore;
use openwand_session::testing::mock_policy::MockPolicyEngine;
use openwand_session::testing::mock_tools::MockToolExecutor;
use openwand_session::ApprovalDecision;
use openwand_store::StoredEvent;
use openwand_trace::TraceStore;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

fn conversational_config() -> RunConfig {
    RunConfig {
        mode: InteractionMode::Conversational,
        ..Default::default()
    }
}

fn temp_sqlite_path() -> std::path::PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let thread_id = std::thread::current().id();
    let dir = std::env::temp_dir().join(format!("openwand-03d-{n:?}-{thread_id:?}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("trace.db")
}

async fn open_sqlite_at(path: &std::path::Path) -> Arc<dyn TraceStore<StoredEvent>> {
    let config = openwand_store::backends::sqlite::SqliteStoreConfig::file(path);
    let store = openwand_store::backends::sqlite::SqliteStore::open(config)
        .await
        .expect("open sqlite");
    Arc::new(store) as Arc<dyn TraceStore<StoredEvent>>
}

fn cleanup(path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::remove_dir_all(parent);
    }
}

// ---- Case A: Crash before user decision → reconstruct UI ----

#[tokio::test]
async fn sqlite_recovery_case_a_crash_before_decision() {
    let db_path = temp_sqlite_path();
    let trace = open_sqlite_at(&db_path).await;

    let llm = Arc::new(MockLlmClient::tool_then_stop(
        "tc_1".into(),
        "local__file_write",
        serde_json::json!({"path": "test.txt", "content": "hello"}),
    ));
    let tools = Arc::new(MockToolExecutor::with_success("local__file_write", "Wrote 5 bytes"));
    let policy = Arc::new(MockPolicyEngine::require_confirmation_for("local__file_write"));
    let memory = Arc::new(MockMemoryReadStore::new());

    {
        let runner = SessionRunner::new(
            SessionId::new(),
            trace.clone(),
            llm, tools, policy, memory,
            ".".into(),
        );

        runner
            .run_turn("Write a file".into(), conversational_config())
            .await
            .unwrap();
    }
    // Runner dropped — simulated crash
    drop(trace);

    // Reopen
    let trace2 = open_sqlite_at(&db_path).await;
    let doc = loro::LoroDoc::new();
    let loro = openwand_session::loro_state::LoroSessionState::new(&doc);

    let index = rebuild_approval_state(trace2.as_ref(), &loro).await.unwrap();

    assert_eq!(1, index.pending.len());
    assert_eq!("local__file_write", index.pending[0].tool_name);

    let ui = loro.get_waiting_approval().unwrap();
    assert!(ui.is_some());
    assert_eq!("local__file_write", ui.unwrap().tool_name);
    assert!(!loro.is_recovery_blocked().unwrap());

    cleanup(&db_path);
}

// ---- Crash → restart → approve ----

#[tokio::test]
async fn sqlite_recovery_crash_restart_approve() {
    let db_path = temp_sqlite_path();
    let trace = open_sqlite_at(&db_path).await;
    let session_id = SessionId::new();

    {
        let llm = Arc::new(MockLlmClient::tool_then_stop(
            "tc_1".into(),
            "local__file_write",
            serde_json::json!({"path": "test.txt", "content": "hello"}),
        ));
        let tools = Arc::new(MockToolExecutor::with_success("local__file_write", "Wrote 5 bytes"));
        let policy = Arc::new(MockPolicyEngine::require_confirmation_for("local__file_write"));
        let memory = Arc::new(MockMemoryReadStore::new());

        let runner = SessionRunner::new(
            session_id.clone(),
            trace.clone(),
            llm, tools, policy, memory,
            ".".into(),
        );

        runner
            .run_turn("Write a file".into(), conversational_config())
            .await
            .unwrap();
    }
    drop(trace);

    // Reopen and create new runner
    let trace2 = open_sqlite_at(&db_path).await;
    let tools2 = Arc::new(MockToolExecutor::with_success("local__file_write", "Wrote 5 bytes"));

    let runner2 = SessionRunner::new(
        session_id,
        trace2,
        Arc::new(MockLlmClient::text_response("Done")),
        tools2.clone(),
        Arc::new(MockPolicyEngine::require_confirmation_for("local__file_write")),
        Arc::new(MockMemoryReadStore::new()),
        ".".into(),
    );

    let result = runner2
        .resolve_recovered_approval(ApprovalDecision::Approved, conversational_config())
        .await
        .unwrap();

    assert_eq!(ApprovalDecision::Approved, result.decision);
    assert_eq!("local__file_write", result.tool_name);
    assert_eq!(1, tools2.execution_count().await);

    let calls = tools2.calls().await;
    assert_eq!("local__file_write", calls[0].name);

    cleanup(&db_path);
}

// ---- Crash → restart → reject ----

#[tokio::test]
async fn sqlite_recovery_crash_restart_reject() {
    let db_path = temp_sqlite_path();
    let trace = open_sqlite_at(&db_path).await;
    let session_id = SessionId::new();

    {
        let llm = Arc::new(MockLlmClient::tool_then_stop(
            "tc_1".into(),
            "local__file_write",
            serde_json::json!({"path": "test.txt", "content": "hello"}),
        ));
        let tools = Arc::new(MockToolExecutor::with_success("local__file_write", "Wrote 5 bytes"));
        let policy = Arc::new(MockPolicyEngine::require_confirmation_for("local__file_write"));
        let memory = Arc::new(MockMemoryReadStore::new());

        let runner = SessionRunner::new(
            session_id.clone(),
            trace.clone(),
            llm, tools, policy, memory,
            ".".into(),
        );

        runner
            .run_turn("Write a file".into(), conversational_config())
            .await
            .unwrap();
    }
    drop(trace);

    let trace2 = open_sqlite_at(&db_path).await;
    let tools2 = Arc::new(MockToolExecutor::with_success("local__file_write", "Wrote 5 bytes"));

    let runner2 = SessionRunner::new(
        session_id,
        trace2,
        Arc::new(MockLlmClient::text_response("OK")),
        tools2.clone(),
        Arc::new(MockPolicyEngine::require_confirmation_for("local__file_write")),
        Arc::new(MockMemoryReadStore::new()),
        ".".into(),
    );

    let result = runner2
        .resolve_recovered_approval(ApprovalDecision::Rejected, conversational_config())
        .await
        .unwrap();

    assert_eq!(ApprovalDecision::Rejected, result.decision);
    assert!(result.tool_result.is_none());
    assert_eq!(0, tools2.execution_count().await);

    cleanup(&db_path);
}

// ---- Old suspended without context → recovery blocked ----

#[tokio::test]
async fn sqlite_recovery_old_suspended_without_context_is_recovery_blocked() {
    let db_path = temp_sqlite_path();
    let trace = open_sqlite_at(&db_path).await;

    use openwand_core::events::{OpenWandTraceEvent, ToolEvent};
    use openwand_trace::actor::Actor;
    use openwand_trace::append::AppendTraceEntry;
    use openwand_trace::stream::{TraceStreamId, TraceStreamScope};

    let old_event = StoredEvent::from(OpenWandTraceEvent::Tool(ToolEvent::Suspended {
        tool_call_id: openwand_core::ToolCallId::new(),
        tool_name: "local__file_write".into(),
        reason: "awaiting approval".into(),
        approval_context: None,
    }));

    trace
        .append(AppendTraceEntry {
            actor: Actor::System { component: "gate".into() },
            event: old_event,
            relations: vec![],
            stream_id: TraceStreamId { scope: TraceStreamScope::Session, id: "old".into() },
            idempotency_key: None,
        })
        .await
        .unwrap();

    let doc = loro::LoroDoc::new();
    let loro = openwand_session::loro_state::LoroSessionState::new(&doc);

    let index = rebuild_approval_state(trace.as_ref(), &loro).await.unwrap();

    assert!(loro.is_recovery_blocked().unwrap());
    assert!(index.pending.is_empty());
    assert!(index.conflicts.iter().any(|c| matches!(
        c,
        ApprovalRecoveryConflict::SuspendedMissingApprovalContext { .. }
    )));
    assert!(loro.get_waiting_approval().unwrap().is_none());

    cleanup(&db_path);
}

// ---- Deferred tools not reconstructed as pending ----

#[tokio::test]
async fn sqlite_recovery_deferred_tools_not_reconstructed_as_pending() {
    let db_path = temp_sqlite_path();
    let trace = open_sqlite_at(&db_path).await;

    use openwand_core::events::{OpenWandTraceEvent, ToolEvent};
    use openwand_trace::actor::Actor;
    use openwand_trace::append::AppendTraceEntry;
    use openwand_trace::stream::{TraceStreamId, TraceStreamScope};

    let deferred_event = StoredEvent::from(OpenWandTraceEvent::Tool(ToolEvent::Deferred {
        tool_call_id: openwand_core::ToolCallId::new(),
        tool_name: "local__write_B".into(),
        reason: "deferred: another approval pending".into(),
        blocked_by_tool_call_id: None,
        blocked_by_approval_request_id: None,
        original_order_index: Some(1),
        args_hash: Some("h2".into()),
    }));

    trace
        .append(AppendTraceEntry {
            actor: Actor::System { component: "gate".into() },
            event: deferred_event,
            relations: vec![],
            stream_id: TraceStreamId { scope: TraceStreamScope::Session, id: "def".into() },
            idempotency_key: None,
        })
        .await
        .unwrap();

    let doc = loro::LoroDoc::new();
    let loro = openwand_session::loro_state::LoroSessionState::new(&doc);

    let index = rebuild_approval_state(trace.as_ref(), &loro).await.unwrap();

    assert!(index.pending.is_empty());
    assert_eq!(1, index.deferred.len());
    assert_eq!("local__write_B", index.deferred[0].tool_name);
    assert!(!loro.is_recovery_blocked().unwrap());

    cleanup(&db_path);
}

// ---- tool.deferred contains blocking approval request id ----

#[tokio::test]
async fn sqlite_recovery_tool_deferred_contains_blocking_id() {
    let db_path = temp_sqlite_path();
    let trace = open_sqlite_at(&db_path).await;

    use openwand_core::events::{OpenWandTraceEvent, ToolEvent};
    use openwand_core::ids::ApprovalRequestId;
    use openwand_trace::actor::Actor;
    use openwand_trace::append::AppendTraceEntry;
    use openwand_trace::stream::{TraceStreamId, TraceStreamScope};

    let ar_id = ApprovalRequestId::new();

    let deferred_event = StoredEvent::from(OpenWandTraceEvent::Tool(ToolEvent::Deferred {
        tool_call_id: openwand_core::ToolCallId::new(),
        tool_name: "local__write_B".into(),
        reason: "deferred".into(),
        blocked_by_tool_call_id: None,
        blocked_by_approval_request_id: Some(ar_id.clone()),
        original_order_index: Some(2),
        args_hash: Some("hash_B".into()),
    }));

    trace
        .append(AppendTraceEntry {
            actor: Actor::System { component: "gate".into() },
            event: deferred_event,
            relations: vec![],
            stream_id: TraceStreamId { scope: TraceStreamScope::Session, id: "def2".into() },
            idempotency_key: None,
        })
        .await
        .unwrap();

    let doc = loro::LoroDoc::new();
    let loro = openwand_session::loro_state::LoroSessionState::new(&doc);

    let index = rebuild_approval_state(trace.as_ref(), &loro).await.unwrap();

    assert_eq!(1, index.deferred.len());
    assert_eq!(Some(ar_id), index.deferred[0].blocked_by_approval_request_id);
    assert_eq!(Some(2), index.deferred[0].original_order_index);

    cleanup(&db_path);
}
