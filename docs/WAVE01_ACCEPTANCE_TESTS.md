# WAVE 01 ACCEPTANCE TESTS

**Status:** Required before Wave 01 implementation  
**Scope:** 01a through 01e  
**Principle:** Every sub-wave must prove one contract seam independently before the next sub-wave starts.

---

## Global Test Rules

1. CI must not require API keys.
2. CI must not require network access.
3. CI must not require a real MCP server outside test fixtures.
4. Real provider tests are manual smoke tests only.
5. Every durable session-visible message/tool result must have a trace ID.
6. Trace append failure is a hard stop.
7. Policy evaluation failure blocks the tool call, not the session.
8. Tool failure is recorded as a tool result, not treated as session corruption.
9. Loro is projection only; trace remains authority.
10. SQLite persistence is required before Wave 01 is accepted.

---

## 01a — Core + Trace

### Purpose

Prove that the core event vocabulary and generic trace substrate compile, serialize, and support append/query/relation behavior with an in-memory test implementation.

### Required Tests

| Test | Description | Pass condition |
|---|---|---|
| `core_event_roundtrip_all_families` | Construct one event from each `OpenWandTraceEvent` family, serialize to JSON, deserialize, compare `event_kind()` and `schema_version()` | All 9 families round-trip |
| `core_ids_serialize_as_strings` | Create every core ID type, serialize/deserialize | Equality preserved |
| `core_no_forbidden_dependencies` | Verify `openwand-core` does not depend on trace/session/store/loro/rig/rmcp/tokio/blake3/uuid/thiserror | Dependency check passes |
| `trace_append_assigns_ids` | Append one event to `InMemoryTraceStore<OpenWandTraceEvent>` | Store assigns `TraceId`, stream sequence, global sequence |
| `trace_append_1000_entries` | Append 1000 events to one stream | Sequence is monotonic; count is 1000 |
| `trace_query_by_stream` | Query entries by `TraceStreamId` | Only matching stream returned |
| `trace_query_by_event_kind` | Query by event kind/family | Count matches inserted events |
| `trace_relations_roundtrip` | Append event B with relation to event A | Relation can be queried from B → A |
| `trace_idempotency_key` | Append same idempotency key twice | Same `TraceId` returned; no duplicate entry |
| `trace_hash_chain_valid` | Append ordered stream events | Each entry links to previous stream hash |

### Acceptance Gate

```bash
cargo test -p openwand-core
cargo test -p openwand-trace --features testing
```

01a passes only when the event vocabulary and trace substrate are stable enough for downstream crates.

---

## 01b — Policy + LLM Contracts

### Purpose

Prove that deterministic policy decisions and deterministic mock LLM streaming work without real providers.

### Required Tests

| Test | Description | Pass condition |
|---|---|---|
| `policy_read_allows_auto` | Evaluate `ToolEffect::Read` in default policy | `GateDecision::Allow` |
| `policy_search_allows_or_informs` | Evaluate `ToolEffect::Search` | Expected low-risk decision |
| `policy_unknown_blocks` | Evaluate `ToolEffect::Unknown` | `GateDecision::Block` |
| `policy_write_requires_confirmation` | Evaluate `ToolEffect::Write` | Requires confirmation |
| `policy_delete_escalates_or_blocks` | Evaluate `ToolEffect::Delete` | High/critical decision |
| `policy_fail_closed_on_error` | Force evaluator error | Produces blocked fail-closed evaluation |
| `policy_mode_floor_never_lowers_risk` | Apply Direct/Conversational/AutoRouting floors | Confirmation only stays same or increases |
| `llm_mock_stream_text` | Mock LLM emits 3 `LlmDelta::Text` and `Done` | Stream order preserved |
| `llm_mock_stream_tool_call_complete` | Mock LLM emits complete tool call | Session-facing delta has full JSON arguments |
| `llm_tool_buffer_malformed_json` | Feed malformed tool-call argument deltas | Produces decode/stream error, not partial tool call |
| `llm_error_mapping` | Simulate network/provider/decode/cancel errors | Each maps to normalized `LlmError` |
| `llm_no_rig_types_escape` | Public API inspection | No Rig types in exported OpenWand API |

### Acceptance Gate

```bash
cargo test -p openwand-policy
cargo test -p openwand-llm
```

01b passes only when policy and LLM contracts can be used by session without provider/network dependency.

---

## 01c — Tools + MCP Pool

### Purpose

Prove that local tools and MCP tools can be exposed through one `ToolExecutor` seam, with canonical names and normalized results.

### Required Tests

| Test | Description | Pass condition |
|---|---|---|
| `local_read_file_reads_temp_file` | Create temp file; call `local__read_file` | Output contains expected content |
| `local_list_directory_lists_temp_dir` | Create temp dir; call `local__list_directory` | Output contains expected entries |
| `local_search_files_respects_gitignore` | Create ignored and non-ignored files | Search excludes ignored files |
| `canonical_tool_name_local` | Generate/parse local canonical name | `local__read_file` parses correctly |
| `canonical_tool_name_mcp` | Generate/parse MCP canonical name | `mcp__server__tool` parses correctly |
| `tool_unknown_returns_error_result` | Execute missing tool | Returns `ToolResult { is_error: true }` |
| `tool_result_truncation` | Return oversized output | Output truncated with original size recorded |
| `composite_available_tools_local_only` | Registry with local tools only | Available tools contains local descriptors |
| `mcp_stdio_fixture_discovers_one_tool` | Start test stdio MCP server | One MCP tool discovered |
| `mcp_stdio_fixture_calls_tool` | Call discovered MCP fixture tool | Normalized successful result |
| `composite_available_tools_local_plus_mcp` | Local registry + MCP fixture | Both local and MCP tools returned |
| `rmcp_types_do_not_escape_pool` | Public API inspection | No rmcp types exposed above `openwand-mcp-pool` |

