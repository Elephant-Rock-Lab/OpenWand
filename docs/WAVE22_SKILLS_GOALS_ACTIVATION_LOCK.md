# Wave 22 — Skills and Goals Activation Foundation — LOCK

**Committed:** Wave 22 batch commit
**Baseline:** 1613 tests (Wave 21 locked)
**Final:** 1673 tests (+60), zero failures

---

## What Shipped

### Skills Crate (`openwand-skills`)
- **`SkillManifest`** — TOML structure with `[[skill]]` entries from `.openwand/skills.toml`
- **`SkillDefinition`** — id, name, description, category, enabled, tags, inputs, outputs, constraints, allowed_context
- **`SkillContextKind`** — enum (TraceSummary, MemorySummary, FileDiffSummary, TestOutputSummary, GovernanceSummary, UserInstruction), `serde rename_all = "snake_case"`
- **`SkillRegistry`** — validated collection, deterministic ID-ordered
- **`SkillContextSummary`** — read-only projection, enabled skills only, no executable fields
- **Validation**: non-empty ID/name/description, unique IDs, unknown context kinds → error, enabled-with-no-outputs → warning
- **Missing file** → empty registry + warning, not error

### Goals Crate (`openwand-goals`)
- **`GoalManifest`** — TOML structure with `[[goal]]` entries from `.openwand/goals.toml`
- **`GoalDefinition`** — id, title, description, status, priority, tags, success_criteria, constraints, linked_skills
- **`GoalStatus`** — enum (Active, Paused, Completed, Archived), `serde rename_all = "snake_case"`
- **`GoalRegistry`** — validated collection, ordered by priority-desc then ID-asc
- **`GoalContextSummary`** — read-only projection, active goals only, no executable fields
- **Validation**: non-empty ID/title, unique IDs, active-with-no-success-criteria → warning
- **Cross-registry**: linked_skills preserved as unresolved strings. Validation happens in app crate.
- **Missing file** → empty registry + warning, not error

### Session Capability Context (`openwand-app`)
- **`SessionCapabilityContext`** — combines skill + goal context summaries
- **`load_session_capability_context(openwand_dir)`** — reads both TOML files, performs cross-registry validation
- **`capability_context_as_text()`** — renders text for prompt inclusion
- **Cross-registry validation**: warns when goal links to unknown skill ID

### UI Skills/Goals Inspector (`openwand-app/ui`)
- **`SkillUiRow`**, **`GoalUiRow`** — display DTOs
- **`skill_rows()`**, **`goal_rows()`**, **`skill_goal_validation_lines()`** — view helpers
- **`skills_goals_safety_warning()`** — invariant text

### Dependency Pruning
- `openwand-skills`: deps = serde, serde_json, toml, thiserror, tracing (5 total)
- `openwand-goals`: deps = serde, serde_json, toml, thiserror, tracing (5 total)
- Removed: tokio, uuid, walkdir, comrak, chrono, anyhow, serde_yaml, openwand-memory

---

## Test Breakdown

| Area | Count |
|------|------:|
| Skills manifest + registry | 12 |
| Skills guard (source + dependency) | 5 |
| Goals manifest + registry | 13 |
| Goals guard (source + dependency) | 5 |
| Session capability context | 9 |
| UI skills/goals + guards | 8 |
| Guard/no-mutation (app) | 10 |
| **Total** | **62** |

---

## New Files

| File | Purpose |
|------|---------|
| `crates/skills/src/manifest.rs` | Skill DTOs |
| `crates/skills/src/registry.rs` | Validation + loading |
| `crates/skills/src/context.rs` | Session-safe projection |
| `crates/skills/tests/dependency_guards.rs` | 5 guard tests |
| `crates/goals/src/manifest.rs` | Goal DTOs |
| `crates/goals/src/registry.rs` | Validation + loading |
| `crates/goals/src/context.rs` | Session-safe projection |
| `crates/goals/tests/dependency_guards.rs` | 5 guard tests |
| `crates/app/src/session_capability.rs` | Combined context + cross-registry |
| `crates/app/src/ui/skills_goals_state.rs` | UI view helpers |
| `crates/app/src/ui/skills_goals_components.rs` | Desktop-gated render |
| `crates/app/tests/skills_goals_guards.rs` | 10 guard/no-mutation tests |

---

## TOML Config Format

### `.openwand/skills.toml`

```toml
[[skill]]
id = "rust-test-triage"
name = "Rust Test Triage"
description = "Helps interpret failing Rust test output."
category = "engineering"
enabled = true
tags = ["rust", "tests", "debugging"]
inputs = ["test output", "changed files"]
outputs = ["failure summary", "likely cause"]
constraints = ["Must not run commands directly"]
allowed_context = ["trace_summary", "test_output_summary"]
```

### `.openwand/goals.toml`

```toml
[[goal]]
id = "ship-governed-agent"
title = "Ship a governed agent product"
description = "Turn OpenWand into a usable governed agent."
status = "active"
priority = 100
tags = ["product", "governance"]
success_criteria = ["User can run a session from UI"]
constraints = ["Do not bypass policy gates"]
linked_skills = ["rust-test-triage"]
```

---

## Guard Coverage

### Source Guards (proven by scanning crate source)
- skills: no import of tool executor, policy engine, memory store, trace append, process command
- goals: no import of tool executor, policy engine, memory store, trace append, process command
- session_capability: no import of tools, policy, runner, process

### Dependency Guards (proven by Cargo.toml scanning)
- skills: only serde, serde_json, toml, thiserror, tracing
- goals: only serde, serde_json, toml, thiserror, tracing

### Structural Guards (proven by field scanning)
- Neither crate contains: command, shell, tool_name, tool_args, script, cwd, env, function_ref fields

### No-Mutation Guards (proven by dependency chain)
- Loading skills/goals cannot: append trace, mutate memory, modify git HEAD/index/worktree

---

## Central Invariant

```
Skills describe reusable capabilities.
Goals describe intended outcomes.
SessionRunner owns the loop.
Policy gates tools.
ToolExecutor executes tools.
Trace records authority.
Memory writes remain governed/projection-derived.

Skills and goals are context, not authority.
```

---

## Honest Caveats

- Wave 22 does not add a skill execution engine, goal planner, or autonomous decomposition.
- No workflow spawning, dynamic code loading, plugin execution, or LLM-generated skill/goal creation.
- Skills/goals inform the LLM via prompt context but the LLM may ignore them.
- Free-text descriptions/constraints may contain words that resemble commands — they are treated as text only, never interpreted.
- No remote skill marketplace or skill versioning.
- `build_session_runtime_with_provider()` and session capability context exist but are not yet wired to the CLI `run` command.
