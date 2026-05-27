# WAVE 01D LOCK — Session + Mocks

**Status:** COMPLETE  
**Commits:** 27 (single batch)  
**Date:** 2026-05-27  
**Tests:** 167 workspace total (session: 7 unit + 11 acceptance + 2 guards)  
**Warnings:** zero

---

## 1. Crate Summary

`openwand-session` — the agent loop crate. Orchestrates the 10-phase cycle:
RunStart → StepStart → BeforeInference → Inference → AfterInference → ToolGate → BeforeToolExecute → AfterToolExecute → StepEnd → RunEnd

### Files Added/Modified

| File | Purpose |
|------|---------|
| `src/lib.rs` | Crate root, re-exports, `testing` feature gate |
| `src/error.rs` | `SessionError` enum (Trace, Llm, Policy, Tool, Projection, RunAlreadyActive, Cancelled) |
| `src/phase.rs` | `Phase` enum (10 phases) |
| `src/config.rs` | `RunConfig`, `RunSummary`, `RunStopReason` |
| `src/message.rs` | `Message`, `MessageRole`, `MessageContent` |
| `src/tool.rs` | `ToolCall`, `ToolResult` session-side DTOs |
| `src/agent_event.rs` | `AgentEvent` broadcast enum |
| `src/loro_state.rs` | `LoroSessionState` (CRDT-backed message store) |
| `src/projector.rs` | `LoroProjector` (trace event → Loro mutations) |
| `src/mutation.rs` | `MutationHelper` (trace-first, then Loro, then AgentEvent) |
| `src/runner.rs` | `SessionRunner` (10-phase loop, concurrent-run guard) |
| `src/adapters/mod.rs` | Adapter module root |
| `src/adapters/llm.rs` | Session→LLM adapter (OpenWand DTOs → LlmClient) |
| `src/adapters/policy.rs` | Session→Policy adapter (ToolCall → PolicyRequest) |
| `src/adapters/tools.rs` | Session→Tools adapter (LLM tool calls → ToolExecutor) |
| `src/testing/mod.rs` | Testing module root (behind `#[cfg(feature = "testing")]`) |
| `src/testing/harness.rs` | `SessionHarness` — deterministic test harness |
| `src/testing/mock_llm.rs` | `MockLlmClient` — preset LLM behaviors |
| `src/testing/mock_policy.rs` | `MockPolicyEngine` — AllowAll/BlockToolName/Fail |
| `src/testing/mock_tools.rs` | `MockToolExecutor` — success/error/empty responses |
| `src/testing/mock_memory.rs` | `MockMemoryReadStore` — records queries, returns empty |
| `tests/acceptance.rs` | 11 integration tests |
| `tests/dependency_guards.rs` | 2 dependency boundary guards |

---

## 2. Key Decisions

### 2.1 StoredEvent as Trace Type
Session uses `TraceStore<StoredEvent>` (not `TraceStore<OpenWandTraceEvent>`).  
`StoredEvent` is a newtype from `openwand-store` that wraps `OpenWandTraceEvent` and implements `TraceEventEnvelope`.  
This satisfies the orphan rule: store owns the impl, session just uses it.

### 2.2 Session Depends on Store
Session imports `openwand-store` for `StoredEvent` only.  
This is acceptable because store is a thin crate (no SQLite yet in Wave 01d).  
Store will gain SQLite persistence in Wave 01e.

### 2.3 Feature-Gated Testing Module
All mocks are behind `#[cfg(feature = "testing")]`.  
Integration tests run with `--features openwand-session/testing`.  
CI uses mocks exclusively — no API keys or real MCP servers needed.

### 2.4 Infallible Tool Execution
`ToolExecutor::execute` always returns `ToolResult`, never `Err`.  
Errors become `ToolResult { is_error: true, output: "error message" }`.  
Session feeds error results back to the LLM.

### 2.5 Concurrent-Run Guard
`SessionRunner::run_turn()` uses `tokio::sync::Mutex::try_lock()`.  
Second concurrent call gets `SessionError::RunAlreadyActive` immediately.

---

## 3. API Surface (Locked)

### 3.1 SessionRunner

```rust
impl SessionRunner {
    pub fn new(
        session_id: SessionId,
        trace: Arc<dyn TraceStore<StoredEvent>>,
        llm: Arc<dyn LlmClient>,
        tools: Arc<dyn ToolExecutor>,
        policy: Arc<dyn PolicyEngine>,
        memory: Arc<dyn MemoryReadStore>,
        working_dir: PathBuf,
    ) -> Self;

    pub async fn run_turn(
        &self,
        user_message: String,
        config: RunConfig,
    ) -> Result<RunSummary, SessionError>;

    pub fn loro_state(&self) -> &LoroSessionState;
    pub fn subscribe(&self) -> broadcast::Receiver<AgentEvent>;
}
```

### 3.2 SessionError

```rust
pub enum SessionError {
    Trace(TraceError),
    Llm(LlmError),
    Policy(PolicyError),
    Tool(ToolError),
    Projection(String),
    ProjectionStaleMarker(String),
    RunAlreadyActive,
    Cancelled,
}
```

### 3.3 RunSummary / RunStopReason

