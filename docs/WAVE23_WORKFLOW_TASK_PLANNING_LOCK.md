# Wave 23 — Workflow / Task Planning — LOCK

**Committed:** 4 commits (batched for disk-space efficiency)
**Baseline:** 1673 tests (Wave 22 locked)
**Final:** 1730 tests (+57), zero failures

---

## What Shipped

### New Crate: `openwand-workflow` (6 dependencies, no `openwand-core`)

| Module | Purpose |
|--------|---------|
| `plan.rs` | TaskPlan, TaskPlanStep, TaskPlanId, TaskPlanStepKind, assumptions, risks, approval requirements, evidence links |
| `plan_review.rs` | TaskPlanReview, TaskPlanReviewId, TaskPlanReviewDecision, TaskPlanFeedback, `creates_execution_grant: false` |
| `validation.rs` | validate_task_plan, validate_task_plan_review, BLAKE3 content-addressed IDs |
| `builder.rs` | Deterministic build_task_plan from TaskPlanInput (no LLM) |
| `context.rs` | TaskPlanInput (string-based context), evidence helpers |

### App Crate Integration

| File | Purpose |
|------|---------|
| `crates/app/src/task_planning.rs` | Persistence (JSON under `eval_reports/task_plans/`), supersession in save |
| `crates/app/src/main.rs` | CLI: `openwand task-plan create/show/latest/review` (no execute) |
| `crates/app/src/ui/task_plan_state.rs` | UI view helpers, safety warning |
| `crates/app/src/ui/task_plan_components.rs` | Desktop-gated render placeholder |

---

## Dependency Posture

### `openwand-workflow`

```toml
serde, serde_json, blake3, chrono, thiserror, tracing
```

**No dependency on:** openwand-core, openwand-session, openwand-tools, openwand-policy, openwand-memory, openwand-trace, openwand-store, openwand-skills, openwand-goals, tokio, uuid.

Risk levels use `String`. Evidence kinds use workflow-local enum. Context input carries `Vec<String>` summaries.

---

## Test Breakdown

| Area | Count |
|------|------:|
| Workflow crate unit tests (builder) | 8 |
| Workflow crate guards (dep, import, field, JSON shape) | 11 |
| Persistence + durability + allowed/forbidden writes | 12 |
| CLI integration tests (real binary) | 8 |
| UI state + guards + no-mutation | 18 |
| **Total** | **57** |

---

## Central Invariant

```
Skills describe capabilities.
Goals describe outcomes.
Plans describe intended work.
Workflows execute only in future governed execution waves.

A plan is not execution.
A reviewed plan is not an execution grant.
A plan step is not a tool call.
A required approval is not an approval record.
```

---

## Patch Compliance

| Patch | Status |
|-------|--------|
| 1. No `openwand-core` dependency | ✅ 6 deps only, risk levels are strings, evidence kinds are local enum |
| 2. Structural executable-field protection | ✅ Source guard + serialized-shape guard + struct whitelist tests |
| 3. Durability tests in persistence commit | ✅ `prior_plan_reviews_remain_persisted` + `latest_plan_review_supersedes_prior_for_lookup` |
| 4. Explicit allowed vs forbidden writes | ✅ `task_plan_creation_writes_only_task_plan_evidence`, `task_plan_creation_does_not_write_governance_records`, `plan_review_does_not_write_governance_records` |

---

## CLI Surface

```bash
openwand task-plan create --intent <text> [--output-dir] [--json]
openwand task-plan show <plan-id> [--output-dir] [--json]
openwand task-plan latest [--goal-id <id>] [--skill-id <id>] [--output-dir] [--json]
openwand task-plan review approve --plan-id <id> --reviewer <name> --rationale <text> [--json]
openwand task-plan review reject --plan-id <id> --reviewer <name> --rationale <text> --feedback <text> [--json]
openwand task-plan review request-changes --plan-id <id> --reviewer <name> --rationale <text> --feedback <text> [--json]
```

**No `execute` subcommand.** Task-plan is a top-level command, not gated behind `eval`.

---

## Honest Caveats

- Wave 23 does not add workflow execution, autonomous decomposition, or scheduling.
- A reviewed plan is still only evidence — future waves must translate plans to executable workflows.
- The deterministic builder produces simple plans — real-world planning may need LLM assistance.
- No plan versioning or branching — a new intent creates a new plan.
- No concurrent plan editing — single-user assumption holds for now.
- Plan steps reference skills/goals by ID but don't invoke them.