### MCP Test Fixture

```text
Location: crates/mcp-pool/tests/fixtures/echo-server/
Type:     Minimal Rust stdio MCP server binary
Behavior: Responds to tools/list with one "echo" tool. Echoes tools/call arguments back.
CI:       Self-contained. No Node.js dependency.
```

### Acceptance Gate

```bash
cargo test -p openwand-mcp-pool
cargo test -p openwand-tools
```

01c passes only when session can depend on `openwand-tools::ToolExecutor` without knowing whether a tool is local or MCP-backed.

---

## 01d — Session with Mocks

### Purpose

Prove the 10-phase session loop integrates trace, LLM, policy, tools, memory-read seam, Loro projection, and transient agent events with deterministic mocks.

### Required Tests

| Test | Description | Pass condition |
|---|---|---|
| `session_text_only_turn_runs` | User message → mock LLM text response | Run completes naturally |
| `session_text_only_trace_events` | Inspect trace after text-only turn | Contains session start, user message, inference call/completion, assistant message, step end |
| `session_text_only_loro_projection` | Inspect Loro messages | User and assistant messages projected with trace IDs |
| `session_tool_turn_runs` | Mock LLM proposes `local__read_file`; policy allows; tool executes | Run completes with tool result |
| `session_tool_turn_trace_events` | Inspect trace after tool turn | Contains inference, gate, tool called/completed, assistant/tool messages |
| `session_policy_blocked_tool` | Mock LLM proposes blocked tool | Tool does not execute; block recorded |
| `session_policy_failure_fail_closed` | Mock policy returns error | Tool blocked; session continues/end is controlled |
| `session_tool_failure_recorded` | Mock tool returns error result | Trace has failed tool event; loop handles result |
| `session_trace_failure_hard_stops` | Mock trace append fails | No Loro mutation after failed append |
| `session_loro_failure_marks_stale` | Force projection error | Trace remains authoritative; run continues or warns |
| `session_memory_read_called_before_inference` | Mock `MemoryReadStore` records calls | Retrieval called during context assembly |
| `session_memory_projection_not_accessible` | Type/compile boundary | Session has no `MemoryProjectionStore` dependency |
| `session_agent_events_transient` | Subscribe to event channel | Agent events emitted but not treated as authority |
| `session_no_concurrent_runner` | Start two turns on same session | Second run rejected or queued per design |

### Acceptance Gate

```bash
cargo test -p openwand-session --features testing
```

01d passes only when one text-only turn and one read-only tool turn run deterministically end-to-end with mocks.

---

## 01e — SQLite TraceStore

### Purpose

Prove Wave 01 is not in-memory-only. The authoritative trace must survive process restart and support reload/replay.

### Required Tests

| Test | Description | Pass condition |
|---|---|---|
| `sqlite_migrations_create_trace_schema` | Open empty DB with migrations | Trace tables and indexes exist |
| `sqlite_append_one_entry` | Append one trace entry | Entry can be fetched by ID |
| `sqlite_append_100_entries_reopen_replay` | Append 100 entries, close, reopen, scan | All entries match |
| `sqlite_stream_sequence_monotonic` | Append multiple streams | Per-stream and global sequences correct |
| `sqlite_relation_graph_survives_reload` | Append relations, close, reopen | Relations query correctly |
| `sqlite_idempotency_key_survives_reload` | Append with idempotency key, reopen, append same key | Existing ID returned |
| `sqlite_hash_chain_valid_after_reload` | Verify stream hash chain after reopen | Chain valid |
| `sqlite_query_by_event_kind` | Query indexed event kind | Correct entries returned |
| `sqlite_concurrent_append_serialized` | Spawn concurrent append tasks | No duplicate global or stream sequences. Tests OpenWand single-writer invariant, not SQLite locking. |
| `sqlite_session_replay_minimal_history` | Run 01d session against SQLite trace, close, reopen, scan session stream | Session-relevant history reconstructable |
| `sqlite_no_memory_projection_required` | Build 01e scope | Memory projection tables/behavior not required for pass |

### Acceptance Gate

```bash
cargo test -p openwand-store --features sqlite
cargo test -p openwand-session --features sqlite-testing
```

01e passes only when SQLite trace persistence can replace the in-memory trace implementation for the deterministic session acceptance tests.

---

## Manual Smoke Tests

Manual tests are outside CI.

| Smoke test | Purpose | Pass condition |
|---|---|---|
| Real provider text-only completion | Validate Rig adapter path | Provider returns streamed text |
| Real provider tool-call completion | Validate provider tool-call shape | Tool call normalizes into `LlmDelta::ToolCallComplete` |
| Real MCP stdio server | Validate non-fixture MCP lifecycle | Server starts, tools discovered, one read-only call succeeds |
| SQLite reload via app harness | Validate end-to-end restart path | Session-visible history reloads from trace |

---

## Final Wave 01 Gate

Wave 01 is complete only when:

```text
A deterministic session turn can run through the 10-phase loop,
record all durable events to trace,
project to Loro,
and reload from SQLite trace persistence.
```
