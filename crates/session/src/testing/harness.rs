use crate::runner::SessionRunner;
use openwand_core::{SessionId, ToolCallId};
use openwand_llm::LlmClient;
use openwand_memory::MemoryReadStore;
use openwand_policy::PolicyEngine;
use openwand_store::StoredEvent;
use openwand_tools::executor::ToolExecutor;
use openwand_trace::testing::InMemoryTraceStore;
use openwand_trace::TraceStore;
use std::sync::Arc;

pub use super::mock_llm::MockLlmClient;
pub use super::mock_memory::MockMemoryReadStore;
pub use super::mock_policy::{MockPolicyBehavior, MockPolicyEngine};
pub use super::mock_tools::MockToolExecutor;

/// Deterministic test harness for session acceptance tests.
pub struct SessionHarness {
    pub runner: SessionRunner,
    pub trace: Arc<InMemoryTraceStore<StoredEvent>>,
    pub llm: Arc<MockLlmClient>,
    pub policy: Arc<MockPolicyEngine>,
    pub tools: Arc<MockToolExecutor>,
    pub memory: Arc<MockMemoryReadStore>,
}

impl SessionHarness {
    fn build(
        trace: Arc<InMemoryTraceStore<StoredEvent>>,
        llm: Arc<MockLlmClient>,
        tools: Arc<MockToolExecutor>,
        policy: Arc<MockPolicyEngine>,
        memory: Arc<MockMemoryReadStore>,
    ) -> Self {
        let session_id = SessionId::new();

        let runner = SessionRunner::new(
            session_id,
            trace.clone() as Arc<dyn TraceStore<StoredEvent>>,
            llm.clone() as Arc<dyn LlmClient>,
            tools.clone() as Arc<dyn ToolExecutor>,
            policy.clone() as Arc<dyn PolicyEngine>,
            memory.clone() as Arc<dyn MemoryReadStore>,
            ".".into(),
        );

        Self {
            runner,
            trace,
            llm,
            policy,
            tools,
            memory,
        }
    }

    /// Text-only harness: LLM streams "Hello, world." then stops.
    pub fn text_only() -> Self {
        let trace = Arc::new(InMemoryTraceStore::new());
        let llm = Arc::new(MockLlmClient::text_response("Hello, world."));
        let tools = Arc::new(MockToolExecutor::empty());
        let policy = Arc::new(MockPolicyEngine::allow_all());
        let memory = Arc::new(MockMemoryReadStore::new());
        Self::build(trace, llm, tools, policy, memory)
    }

    /// Tool turn harness: LLM requests a tool call, tool returns mock content.
    pub fn read_file_tool_turn() -> Self {
        let trace = Arc::new(InMemoryTraceStore::new());
        let tool_call_id = ToolCallId::new();
        let llm = Arc::new(MockLlmClient::tool_then_stop(
            tool_call_id.as_str().to_string(),
            "local__file_read",
            serde_json::json!({"path": "README.md"}),
        ));
        let tools = Arc::new(MockToolExecutor::with_success(
            "local__file_read",
            "mock file contents",
        ));
        let policy = Arc::new(MockPolicyEngine::allow_all());
        let memory = Arc::new(MockMemoryReadStore::new());
        Self::build(trace, llm, tools, policy, memory)
    }

    /// Tool turn with custom policy behavior.
    pub fn tool_turn_with_policy(behavior: MockPolicyBehavior) -> Self {
        let trace = Arc::new(InMemoryTraceStore::new());
        let tool_call_id = ToolCallId::new();
        let llm = Arc::new(MockLlmClient::tool_then_stop(
            tool_call_id.as_str().to_string(),
            "local__file_read",
            serde_json::json!({"path": "README.md"}),
        ));
        let tools = Arc::new(MockToolExecutor::with_success(
            "local__file_read",
            "mock file contents",
        ));
        let policy = Arc::new(MockPolicyEngine::new(behavior));
        let memory = Arc::new(MockMemoryReadStore::new());
        Self::build(trace, llm, tools, policy, memory)
    }

