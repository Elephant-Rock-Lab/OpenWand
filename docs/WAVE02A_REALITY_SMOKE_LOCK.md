# WAVE 02A — REALITY SMOKE — LOCK

**Status:** ✅ COMPLETE  
**Date:** 2026-05-27  
**Scope:** Real provider I/O + app wiring + real tool-call loop  
**Tests:** 197 total, 0 failures  

## Gate Satisfied

A real model can run through the OpenWand spine:

```
user message
→ real LLM inference (streamed SSE)
→ streamed tool call parsed from SSE deltas
→ tool-call buffer flushed on finish_reason: "tool_calls"
→ policy evaluation (Read/Search allowed, Write/Delete blocked)
→ local tool execution (file_list, file_read, file_search)
→ tool result returned to model via conversation history
→ final assistant answer summarizing tool results
→ SQLite trace persisted
→ Loro projection fresh
```

## Models Verified

| Model | Text-only | Tool Call | Notes |
|-------|-----------|-----------|-------|
| Qwen3 4B (qwen/qwen3-4b-2507) | ✅ | ✅ | Primary smoke model. Consistent structured tool calls via LM Studio streaming. |
| Qwen2.5 14B Instruct | ✅ | ⚠️ | Tool calls emitted as text in streaming mode (LM Studio parsing gap). Non-streaming works. |
| Qwen2.5 Coder 7B | ✅ | ❌ | Model declines to call tools. Text-only works. |
| Gemma 4 e4B | ✅ | ⚠️ | Sometimes emits structured tool calls, sometimes text. Non-deterministic. |

**Conclusion:** Model capability varies. Qwen3 4B is the reliable smoke-test model. The adapter correctly handles all cases — no crashes, no hangs, graceful degradation.

## Acceptance Criteria

- [x] Real text-only model turn (3 models)
- [x] Real tool execution turn (Qwen3 4B → `local__file_list`)
- [x] Tool result fed back into model for final answer
- [x] SQLite trace persisted and reload path covered
- [x] Smoke policy allows only Read/Search and blocks Write/Delete/Unknown

## Regression Coverage Added

- [x] App wiring exposes Batch 1 local tools (`app/tests/smoke_wiring.rs`)
- [x] Smoke policy permits Read/Search only
- [x] Smoke policy blocks Write/Delete/Unknown
- [x] SSE `finish_reason: tool_calls` flushes buffered tool calls (`llm/tests/sse_buffer_flush.rs`)
- [x] Buffered tool calls emit `ToolCallComplete` before `Done`

## Bugs Found and Fixed

| Bug | Severity | Root Cause | Fix |
|-----|----------|------------|-----|
| Empty tools array in request | Critical | `BuiltinToolProvider::new()` creates empty registry | Use `batch1_local_tools()` which registers file_read, file_list, file_search |
| Tool calls buffered but never flushed | Critical | SSE parser buffered Start/ArgsDelta but `complete()` never called | `drain_ids()` + flush all buffered calls on `finish_reason: "tool_calls"` |
| Empty policy blocks everything | Design-correct | Fail-closed: no matching rule = block | Smoke profile with Read+Search-only rules |
| Conversational mode floors Auto→Inform | Design-correct | Runner hardcoded `Conversational` | Runner uses `config.mode`; smoke sets `Direct` |
| No `tool_choice` or system prompt | Enhancement | Models need explicit tool-use affordances | `tool_choice: "auto"` + default system prompt when tools available |
| No HTTP timeout | Enhancement | Real I/O needs guardrails | 120s timeout on reqwest client |

## Key Design Decisions

- **Buffer mutex:** Changed from `tokio::sync::Mutex` to `std::sync::Mutex` in SSE parser (sync scan closure)
- **Smoke policy profile:** Read + Search effects only. Write, Delete, Unknown blocked. Preserves trust model.
- **InteractionMode in RunConfig:** Runner reads mode from config instead of hardcoding. Smoke uses `Direct`.
- **tool_choice invariant:** `tools.is_empty() → None`; `tools.non_empty() → Auto`
- **HTTP timeout:** 120s default, to be made configurable via `OPENWAND_LLM_TIMEOUT_SECS`

## Files Changed

- `crates/llm/src/adapters/openai_compatible.rs` — SSE buffer flush on finish_reason: "tool_calls", std::sync::Mutex, HTTP timeout
- `crates/llm/src/tool_buffer.rs` — Added `drain_ids()` method
- `crates/llm/tests/sse_buffer_flush.rs` — NEW: 4 buffer flush fixture tests
- `crates/session/src/runner.rs` — Uses `config.mode` for policy, sends `tool_choice: auto`, default system prompt
- `crates/app/src/main.rs` — batch1_local_tools(), Read+Search policy, Direct mode
- `crates/app/tests/smoke_wiring.rs` — NEW: 6 wiring + policy regression tests

## Test Count

```
Wave 01 final:  187 tests
Wave 02a added: +10 tests (6 wiring + 4 buffer flush)
Current total:  197 tests, 0 failures, 0 warnings
```