```rust
pub struct RunSummary {
    pub steps_completed: u32,
    pub tools_called: u32,
    pub stop_reason: RunStopReason,
    pub token_usage: TokenUsageSnapshot,
    pub trace_ids: Vec<TraceId>,
    pub recoverable: bool,
}

pub enum RunStopReason {
    Natural,
    ToolBlocked,
    StepLimitReached,
    TokenBudgetExhausted,
    Cancelled,
}
```

### 3.4 LoroSessionState

```rust
impl LoroSessionState {
    pub fn new() -> Self;
    pub fn messages(&self) -> Result<Vec<Message>, String>;
    pub fn projection_is_stale(&self) -> Result<bool, String>;
    pub fn mark_projection_stale(&self, trace_id: TraceId, reason: String) -> Result<(), String>;
}
```

### 3.5 AgentEvent (broadcast)

```rust
pub enum AgentEvent {
    PhaseEntered { session_id: SessionId, phase: String, step: u32 },
    TextDelta { session_id: SessionId, text: String },
    ToolCalled { session_id: SessionId, tool_name: String, call_id: ToolCallId },
    ToolResult { session_id: SessionId, tool_name: String, is_error: bool },
    Error { session_id: SessionId, message: String },
    Complete { session_id: SessionId, stop_reason: RunStopReason },
}
```

---

## 4. Dependency DAG

```
openwand-session depends on:
  ├── openwand-core       (events, ids, mode, risk, snapshots)
  ├── openwand-trace      (TraceStore, Actor, AppendTraceEntry, TraceId)
  ├── openwand-store      (StoredEvent — newtype for TraceEventEnvelope)
  ├── openwand-llm        (LlmClient, LlmDelta, LlmRequest, LlmTarget)
  ├── openwand-tools      (ToolExecutor, ToolCall, ToolCallContext, ToolResult)
  ├── openwand-policy     (PolicyEngine, PolicyRequest, PolicyEvaluation)
  ├── openwand-memory     (MemoryReadStore, MemoryQuery)
  ├── loro                (CRDT for session state projection)
  ├── futures             (StreamExt for LLM delta consumption)
  ├── tokio               (sync, rt-multi-thread)
  └── tokio-util          (CancellationToken)

openwand-session MUST NOT depend on:
  ├── openwand-mcp-pool   (MCP is behind tools/app wiring)
  └── rmcp                (rmcp types never escape mcp-pool)
```

---

## 5. Acceptance Tests (11)

| # | Test | Validates |
|---|------|-----------|
| 1 | `session_text_only_turn_runs` | Basic text-only turn completes with Natural stop |
| 2 | `session_text_only_loro_projection` | User + assistant messages in Loro |
| 3 | `session_tool_turn_runs` | Tool call requested and executed |
| 4 | `session_tool_turn_loro_projection` | Tool result recorded in Loro |
| 5 | `session_policy_blocked_tool` | Policy blocks tool, tool never called |
| 6 | `session_policy_failure_fail_closed` | Policy error → tool not called (fail-closed) |
| 7 | `session_tool_failure_recorded` | Tool error result in Loro with is_error=true |
| 8 | `session_loro_projection_works` | Loro not stale after successful run |
| 9 | `session_memory_read_called` | Memory search invoked during inference |
| 10 | `session_agent_events_emitted` | PhaseEntered + TextDelta + Complete events |
| 11 | `session_no_concurrent_runner` | Second concurrent call → RunAlreadyActive |

---

## 6. Mock Implementations

### 6.1 MockLlmClient
- `text_response(text)` — streams text deltas, then Done
- `tool_then_stop(call_id, name, args)` — streams one ToolCallComplete delta, then Done
- Records all `LlmRequest` values for assertion

### 6.2 MockPolicyEngine
- `AllowAll` — returns GateDecision::Allow
- `BlockToolName(name)` — blocks specific tool
- `Fail` — returns PolicyError::Internal (tests fail-closed)
- Records all `PolicyRequest` values for assertion

### 6.3 MockToolExecutor
- `empty()` — no tools available
- `with_success(name, output)` — returns successful ToolResult
- `with_error(name, error_msg)` — returns error ToolResult
- Panics on `refresh_mcp_tools()` — session must never call this
- Records all `ToolCall` values for assertion

### 6.4 MockMemoryReadStore
- Returns empty `RetrievalContext` for all queries
- Records all `MemoryQuery` values for assertion

---

## 7. Deviations from Design Document

| Design | Implementation | Reason |
|--------|---------------|--------|
| Session uses `TraceStore<OpenWandTraceEvent>` | Session uses `TraceStore<StoredEvent>` | Orphan rule: can't impl foreign trait for foreign type. StoredEvent newtype bridges this. |
| No store dependency in session | `openwand-store` added as dependency | Needed for `StoredEvent` type. Store is thin (no SQLite yet). Acceptable. |
| `RunStopReason` as string | `RunStopReason` as enum | Type safety, match exhaustiveness |
| Separate mock files per adapter | All mocks in `testing/` module | Simpler feature gating |

---

## 8. What's Next

**Wave 01e: SQLite Store Persistence**
- `openwand-store` gains SQLite backend
- `TraceStore<StoredEvent>` with WAL mode
- Blocking writer thread (`mpsc::Sender<WriterCommand>`)
- Ordered trace replay for Loro rebuild
- Content-addressed blob storage for episode text
- Verify trace is authoritative audit log

**After 01e:** Wave 01 is COMPLETE. Wave 02 begins (UI integration with Dioxus).