    /// Tool turn where the tool returns an error result.
    pub fn tool_turn_with_tool_error(tool_name: &str, error_msg: &str) -> Self {
        let trace = Arc::new(InMemoryTraceStore::new());
        let tool_call_id = ToolCallId::new();
        let llm = Arc::new(MockLlmClient::tool_then_stop(
            tool_call_id.as_str().to_string(),
            tool_name,
            serde_json::json!({}),
        ));
        let tools = Arc::new(MockToolExecutor::with_error(tool_name, error_msg));
        let policy = Arc::new(MockPolicyEngine::allow_all());
        let memory = Arc::new(MockMemoryReadStore::new());
        Self::build(trace, llm, tools, policy, memory)
    }

    /// Tool turn with a write tool that requires confirmation.
    /// Policy: local__file_write requires confirmation, all others allowed.
    pub fn write_tool_requires_confirmation() -> Self {
        let trace = Arc::new(InMemoryTraceStore::new());
        let tool_call_id = ToolCallId::new();
        let llm = Arc::new(MockLlmClient::tool_then_stop(
            tool_call_id.as_str().to_string(),
            "local__file_write",
            serde_json::json!({"path": "test.txt", "content": "hello"}),
        ));
        let tools = Arc::new(MockToolExecutor::with_success(
            "local__file_write",
            "Wrote 5 bytes to test.txt",
        ));
        let policy = Arc::new(MockPolicyEngine::require_confirmation_for("local__file_write"));
        let memory = Arc::new(MockMemoryReadStore::new());
        Self::build(trace, llm, tools, policy, memory)
    }

    /// Multi-tool batch: LLM emits write_A, write_B, read_C.
    /// Policy: write_A and write_B require confirmation, read_C allowed.
    /// Only write_A should suspend; write_B and read_C should NOT execute.
    pub fn multi_tool_batch() -> Self {
        let trace = Arc::new(InMemoryTraceStore::new());
        let id_a = ToolCallId::new();
        let id_b = ToolCallId::new();
        let id_c = ToolCallId::new();
        let llm = Arc::new(MockLlmClient::multi_tool_then_stop(vec![
            (id_a.as_str().to_string(), "local__write_A".into(), serde_json::json!({"path": "a.txt"})),
            (id_b.as_str().to_string(), "local__write_B".into(), serde_json::json!({"path": "b.txt"})),
            (id_c.as_str().to_string(), "local__read_C".into(), serde_json::json!({"path": "c.txt"})),
        ]));
        let tools = Arc::new(MockToolExecutor::with_many(&["write_A", "write_B", "read_C"]));
        let policy = Arc::new(MockPolicyEngine::require_confirmation_for_many(
            vec!["local__write_A", "local__write_B"],
        ));
        let memory = Arc::new(MockMemoryReadStore::new());
        Self::build(trace, llm, tools, policy, memory)
    }

    /// Multi-turn: tool call on first turn, text on second.
    /// For testing rejection→continuation.
    pub fn write_tool_then_text_after_denial() -> Self {
        let trace = Arc::new(InMemoryTraceStore::new());
        let tool_call_id = ToolCallId::new();
        let llm = Arc::new(MockLlmClient::tool_then_text_after_denial(
            tool_call_id.as_str().to_string(),
            "local__file_write",
            serde_json::json!({"path": "test.txt", "content": "hello"}),
            "Understood, I won't write that file. Is there something else I can help with?",
        ));
        let tools = Arc::new(MockToolExecutor::with_success(
            "local__file_write",
            "Wrote 5 bytes to test.txt",
        ));
        let policy = Arc::new(MockPolicyEngine::require_confirmation_for("local__file_write"));
        let memory = Arc::new(MockMemoryReadStore::new());
        Self::build(trace, llm, tools, policy, memory)
    }
}
