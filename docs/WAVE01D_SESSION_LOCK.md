# WAVE 01D — SESSION WITH MOCKS — LOCK

**Status:** ✅ COMPLETE
**Date:** 2026-05-27
**Commit:** 27

## Summary

Wave 01d is complete. `openwand-session` now consumes the `ToolExecutor` seam with deterministic mocks and no direct MCP awareness.

## Verification

| Metric | Value |
|---|---:|
| Workspace tests | 167 total |
| Previous test count | 154 |
| New session tests | 20 |
| Unit tests | 7 |
| Acceptance tests | 11 |
| Dependency guards | 2 |
| Warnings | 0 |
| Compile | Clean |

## Built

- `MockLlmClient`
- `MockPolicyEngine`
- `MockToolExecutor`
- `MockMemoryReadStore`
- `SessionHarness`
- Preset harness scenarios:
  - `text_only()`
  - `read_file_tool_turn()`
  - `tool_turn_with_policy()`
  - `tool_turn_with_tool_error()`
- Acceptance tests for:
  - text-only turn
  - tool turn
  - policy block
  - policy failure / fail-closed behavior
  - tool error recording
  - Loro projection
  - memory read before inference
  - transient agent events
  - concurrent run rejection
- Dependency guards:
  - session must not depend on `openwand-mcp-pool`
  - session must not depend on `rmcp`
  - session must depend on required Wave 01d seam crates

## Locked Boundary

```text
openwand-session → openwand-tools::ToolExecutor
openwand-session ↛ openwand-mcp-pool
openwand-session ↛ rmcp
```

## Key API Discovery

`openwand-session` uses `TraceStore<StoredEvent>`, not `TraceStore<OpenWandTraceEvent>`.

This follows the Wave 01a bridge decision:

```rust
pub struct StoredEvent(pub OpenWandTraceEvent);

impl TraceEventEnvelope for StoredEvent {
    fn event_kind(&self) -> &'static str { self.0.event_kind() }
    fn schema_version(&self) -> u16 { self.0.schema_version() }
}
```

The practical result is:

```text
core owns event vocabulary
trace owns generic trace substrate
store owns the bridge
session consumes the bridged trace store
```

## Accepted Execution Deviation

| Planned | Actual | Reason | Impact |
|---------|--------|--------|--------|
| Commits 27–33 split across mocks, tests, guards, lock | Commit 27 completed all Wave 01d scope | Real API mismatches made batching more efficient | No reduction in coverage; acceptance scope preserved |

## Acceptance Status

Wave 01d satisfies the intended 01d acceptance purpose: the session loop integrates trace, LLM, policy, tools, memory-read, Loro projection, and transient agent events with deterministic mocks. The official acceptance list includes exactly these categories: text-only turns, tool turns, policy block/failure, tool failure, trace/Loro behavior, memory-read, transient `AgentEvent`s, and concurrent runner behavior.

## Final Statement

Wave 01d is locked. Session now consumes the `ToolExecutor` seam through deterministic mocks, records behavior through the trace bridge, projects into Loro, emits transient agent events, rejects concurrent runs, and remains free of direct MCP/rmcp awareness.
