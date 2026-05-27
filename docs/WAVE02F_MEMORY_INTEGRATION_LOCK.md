# WAVE 02F — MEMORY INTEGRATION HARDENING — LOCK

**Status:** ✅ COMPLETE
**Date:** 2026-05-27
**Scope:** Automatic memory projection, prompt injection, UI refresh, idempotent rebuild

## Proven

- Memory projection runs after session run (coordinator subscribes to RunCompleted)
- Projection is idempotent by source_trace_id (UNIQUE constraint)
- Memory context injected into system prompt when memories match user message
- Runner queries memory with last user message text before inference
- Memory panel refreshes after projection (build_memory_panel)
- Manual rebuild is idempotent (re-project same entries → no duplicates)
- Malformed extraction does not corrupt existing records
- Memory failures are non-fatal (errors captured, not propagated)

## Architecture

```
SessionRunner.run_turn()
  → assemble_llm_request()
  → memory.search(last_user_message_text)
  → memory_context.to_context_block()
  → appended to system_prompt

MemoryCoordinator (app crate)
  → project_after_run(session_id)
  → scans trace entries for session
  → projects relevant entries as MemoryEpisodes
  → extracts candidates via MemoryExtractor
  → accepts via deterministic rules
  → errors captured in ProjectionResult

Session LoroState
  → last_user_message_text() — used as memory search query
```

## Key Changes

- Runner: memory retrieval uses last user message text (not empty string)
- Runner: memory context injected into system prompt
- LoroSessionState: new `last_user_message_text()` method
- MemoryCoordinator: new wiring in app crate
  - `project_after_run()` — automatic after runs
  - `rebuild_from_trace()` — manual rebuild
- ProjectionResult: episodes_projected, candidates_extracted, records_accepted, errors
- Session acceptance test: verifies memory.query called during run

## New Files

- `crates/app/src/memory_coordinator.rs` — MemoryCoordinator + ProjectionResult
- `crates/app/tests/memory_integration.rs` — 7 integration tests

## Modified Files

- `crates/session/src/runner.rs` — memory query with user message text, system prompt injection
- `crates/session/src/loro_state.rs` — last_user_message_text()
- `crates/session/tests/acceptance.rs` — session_memory_retrieval_called_during_run test

## Tests: 265 total (+8), 0 failures
