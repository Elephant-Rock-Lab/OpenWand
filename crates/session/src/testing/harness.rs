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
}
