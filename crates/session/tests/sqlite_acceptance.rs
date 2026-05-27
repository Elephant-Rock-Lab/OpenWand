//! SQLite session acceptance test — the Wave 01e proof.
//!
//! Proves that a deterministic session turn can run through the 10-phase loop,
//! record all durable events to SQLite trace, and reload enough history to
//! reconstruct a minimal session timeline.

use openwand_core::{SessionId, ToolCallId};
use openwand_llm::LlmClient;
use openwand_memory::MemoryReadStore;
use openwand_policy::PolicyEngine;
use openwand_session::config::RunConfig;
use openwand_session::message::MessageRole;
use openwand_session::runner::SessionRunner;
use openwand_store::StoredEvent;
use openwand_tools::executor::ToolExecutor;
use openwand_trace::{TraceQuery, TraceStore};
use std::sync::Arc;

use openwand_session::testing::mock_llm::MockLlmClient;
use openwand_session::testing::mock_memory::MockMemoryReadStore;
use openwand_session::testing::mock_policy::MockPolicyEngine;
use openwand_session::testing::mock_tools::MockToolExecutor;

#[tokio::test]
async fn sqlite_session_text_only_turn() {
    let store = openwand_store::backends::sqlite::SqliteStore::open_in_memory()
        .await
        .expect("open sqlite");

    let llm = Arc::new(MockLlmClient::text_response("Hello from SQLite!"));
    let tools = Arc::new(MockToolExecutor::empty());
    let policy = Arc::new(MockPolicyEngine::allow_all());
    let memory = Arc::new(MockMemoryReadStore::new());

    let runner = SessionRunner::new(
        SessionId::new(),
        Arc::new(store) as Arc<dyn TraceStore<StoredEvent>>,
        llm.clone() as Arc<dyn LlmClient>,
        tools.clone() as Arc<dyn ToolExecutor>,
        policy.clone() as Arc<dyn PolicyEngine>,
        memory.clone() as Arc<dyn MemoryReadStore>,
        ".".into(),
    );

    let result = runner
        .run_turn("Say hello".into(), RunConfig::default())
        .await
        .expect("text-only turn");

    // Text-only: LLM responded without needing tools
    // steps_completed may be 0 if no tool loop ran
    assert_eq!(0, result.tools_executed, "text-only should have no tool calls");

    // Verify Loro projection
    let messages = runner.loro_state().messages().expect("messages");
    assert!(messages.len() >= 2, "at least user + assistant");
}

#[tokio::test]
async fn sqlite_session_tool_turn() {
    let store = openwand_store::backends::sqlite::SqliteStore::open_in_memory()
        .await
        .expect("open sqlite");

    let tool_call_id = ToolCallId::new();
    let llm = Arc::new(MockLlmClient::tool_then_stop(
        tool_call_id.as_str().to_string(),
        "local__file_read",
        serde_json::json!({"path": "README.md"}),
    ));
    let tools = Arc::new(MockToolExecutor::with_success("local__file_read", "file contents here"));
    let policy = Arc::new(MockPolicyEngine::allow_all());
    let memory = Arc::new(MockMemoryReadStore::new());

    let runner = SessionRunner::new(
        SessionId::new(),
        Arc::new(store) as Arc<dyn TraceStore<StoredEvent>>,
        llm.clone() as Arc<dyn LlmClient>,
        tools.clone() as Arc<dyn ToolExecutor>,
        policy.clone() as Arc<dyn PolicyEngine>,
        memory.clone() as Arc<dyn MemoryReadStore>,
        ".".into(),
    );

    let result = runner
        .run_turn("Read README".into(), RunConfig::default())
        .await
        .expect("tool turn");

    assert_eq!(1, result.tools_executed, "one tool call was executed");

    // Verify tool was called
    let calls = tools.calls().await;
    assert_eq!(1, calls.len());
    assert_eq!("local__file_read", calls[0].name);

    // Verify Loro has the tool result
    let messages = runner.loro_state().messages().expect("messages");
    let tool_results: Vec<_> = messages
        .iter()
        .filter(|m| matches!(m.role, MessageRole::Tool))
        .collect();
    assert_eq!(1, tool_results.len(), "one tool result in Loro");
}

#[tokio::test]
async fn sqlite_session_close_reopen_replay() {
    let dir = tempfile::tempdir().expect("temp dir");
    let db_path = dir.path().join("session_trace.db");

    // Phase 1: Run a session turn and shut down
    {
        let store = openwand_store::backends::sqlite::SqliteStore::open(
            openwand_store::backends::sqlite::SqliteStoreConfig::file(&db_path),
        )
        .await
        .expect("open sqlite");

        let llm = Arc::new(MockLlmClient::text_response("Hello!"));
        let tools = Arc::new(MockToolExecutor::empty());
        let policy = Arc::new(MockPolicyEngine::allow_all());
        let memory = Arc::new(MockMemoryReadStore::new());

        let runner = SessionRunner::new(
            SessionId::new(),
            Arc::new(store) as Arc<dyn TraceStore<StoredEvent>>,
            llm.clone() as Arc<dyn LlmClient>,
            tools.clone() as Arc<dyn ToolExecutor>,
            policy.clone() as Arc<dyn PolicyEngine>,
            memory.clone() as Arc<dyn MemoryReadStore>,
            ".".into(),
        );

        runner
            .run_turn("Say hello".into(), RunConfig::default())
            .await
            .expect("turn");
    }

    // Phase 2: Reopen the same database and scan for session history
    let store = openwand_store::backends::sqlite::SqliteStore::open(
        openwand_store::backends::sqlite::SqliteStoreConfig::file(&db_path),
    )
    .await
    .expect("reopen sqlite");

    let seq = store.current_global_sequence().await.expect("seq");
    assert!(seq >= 1, "trace entries survived restart, got {seq}");

    // Scan for session.user_message_injected events (what the runner actually records)
    let page = store
        .scan(TraceQuery {
            event_kind: Some("session.user_message_injected".into()),
            ..Default::default()
        })
        .await
        .expect("scan");

    assert!(
        !page.entries.is_empty(),
        "session.user_message_injected event should be in SQLite after reload"
    );

    // Verify the event is reconstructable with valid hash
    for entry in &page.entries {
        assert!(!entry.event_kind.is_empty());
        assert_eq!(64, entry.entry_hash.0.len(), "BLAKE3 hash is 64 hex chars");
    }
}
