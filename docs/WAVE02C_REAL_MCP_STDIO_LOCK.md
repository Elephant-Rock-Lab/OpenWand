# WAVE 02C — REAL MCP STDIO — LOCK

**Status:** ✅ COMPLETE
**Date:** 2026-05-27
**Scope:** Real stdio MCP server lifecycle, discovery, execution, composite integration

## Proven

- Real stdio MCP server starts and stops cleanly
- Tool discovery via `tools/list` returns discovered tools
- Tool execution via `tools/call` returns real results
- Composite executor lists both local and MCP tools
- Composite executor dispatches canonical `mcp__{server}__{tool}` names
- MCP tool results flow back through ToolExecutor → SessionRunner → UI
- No API keys, no network, no external dependencies required for CI

## Architecture

```
McpServerConfig (loaded from config)
  → McpServerPool::new(configs)
  → ensure_started() spawns child process + rmcp handshake
  → discover_all_tools() calls tools/list on each server
  → execute_tool() calls tools/call with arguments
  → CompositeToolExecutor unifies local + MCP
  → canonical names: mcp__{server}__{remote_tool}
```

## Echo Server Fixture

`crates/mcp-pool/tests/fixtures/echo-server/` — minimal Rust stdio MCP server:
- `echo_read`: returns "echo: {text}" (Read effect)
- `echo_list`: returns fixed list of items (Read effect)
- Built as `openwand-echo-mcp-server` binary
- CI deterministic: no Node, no network, no API keys

## New Files

- `crates/mcp-pool/tests/fixtures/echo-server/` — echo MCP server fixture
- `crates/mcp-pool/tests/mcp_stdio_integration.rs` — 7 integration tests

## Tests: 235 total (+7), 0 failures

- mcp_server_start_stop_cleanly
- mcp_discover_tools_real_stdio_fixture
- mcp_call_tool_real_stdio_fixture
- mcp_call_list_tool
- mcp_call_nonexistent_tool_returns_error
- composite_executor_lists_local_plus_real_mcp
- composite_executor_executes_mcp_canonical_name
