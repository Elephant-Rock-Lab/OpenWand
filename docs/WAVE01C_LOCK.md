# WAVE01C_LOCK — Tools + MCP Pool

**Status:** ✅ Locked  
**Date:** 2026-05-27  
**Commits:** 16–22  
**Workspace verification:** `cargo test --workspace` → 149 tests, 0 failures, 0 warnings  
**Scope:** Local read/search/list tools + MCP stdio seam + unified `ToolExecutor`

---

## 1. Summary

Wave 01c locks the Tools + MCP Pool seam.

The important outcome is not merely that MCP tool calls work. The important outcome is that MCP became one backend behind the same OpenWand tool abstraction that local tools use.

```text
openwand-session
  → openwand-tools::ToolExecutor

openwand-tools
  → local tools
  → openwand-mcp-pool::McpToolGateway

openwand-mcp-pool
  → rmcp
```

Session sees one seam:

```rust
openwand_tools::ToolExecutor
```

Session does not know whether a tool is local or MCP-backed.

---

## 2. Completed Commits

| Commit | Scope                                                          | Status     |
| -----: | -------------------------------------------------------------- | ---------- |
|     16 | `mcp-pool` DTOs + `McpToolGateway` trait                       | ✅ Accepted |
|     17 | `tools` DTOs + `ToolExecutor` trait + naming/effect resolution | ✅ Accepted |
|     18 | Local tools: `file_read`, `file_list`, `file_search`           | ✅ Accepted |
|     19 | Local registry + composite executor local path                 | ✅ Accepted |
|     20 | MCP stdio implementation + mock gateway/test support           | ✅ Accepted |
|     21 | Composite local + MCP integration tests                        | ✅ Accepted |
|     22 | Dependency guards + `WAVE01C_LOCK.md`                          | ✅ Accepted |

---

## 3. What Was Built

### 3.1 `openwand-mcp-pool`

`openwand-mcp-pool` owns MCP protocol integration, MCP server lifecycle, MCP DTOs, and the gateway trait consumed by `openwand-tools`.

Built modules:

| File                  | Purpose                                           |
| --------------------- | ------------------------------------------------- |
| `config.rs`           | `McpServerConfig`, `McpTransportConfig`           |
| `discovered.rs`       | `McpDiscoveredTool`, `McpToolAnnotations`         |
| `result.rs`           | `McpToolResult`                                   |
| `state.rs`            | `McpServerState`                                  |
| `error.rs`            | `McpPoolError`                                    |
| `gateway.rs`          | `McpToolGateway` trait                            |
| `pool.rs`             | `McpServerPool`, rmcp-backed stdio implementation |
| `testing.rs`          | `MockMcpGateway`, `MockMcpServer`, `MockTool`     |
| `tests/public_api.rs` | Public API and dependency guards                  |

Core public seam:

```rust
#[async_trait::async_trait]
pub trait McpToolGateway: Send + Sync {
    async fn ensure_started(
        &self,
        server_name: &str,
    ) -> Result<(), McpPoolError>;

    async fn discover_all_tools(
        &self,
    ) -> Result<Vec<McpDiscoveredTool>, McpPoolError>;

    async fn execute_tool(
        &self,
        server_name: &str,
        remote_name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult, McpPoolError>;
}
```

MCP result shape:

```rust
pub struct McpToolResult {
    pub output: String,
    pub is_error: bool,
}
```

Lifecycle state:

```rust
pub enum McpServerState {
    NotStarted,
    Starting,
    Ready,
    Failed,
    Stopped,
}
```

Error shape:

```rust
pub enum McpPoolError {
    ServerNotFound,
    ServerDisabled,
    StartFailed,
    DiscoveryFailed,
    CallFailed,
    Transport,
    Protocol,
}
```

---

### 3.2 `openwand-tools`

`openwand-tools` owns the session-facing tool abstraction, local tool implementation, canonical naming, result normalization, and composite dispatch.

Built modules:

