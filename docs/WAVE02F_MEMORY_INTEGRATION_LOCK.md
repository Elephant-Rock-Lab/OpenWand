# WAVE 02F — MEMORY INTEGRATION HARDENING — LOCK

> **⚠️ STATUS CORRECTION — PREMATURE LOCK**
>
> This lock was premature. The stated lock condition was not met at the time
> this document was written because production binaries still used
> `StubMemoryStore` and no binary-level automated E2E proved the behavior.
>
> Unit/component tests of isolated APIs do not prove the stated condition:
> *"A user can say 'remember X,' finish the run, see X appear in the Memory
> panel automatically, then start a later run where X is retrieved into the
> prompt without manual database work."*
>
> **Current status: SUPERSEDED / INVALID LOCK.**
> **Superseded by: 02g (real wiring) and follow-up corrective wave 02h.**
>
> What was actually proven: library-level component tests pass. Wiring into
> running binaries was not done until 02g.

**Original lock date:** 2026-05-27
**Correction date:** 2026-05-27

## What was proven (library level)

- Memory projection runs via coordinator function call (not event-driven)
- Projection is idempotent by source_trace_id (UNIQUE constraint)
- Memory context can be injected into system prompt when assembled
- Runner queries memory with last user message text before inference
- Malformed extraction does not corrupt existing records
- Memory failures are non-fatal (errors captured, not propagated)

## What was NOT proven (despite claims)

- "Automatic" projection after run — nothing subscribed to RunCompleted
- Memory panel "refreshes" — no UI rendering was updated
- "Retrieved into a later session prompt" — retrieval was full-substring match only
- Production binary behavior — both binaries still had StubMemoryStore

## Architecture (as implemented)

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
  - `project_after_run()` — function exists, caller added in 02g
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
