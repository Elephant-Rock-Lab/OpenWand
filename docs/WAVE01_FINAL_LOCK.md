# WAVE 01 — FINAL LOCK

**Status:** ✅ COMPLETE — ALL SUB-WAVES LOCKED
**Date:** 2026-05-27
**Commits:** 8 (Wave 01a) + 7 (01b) + 7 (01c) + 1 (01d) + 4 (01e) = **27 commits** (Wave 00 excluded)

---

## Sub-Wave Lock Status

| Sub-wave | Scope | Commits | Tests Added | Lock Document |
|----------|-------|---------|-------------|---------------|
| 01a | Core + Trace | 1–8 | 47 | `WAVE01A_CORE_TRACE_LOCK.md` |
| 01b | Policy + LLM | 9–15 | 114 cumulative | `WAVE01B_POLICY_LOCK.md` |
| 01c | Tools + MCP Pool | 16–22 | 149 cumulative | `WAVE01C_LOCK.md` |
| 01d | Session + Mocks | 27 | 167 cumulative | `WAVE01D_SESSION_LOCK.md` |
| 01e | SQLite TraceStore | 28–31 | **187** cumulative | `WAVE01E_SQLITE_TRACESTORE_LOCK.md` |

---

## Final Metrics

| Metric | Value |
|--------|------:|
| Total tests | **187** |
| Total crates with real code | **9** |
| Total crates (including scaffolds) | **13** |
| Warnings | **0** |
| Failed tests | **0** |
| Persistence backend | SQLite TraceStore |
| Hash chain | BLAKE3 (64 hex chars) |

### Per-Crate Test Counts

| Crate | Unit | Integration | Guards | Total |
|-------|-----:|------------:|-------:|------:|
| openwand-core | 15 | — | 2 | 17 |
| openwand-trace | 21 | — | — | 21 |
| openwand-store | 13 | 20 | 2 | 35 |
| openwand-policy | 25 | 14 | 1 | 40 |
| openwand-llm | 23 | — | 2 | 25 |
| openwand-mcp-pool | 3 | 2 | — | 5 |
| openwand-tools | — | 26 | — | 26 |
| openwand-memory | 1 | — | — | 1 |
| openwand-session | 7 | 14 | 2 | 23 |
| **Total** | | | | **187** |

---

## Accepted Deviations (All Sub-Waves)

| Sub-wave | Planned | Actual | Reason |
|----------|---------|--------|--------|
| 01a | Single commit | 8 commits | API convergence against real types |
| 01c | 5 commits | 7 commits | rmcp API mismatches resolved |
| 01d | 8 commits (27–33) | 1 commit (27) | Real API mismatches made batching more efficient |
| 01e | 9 commits (28–36) | 4 commits (28–31) | Same lesson: batch harder |
| — | `TraceStore<OpenWandTraceEvent>` | `TraceStore<StoredEvent>` | Orphan rule: store owns the bridge |
| — | Separate mock files per adapter | All mocks in `testing/` module | Simpler feature gating |
| — | `openwand-store` depends on `openwand-memory` | No memory dependency in 01e | Memory projection deferred to Wave 02 |

---

## Final Crate Dependency DAG

```text
openwand-core (serde, serde_json, chrono, ulid ONLY)
  ├── openwand-trace (async-trait, chrono, serde, blake3, thiserror, tokio)
  ├── openwand-store (core + trace + rusqlite + blake3 + tokio)
  ├── openwand-policy (core + async-trait, serde_json, thiserror)
  ├── openwand-llm (core + async-trait, futures, chrono, thiserror)
  ├── openwand-mcp-pool (core + rmcp=1.7.0 + async-trait, tokio)
  ├── openwand-tools (core + mcp-pool + async-trait, tokio, walkdir, ignore)
  ├── openwand-memory (core + async-trait, serde_json, thiserror)
  └── openwand-session (core + trace + store + llm + tools + policy + memory + loro + tokio-util)
```

### Forbidden Edges (Enforced by Guard Tests)

```text
session ↛ mcp-pool, rmcp
tools   ↛ rmcp
store   ↛ session, policy, tools, llm, memory, loro, rig, rmcp
```

---

## Durable Authority Invariant

This is the single most important design property validated by Wave 01:

```text
Trace is authority.
Loro is projection.
Memory is derived.
Session coordinates.
Store persists.
```

Concretely:

1. **No important state mutates without appending a trace entry first.**
   Trace failure is the only hard stop.

2. **Loro is a rebuildable projection, not authority.**
   Loro failure = degraded UI. Session continues.

3. **Memory is derived from trace events.**
   Memory failure = continue without context. Session continues.

4. **Session coordinates; it does not own truth.**
   SessionRunner is the only writer, but truth lives in the trace log.

5. **Store persists; it does not define.**
   Store implements traits from trace and memory. Owns no domain vocabulary.

6. **Policy gates; it does not execute.**
   Policy failure = fail closed (block tool). Session continues.

7. **Tools execute; they do not decide.**
   Tool failure = feed back to LLM. Session continues.

---

## Wave 02 Entry Criteria

Wave 02 may begin when ALL of the following are true:

```text
[x] 187 tests passing
[x] Zero warnings
[x] SQLite TraceStore persists across process restart
[x] Session loop runs deterministically with mocks
[x] Loro projection updates from trace events
[x] Policy gates tool calls (allow/block/fail-closed)
[x] Hash chain is valid after reload
[x] Concurrent append is serialized (single-writer invariant)
[x] All dependency guard tests pass
[x] All five sub-wave lock documents exist
```

Wave 02 scope (to be defined in its own design pack):

```text
- Real LLM provider integration (via Rig adapter)
- MCP server stdio transport (real, not mock)
- Memory extraction pipeline (trace → claims)
- Dioxus UI skeleton (session view + trace viewer)
- App wiring (connect all crates into running binary)
```

---

## Wave 01 Commits

```text
Wave 00 (cross-document audit):          pre-wave
Wave 01a (Core + Trace):                 commits 1–8
Wave 01b (Policy + LLM):                 commits 9–15
Wave 01c (Tools + MCP Pool):             commits 16–22
Wave 01d (Session + Mocks):              commit 27
Wave 01e (SQLite TraceStore):            commits 28–31
```

---

## Final Statement

Wave 01 is locked end-to-end. The architectural spine is durable, testable, and reloadable. The project has crossed the first real architecture threshold: a deterministic agent loop that persists its audit trail to SQLite, projects state into CRDTs, gates tool execution through policy, and survives process restart.

What was designed on paper is now running in code.