| File                   | Purpose                                                 |
| ---------------------- | ------------------------------------------------------- |
| `naming.rs`            | Canonical names and parsing                             |
| `effect.rs`            | MCP effect resolution                                   |
| `descriptor.rs`        | `ToolDef`, `ToolSource`, `ToolAnnotations`              |
| `normalize.rs`         | Output truncation and metadata                          |
| `result.rs`            | `ToolResult`, `ToolCallContext`                         |
| `executor.rs`          | `ToolExecutor`, `ToolCall`, `ToolRefreshReport`         |
| `local.rs`             | `BuiltinToolProvider`, `LocalTool`, Batch 1 local tools |
| `composite.rs`         | Unified local + MCP executor                            |
| `tests/integration.rs` | Composite discovery/call/listing/source verification    |

Core public seam:

```rust
#[async_trait::async_trait]
pub trait ToolExecutor: Send + Sync {
    fn available_tools(&self) -> Vec<ToolDef>;

    fn get_descriptor(&self, name: &str) -> Option<ToolDef>;

    async fn execute(
        &self,
        call: &ToolCall,
        context: &ToolCallContext,
    ) -> ToolResult;

    async fn refresh_mcp_tools(
        &self,
    ) -> Result<ToolRefreshReport, ToolError>;
}
```

Execution is intentionally infallible at the trait boundary:

```rust
async fn execute(...) -> ToolResult
```

Tool failures are represented as:

```rust
ToolResult {
    is_error: true,
    ...
}
```

not as `Result::Err`.

---

## 4. Locked Design Decisions

### 4.1 `rmcp` types never escape `openwand-mcp-pool`

`rmcp` is a protocol implementation detail.

Allowed:

```text
openwand-mcp-pool → rmcp
```

Forbidden:

```text
openwand-tools → rmcp
openwand-session → rmcp
openwand-policy → rmcp
```

Public API guard tests enforce this.

---

### 4.2 Session sees one tool seam

Session consumes:

```rust
openwand_tools::ToolExecutor
```

Session must not depend directly on:

```rust
openwand_mcp_pool
rmcp
```

App wiring is responsible for composing:

```text
McpServerPool → CompositeToolExecutor → Session
```

---

### 4.3 Canonical tool names are locked

Local tools:

```text
local__{tool}
```

MCP tools:

```text
mcp__{server}__{tool}
```

Locked Batch 1 local names:

| Tool        | Canonical Name       | Effect   |
| ----------- | -------------------- | -------- |
| File read   | `local__file_read`   | `Read`   |
| File list   | `local__file_list`   | `Read`   |
| File search | `local__file_search` | `Search` |

Example MCP name:

```text
mcp__echo__echo
```

---

### 4.4 MCP annotations are hints, not authority

Effect resolution precedence is locked:

```text
1. Per-tool config override
2. Server default effect
3. MCP annotations
4. Unknown
```

Annotations can inform the default, but they do not override OpenWand configuration.

---

### 4.5 `ToolExecutor::execute` is infallible

Tool execution returns `ToolResult` in all cases.

This includes:

* unknown tool
* invalid tool name
* local tool error
* MCP server missing
* MCP call failure
* MCP protocol error
* cancellation-derived execution failure

The session loop should record failed tool execution as a tool result, not treat it as session corruption.

---

### 4.6 Batch 1 MCP runtime is stdio only

Batch 1 supports MCP stdio runtime execution only.

HTTP is not implemented as a runtime transport in Wave 01c.

---

### 4.7 Mock MCP gateway is canonical for CI

CI must not require:

* network
* external MCP servers
* API keys
* Node.js MCP fixtures

Wave 01c uses:

```rust
MockMcpGateway
MockMcpServer
MockTool
```

for deterministic CI coverage.

---

### 4.8 Output normalization is locked

Tool output is normalized at the tool layer.

Limit:

```text
50,000 characters
```

Truncation preserves metadata:

```rust
ToolResult {
    truncated: true,
    original_size: Some(...),
    ...
}
```

---

### 4.9 Composite dispatch is locked

`CompositeToolExecutor` owns dispatch between:

```text
local tools
MCP tools
```

Session does not branch on tool source.

---

## 5. Accepted Deviations

