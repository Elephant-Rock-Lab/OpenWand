# Wave 20: Live Trace and Memory Inspector — Lock

**Commit:** `a65436c`
**Date:** 2026-06-03
**Status:** LOCKED
**Tests:** 1555 total, zero failures

## Scope

Display live and replayed trace events, session timeline, memory retrieval context, included/excluded memory evidence, tool/gate history, and trace-linked message state in the Dioxus UI. The inspector is read-only, rebuildable from existing trace/memory/session projections, and cannot mutate session, memory, tools, policy, git, or persistence authority.

## Non-Negotiable Invariant

```
Trace is authority.
Memory is derived.
Inspector is projection over projections.
The inspector observes.
The inspector explains.
The inspector never mutates.
```

## Module Boundary

```
Wave 19: UI live session productization
Wave 20: read-only trace and memory inspector
```

New files:
- `inspector_state.rs` — all DTOs, loader, bridge, memory inspector builder
- `inspector_components.rs` — view helpers + Dioxus render functions

Existing files unchanged (except `mod.rs` registration).

## Patches Applied

### Patch 1: All Known Families Recognized

All 9 current `OpenWandTraceEvent` families produce timeline items:
- **Session** — rich summaries (started, ended, step, user msg, assistant msg)
- **Inference** — called/completed/failed with model name
- **Gate** — evaluated/batch_completed/output_screened with pass/fail/risk
- **Tool** — full lifecycle: called/suspended/resumed/denied/completed/failed/deferred
- **Memory** — fact extraction/acceptance/rejection + generic for others
- **File/Mode/Workflow/Artifact** — generic family summaries

Test: `inspector_recognizes_all_current_trace_event_families` (9 families = 9 timeline items).

### Patch 2: Relation Direction

`load_event_detail()` loads both outgoing and incoming trace relations:
- Outgoing: `from_trace_id == selected`
- Incoming: `to_trace_id == selected`

`TraceRelationSummary` carries `RelationDirection` (Outgoing/Incoming).
View helper `trace_relation_rows` renders "→ outgoing" / "← incoming".

### Patch 3: Live Bridge

`apply_trace_entry_to_inspector()` is used by both:
1. Batch loader from persisted trace (`load_inspector_from_trace`)
2. Live UI bridge during a run (same function, single entry at a time)

Tests prove live event → inspector state update for trace timeline, gate/tool history, and memory events.

### Patch 4: Read-Only Proof

Source guards block:
- `std::process::Command`
- `ToolExecutor` / `PolicyEngine`
- `MemoryProjectionStore`
- Git backends / governed execution backends
- `.append()` / `TraceRelationDraft` / `append_and_project`
- `save_proposal` / `save_execution` / `save_verification`

## Inspector State Model

`LiveInspectorState` is a pure read-only projection:
- `trace_timeline` — all events in trace order, with family/kind/actor/summary
- `gate_tool_events` — gate decisions and tool lifecycle events
- `memory_context` — counts from memory panel data
- `memory_evidence` — per-claim evidence items
- `selected_event` — detail drawer with payload and relations
- `warnings` — for unknown families or missing data

State is fully serializable (serde) and rebuildable from persistence.

## Memory Inspector

Reuses existing `UiFilteredMemoryPanel` data. No raw store queries.
`load_memory_inspector()` converts panel rows → evidence items:
- Prompt-included → `Included`
- Stale → `ExcludedStale`
- Superseded → `ExcludedSuperseded`
- Unverifiable → `ExcludedUnverifiable`
- Conflicts → `ExcludedConflict`
- Missing in memory → `Missing`

Each evidence item carries `source_trace_ids` for trace-backed linkage.

## What Did Not Ship

```
memory editing
memory acceptance/rejection
trace append
trace repair
session mutation
tool approval/rejection
tool execution
policy editing
git execution
governance record mutation
provider integration
workflow spawning
multi-session orchestration
skills/goals activation
```

## Test Coverage (47 new tests)

- **State/loader/bridge** (14): empty, trace timeline, gate events, tool lifecycle, inference, memory, mode, ordering, no mutation, payload summary, rebuildable, serde roundtrip
- **Patch 1** (2): all 9 families, generic summaries for minimal families
- **Patch 2** (3): outgoing relations, incoming relations, direction distinguish
- **Patch 3** (3): live trace event, live gate event, live memory event
- **View helpers** (10): timeline rows, gate/tool rows, evidence rows, relation rows, detail lines, warnings, all kinds displayable, all statuses displayable, empty inspector
- **Memory inspector** (8): counts, stale/superseded/unverifiable, conflicts, included claims, trace links, empty panel, full panel, missing panel
- **Guards** (10): process, tool executor, policy, memory store, git backends, execution backends, shell, trace append + relations, governed records
- **Runtime** (2): state unchanged, clone independence

## Honest Caveats

- Wave 20 is observability. It does not add mutation capabilities of any kind.
- File/Workflow/Artifact events produce generic summaries. Rich detail can be added when those event families have more payload data in practice.
- Trace relations are loaded on demand for the detail drawer, not pre-loaded for the full timeline.
- The inspector complements but does not replace the governance console (Wave 18) or session transcript (Wave 19).
- Memory inspector reads the same governed panel data the memory panel shows. If the panel is empty (no coordinator run), the inspector shows empty.
