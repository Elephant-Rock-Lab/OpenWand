# WAVE02L COORDINATOR WIRING LOCK

**Date:** 2026-05-29
**Commits:** 5fbe45e → c2e1467 (6 commits)
**Tests:** 652 → 667, zero failures

## Lock Condition (all met)

1. ✅ MemoryCoordinator can produce 02j-filtered, 02k-assembled prompt inputs
2. ✅ UI and CLI both call produce_prompt_inputs after coordinator runs
3. ✅ Cached prompt inputs are scoped by session_id and working_directory
4. ✅ The runner uses provided 02k inputs instead of raw memory search
   - Proven by `runner_uses_memory_prompt_inputs_before_raw_search` test
   - `MockMemoryReadStore.search()` is never called when `memory_prompt_inputs` is `Some`
5. ✅ Missing repo / empty memory / production errors degrade to empty inputs
6. ✅ Supported claims appear in the prompt block with provenance
7. ✅ Unverifiable, stale, missing, and conflicting claims do not appear as trusted context
8. ✅ Workspace tests pass cleanly from 652 to 667

## What Shipped

### Commit 1 — `produce_prompt_inputs()` on MemoryCoordinator
- New `PromptInputProductionConfig` (bounded: 100 records, 5 hits/record)
- New `PromptInputResult` (with `source_session_id`, `source_working_directory`)
- New method: `produce_prompt_inputs(session_id, working_dir, config)`
- Steps: list_active_records → sort deterministically → cap → search_ranked(CurrentState) → observe_repo(StdRepoReadFs) → classify_current_claim → detect_missing_in_memory → RepoConsistencyReport → RepoConsistencyPromptAssembler::assemble_from_report
- Guard: if all search_ranked fail, returns empty (no false missing-memory findings)
- 10 new tests

### Commit 2 — Session-scoped UI cache
- `CachedMemoryPromptInputs` struct (session_id + working_directory + inputs)
- `MEMORY_PROMPT_INPUTS` global signal in ui_main.rs
- Cache cleared on session switch
- Working directory read from `CURRENT_SESSION.working_directory`

### Commit 3 — UI RunConfig wiring
- `start_run()` gains `working_directory: PathBuf` and `memory_prompt_inputs` parameters
- Working directory from `CURRENT_SESSION.working_directory` (no CWD assumptions)
- Cached inputs filtered by session_id + working_directory before consumption
- SessionRunner constructed with explicit working_dir

### Commit 4 — CLI binary wiring
- `produce_prompt_inputs()` called after coordinator runs (diagnostic output)
- Uses `std::env::current_dir()` consistently for runner and coordinator
- CLI is single-shot (no loop), no cross-turn caching needed

### Commit 5 — Tests
- 4 cache filtering tests (session isolation, workdir isolation, clearing)
- 1 runner boundary test (proves raw search not called when 02k inputs exist)

## Architecture

```text
After Turn N completes:
  MemoryCoordinator::project_after_run()
    → extract episodes, accept candidates
  MemoryCoordinator::produce_prompt_inputs(session_id, working_dir)
    → list_active_records → sort → cap → search_ranked(CurrentState)
    → observe_repo(StdRepoReadFs) → classify → detect_missing → assemble
  → CachedMemoryPromptInputs(session_id, workdir, inputs)

Before Turn N+1 starts:
  Read MEMORY_PROMPT_INPUTS
    → filter by session_id + working_directory
  → start_run(session_id, text, llm, runner, workdir, filtered_inputs)
    → RunConfig { working_directory, memory_prompt_inputs }
    → runner.run_turn(config)
      → assemble_llm_request:
        if memory_prompt_inputs.is_some() → 02k to_prompt_block()
        else → raw memory.search()
```

## Key Design Decisions

- **KD-1:** StdRepoReadFs for real filesystem reads. Graceful on non-workspace dirs.
- **KD-2:** Bounded record processing (100 records, 5 hits/record, deterministic sort).
- **KD-3:** Session-scoped cache, not raw global. Filters by session_id + workdir.
- **KD-4:** Explicit working directory, no CWD assumptions. Read from session state.

## What Does NOT Change

- `MemoryStore` trait — no new methods
- `MemoryReadStore` trait — untouched
- `repo_consistency` module — consumed as-is
- `prompt_assembly` module — consumed as-is
- Runner's raw `search()` fallback — preserved when `memory_prompt_inputs` is `None`
- `OutputGuardConfig` — unrelated
- `ProjectionResult` — existing method unchanged
- Any Dioxus component — no UI changes

## Known Gaps

- `StdRepoReadFs` does synchronous filesystem I/O inside an async method. Acceptable for desktop, not for a server.
- `MemoryQuery` built from `record.claim` text. Single-word claims produce noisy queries.
- No caching of repo observation — re-observes on every turn completion.
- The UI uses `CURRENT_SESSION.working_directory` as the working directory source. If a future multi-root workspace model is added, this source should become repo-specific rather than session-wide.
- The `all_ranked_search_failures_degrade_to_empty_inputs` test is indirect (uses InMemory store, can't force search_ranked errors). A true test would need a custom mock store.

## Test Delta

| Suite | Before | After | Delta |
|-------|--------|-------|-------|
| app (coordinator_wiring) | 0 | 14 | +14 |
| session (acceptance) | 18 | 19 | +1 |
| **Total** | **652** | **667** | **+15** |