| Deviation                                                                                                                                   | Reason                                                                           | Impact                                                                                                                                       |
| ------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| Local tool canonical names use `file_read`, `file_list`, `file_search` instead of the earlier `read_file`, `list_directory`, `search_files` | More uniform file-tool namespace                                                 | No seam impact. Canonical names are now locked as `local__file_read`, `local__file_list`, `local__file_search`.                              |
| `McpTransportConfig::StreamableHttp` exists in DTO shape even though runtime HTTP is deferred                                               | Allows config/type shape to exist behind feature-gated future transport planning | No runtime HTTP support in Wave 01c. `StreamableHttp` is DTO/feature-gated scaffold only. Runtime HTTP MCP transport is deferred to Wave 03. |

---

## 6. Dependency DAG

Wave 01c adds this dependency shape:

```text
openwand-core
├── openwand-mcp-pool
│     ├── rmcp = 1.7.0
│     └── stdio transport
│
└── openwand-tools
      ├── openwand-core
      └── openwand-mcp-pool
```

Detailed boundary:

```text
openwand-core
  Owns:
    ToolEffect
    ToolCallId
    SessionId
    shared IDs/vocabulary

openwand-mcp-pool
  Owns:
    McpToolGateway
    McpServerPool
    McpServerConfig
    McpTransportConfig
    McpDiscoveredTool
    McpToolAnnotations
    McpToolResult
    McpPoolError
    MCP lifecycle/protocol handling

openwand-tools
  Owns:
    ToolExecutor
    ToolDef
    ToolSource
    ToolAnnotations
    ToolResult
    ToolCall
    ToolCallContext
    ToolRefreshReport
    BuiltinToolProvider
    LocalTool
    CompositeToolExecutor
    canonical naming
    effect resolution
    output normalization
```

Forbidden dependencies:

```text
openwand-mcp-pool must not depend on:
  openwand-tools
  openwand-session
  openwand-policy
  openwand-llm
  openwand-memory
  openwand-store
  openwand-trace
  loro
  rig-core

openwand-tools must not depend on:
  openwand-session
  openwand-policy
  openwand-llm
  openwand-memory
  openwand-store
  openwand-trace
  loro
  rig-core
  rmcp
```

---

## 7. Test Matrix

### 7.1 Listed Test Categories

| Test Category       |  Count | Purpose                                                                   |
| ------------------- | -----: | ------------------------------------------------------------------------- |
| DTO roundtrip       |      4 | Config, discovered tool, naming, truncation                               |
| Effect resolution   |      5 | Precedence chain, local source, annotations                               |
| Naming              |      4 | Canonical format, parsing, edge cases                                     |
| Local tools         |      6 | `file_read` success/fail, `file_list`, `file_search`, Batch 1 provider    |
| Composite executor  |      6 | Local dispatch, unknown tool, invalid name, MCP miss, listing, descriptor |
| Integration         |      4 | MCP discovery, MCP call, composite listing, source verification           |
| Public API guards   |      2 | No `rmcp` leak, no forbidden dependencies                                 |
| Trait objects       |      2 | `McpToolGateway`, `ToolExecutor` object safety                            |
| **Listed here**     | **33** | Lock-document acceptance accounting                                       |
| **Workspace delta** | **35** | Full Wave 01c workspace test increase                                     |

---

### 7.2 Representative Tests

#### DTO and API tests

```text
mcp_server_config_roundtrips
mcp_discovered_tool_roundtrips
tool_name_roundtrips
tool_result_truncation_preserves_metadata
```

#### Naming tests

```text
canonical_local_tool_name_is_stable
canonical_mcp_tool_name_is_stable
parse_local_tool_name
parse_mcp_tool_name
reject_invalid_tool_names
```

#### Effect resolution tests

```text
mcp_effect_tool_config_override_wins
mcp_effect_server_default_wins
mcp_effect_annotations_used_as_hints
mcp_effect_unknown_without_signal
local_tool_effect_is_declared_by_tool
```

#### Local tool tests

```text
file_read_reads_existing_file
file_read_missing_path_returns_error_result
file_list_lists_directory
file_list_respects_limit
file_search_finds_text
builtin_provider_exposes_batch1_tools
```

#### Composite executor tests

```text
composite_lists_local_tools
composite_executes_local_tool
composite_unknown_tool_returns_error_result
composite_invalid_name_returns_error_result
composite_lists_mcp_tools_after_refresh
composite_returns_descriptor_by_name
```

#### Integration tests

```text
mock_mcp_gateway_discovers_tool
mock_mcp_gateway_calls_tool
composite_lists_local_plus_mcp
composite_descriptor_preserves_mcp_source
```

#### Public API guard tests

```text
rmcp_types_do_not_escape_mcp_pool_public_api
tools_crate_has_no_rmcp_dependency
```

#### Object safety tests

```text
mcp_tool_gateway_is_object_safe
tool_executor_is_object_safe
```

---

## 8. Verification

Workspace verification:

```bash
cargo test --workspace
```

Result:

```text
149 tests, 0 failures, 0 warnings
```

Workspace check:

```bash
cargo check --workspace
```

Result:

```text
compiles clean
```

Required per-crate verification:

```bash
cargo test -p openwand-mcp-pool
cargo test -p openwand-tools
```

---

## 9. Public API Guard Rules

The following must remain true:

```text
No rmcp type appears in openwand-tools public API.
No rmcp type appears in openwand-session public API.
No ToolDef appears in openwand-mcp-pool public API.
No ToolExecutor appears in openwand-mcp-pool public API.
No openwand-session dependency appears in openwand-tools.
No openwand-session dependency appears in openwand-mcp-pool.
No openwand-policy dependency appears in openwand-tools.
No openwand-policy dependency appears in openwand-mcp-pool.
```

Reason:

```text
Policy evaluates OpenWand descriptors.
Session executes OpenWand tools.
MCP remains a backend protocol.
```

---

## 10. Runtime Scope

### Implemented in Wave 01c

```text
Local file read
Local file list
Local file search
MCP stdio gateway
MCP discovery
MCP call
Composite local + MCP listing
Composite local + MCP execution
Mock MCP gateway for CI
Tool output truncation
Canonical tool naming
Effect resolution
Dependency/API guards
```

### Not Implemented in Wave 01c

```text
HTTP MCP runtime
MCP server config persistence
MCP reconnect/backoff policy
MCP tool-list-changed notification refresh
write_file
shell execution
git tools
code edit tools
browser/network local tools
policy argument-risk modifiers
session loop integration
trace recording
Loro projection
memory ingestion
```

---

## 11. Deferred Scope

| Deferred Item                           | Target                                    |
| --------------------------------------- | ----------------------------------------- |
| HTTP MCP runtime transport              | Wave 03                                   |
| MCP dynamic refresh from notifications  | Later MCP hardening wave                  |
| MCP reconnect/backoff lifecycle         | Later MCP hardening wave                  |
| Write-capable local tools               | After policy/session gate integration     |
| Shell/git execution                     | After explicit policy + confirmation path |
| Session loop use of `ToolExecutor`      | Wave 01d                                  |
| Trace-backed tool result recording      | Wave 01d / 01e depending store readiness  |
| SQLite persistence of trace/tool events | Wave 01e                                  |

---

## 12. Handoff to Wave 01d

Wave 01d may now consume:

```rust
Arc<dyn ToolExecutor>
```

with deterministic tests.

Wave 01d should not depend on:

```rust
openwand_mcp_pool
rmcp
```

Recommended Wave 01d test posture:

```text
1. Use MockToolExecutor or CompositeToolExecutor local-only.
2. Prove session can execute one read-only tool call.
3. Prove tool failure becomes a recorded failed tool result.
4. Prove session does not branch on local vs MCP.
5. Prove trace append happens before durable session projection.
```

The safe next seam:

```text
LLM proposes tool call
→ policy evaluates
→ session calls ToolExecutor
→ ToolResult returned
→ trace records result
→ session projects result
```

---

## 13. Final Lock Statement

Wave 01c is locked.

The Tools + MCP seam is now stable enough for session integration because:

1. Session has one executor trait.
2. Local and MCP tools share one descriptor/result model.
3. MCP protocol details are contained inside `openwand-mcp-pool`.
4. `rmcp` does not leak above the MCP pool crate.
5. Tool execution is infallible at the session boundary.
6. CI is deterministic and does not require external MCP servers.
7. Canonical naming is stable.
8. Output normalization is stable.
9. Dependency guards protect the crate DAG.
10. Workspace verification passes cleanly.
