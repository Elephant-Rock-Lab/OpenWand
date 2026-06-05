# OpenWand Capability-to-Code Traceability Matrix

Generated from disk-verified reconnaissance at Wave 40. Baseline: 2824 tests, zero failures.

---

## Primary Capability Rows

These 15 capabilities form the core evidence chain. Each has a workflow source, app persistence, CLI, UI, guard tests, and a sealed lock doc.

| # | Capability | Wave | Workflow Source | App Persistence | CLI Command | UI State | UI Components | CLI Test | Guard Test | Persistence Root | ID Prefix | ID Type | ID Constructor | Lock Doc | Tag |
|---|------------|------|----------------|-----------------|-------------|----------|---------------|----------|------------|-----------------|-----------|---------|----------------|----------|-----|
| 1 | Task Planning | 23 | `plan.rs`, `plan_review.rs`, `validation.rs`, `builder.rs`, `context.rs` | `task_planning.rs` | `task-plan` (create/show/latest/review) | `task_plan_state.rs` | `task_plan_components.rs` | `task_plan_cli.rs` | `task_plan_guards.rs` | `task_plans/` | `tpl_` | TaskPlanId | `validation.rs` | `WAVE23_WORKFLOW_TASK_PLANNING_LOCK.md` | `wave-23-lock` |
| 2 | Workflow Proposal | 24 | `workflow_proposal.rs`, `workflow_proposal_builder.rs`, `workflow_proposal_review.rs`, `workflow_proposal_validation.rs` | `workflow_proposal.rs` | `workflow-proposal` (create/show/latest/review) | `workflow_proposal_state.rs` | `workflow_proposal_components.rs` | `workflow_proposal_cli.rs` | `workflow_proposal_guards.rs` | `workflow_proposals/` | `wfp_` | WorkflowProposalId | `workflow_proposal_validation.rs` | `WAVE24_WORKFLOW_PROPOSAL_LOCK.md` | `wave-24-lock` |
| 3 | Workflow Readiness | 25 | `workflow_readiness.rs`, `workflow_readiness_evaluator.rs`, `workflow_readiness_validation.rs` | `workflow_readiness.rs` | `workflow-readiness` (evaluate/show/latest) | `workflow_readiness_state.rs` | `workflow_readiness_components.rs` | `workflow_readiness_cli.rs` | `workflow_readiness_guards.rs` | `workflow_readiness/` | `wfrd_` | WorkflowReadinessId | `workflow_readiness_validation.rs` | `WAVE25_WORKFLOW_READINESS_LOCK.md` | `wave-25-lock` |
| 4 | Workflow Execution | 26 | `workflow_execution_gate.rs`, `workflow_run.rs`, `workflow_run_lifecycle.rs`, `workflow_run_validation.rs`, `workflow_stage_progression.rs` | `workflow_execution.rs` | `workflow-execution` (create/show/latest) | `workflow_execution_state.rs` | `workflow_execution_components.rs` | `workflow_execution_cli.rs` | `workflow_execution_guards.rs` | `workflow_runs/` | `wfx_` | WorkflowExecutionId | `workflow_run.rs` | `WAVE26_WORKFLOW_EXECUTION_LOCK.md` | `wave-26-lock` |
| 5 | Action Routing | 27 | `workflow_action_route.rs`, `workflow_action_route_gate.rs`, `workflow_action_route_validation.rs` | `workflow_action_routing.rs` | `workflow-action` (route/show/latest) | `workflow_action_routing_state.rs` | `workflow_action_routing_components.rs` | `workflow_action_routing_cli.rs` | `workflow_action_routing_guards.rs` | `workflow_action_routes/` | `war_` | WorkflowActionRouteId | `workflow_action_route_validation.rs` | `WAVE27_WORKFLOW_ACTION_ROUTING_LOCK.md` | `wave-27-lock` |
| 6 | Action Outcome | 29 | `workflow_action_outcome.rs`, `workflow_action_outcome_gate.rs`, `workflow_action_outcome_validation.rs` | `workflow_action_outcome.rs` | `workflow-action-outcome` (create/show/latest) | `workflow_action_outcome_state.rs` | `workflow_action_outcome_components.rs` | `workflow_action_outcome_cli.rs` | `workflow_action_outcome_guards.rs` | `workflow_action_outcomes/` | `wao_` | WorkflowActionOutcomeId | `workflow_action_outcome_validation.rs` | `WAVE29_WORKFLOW_ACTION_OUTCOME_LOCK.md` | `wave-29-lock` |
| 7 | Reconciliation | 30 | `workflow_reconciliation.rs`, `workflow_reconciliation_validation.rs`, `workflow_reconciliation_gate.rs` | `workflow_reconciliation.rs` | `workflow-reconciliation` (reconcile/show-revision/latest-revision/latest) | `workflow_reconciliation_state.rs` | `workflow_reconciliation_components.rs` | `workflow_reconciliation_cli.rs` | `workflow_reconciliation_guards.rs` | `workflow_reconciliations/` + `workflow_run_revisions/` | `wrc_` / `wrr_` | WorkflowReconciliationId / WorkflowRunRevisionId | `workflow_reconciliation_validation.rs` | `WAVE30_WORKFLOW_RECONCILIATION_LOCK.md` | `wave-30-lock` |
| 8 | Continuation | 31 | `workflow_continuation.rs`, `workflow_continuation_validation.rs`, `workflow_next_action_selector.rs` | `workflow_continuation.rs` | `workflow-continuation` (evaluate/show/latest) | `workflow_continuation_state.rs` | `workflow_continuation_components.rs` | `workflow_continuation_cli.rs` | `workflow_continuation_guards.rs` | `workflow_continuation/` | `wcr_` / `wnap_` | WorkflowContinuationReadinessId / WorkflowNextActionProposalId | `workflow_continuation_validation.rs` | `WAVE31_WORKFLOW_CONTINUATION_LOCK.md` | `wave-31-lock` |
| 9 | Next-Action Review | 32 | `workflow_next_action_review.rs` | `workflow_next_action_review.rs` | `workflow-next-action-review` (review/show/latest) | — | — | `workflow_next_action_review_cli.rs` | — | `workflow_next_action_reviews/` | `wnar_` | WorkflowNextActionReviewId | `app/main.rs` | `WAVE32_NEXT_ACTION_REVIEW_ROUTING_READINESS_LOCK.md` | `wave-32-lock` |
| 10 | Routing Readiness | 32 | `workflow_routing_readiness.rs`, `workflow_routing_readiness_gate.rs` | `workflow_routing_readiness.rs` | `workflow-routing-readiness` (evaluate/show/latest) | `workflow_routing_readiness_state.rs` | `workflow_routing_readiness_components.rs` | — | `workflow_routing_readiness_guards.rs` | `workflow_routing_readiness/` | `wrrd_` | WorkflowRoutingReadinessId | `workflow_routing_readiness_gate.rs` | `WAVE32_NEXT_ACTION_REVIEW_ROUTING_READINESS_LOCK.md` | `wave-32-lock` |
| 11 | Next-Action Routing | 33 | `workflow_next_action_routing_gate.rs` | `workflow_next_action_routing.rs` | `workflow-next-action-routing` (route/show/latest) | `workflow_next_action_routing_state.rs` | `workflow_next_action_routing_components.rs` | `workflow_next_action_routing_cli.rs` | `workflow_next_action_routing_guards.rs` | `workflow_next_action_routing/` | `wnaroute_` | WorkflowNextActionRoutingId | `workflow_next_action_routing_gate.rs` | `WAVE33_NEXT_ACTION_ROUTING_GATE_LOCK.md` | `wave-33-lock` |
| 12 | Loop Controller | 34 | `workflow_loop_controller.rs`, `workflow_loop_state.rs`, `workflow_loop_recommendation.rs` | `workflow_loop_controller.rs` | `workflow-loop` (evaluate/show/latest) | `workflow_loop_controller_state.rs` | `workflow_loop_controller_components.rs` | `workflow_loop_controller_cli.rs` | `workflow_loop_controller_guards.rs` | `workflow_loop_controller/` | `wlc_` | WorkflowLoopControllerId | `workflow_loop_controller.rs` | `WAVE34_WORKFLOW_LOOP_CONTROLLER_LOCK.md` | `wave-34-lock` |
| 13 | Command Composer | 35 | `workflow_command_composer.rs`, `workflow_command_descriptor.rs`, `workflow_manual_operation.rs` | `workflow_command_composer.rs` | `workflow-command` (compose/show/latest) | `workflow_command_composer_state.rs` | `workflow_command_composer_components.rs` | `workflow_command_composer_cli.rs` | `workflow_command_composer_guards.rs` | `workflow_command_composer/` | `wcc_` | WorkflowCommandComposerId | `workflow_command_composer.rs` | `WAVE35_WORKFLOW_COMMAND_COMPOSER_LOCK.md` | `wave-35-lock` |
| 14 | Command Review | 36 | `workflow_command_review.rs`, `workflow_command_review_validation.rs` | `workflow_command_review.rs` | `workflow-command-review` (review/show/latest) | `workflow_command_review_state.rs` | `workflow_command_review_components.rs` | `workflow_command_review_cli.rs` | `workflow_command_review_guards.rs` | `workflow_command_reviews/` | `wcrv_` | WorkflowCommandReviewId | `workflow_command_review.rs` | `WAVE36_WORKFLOW_COMMAND_REVIEW_LOCK.md` | `wave-36-lock` |
| 15 | Manual Result | 37 | `workflow_manual_result.rs`, `workflow_manual_result_validation.rs` | `workflow_manual_result.rs` | `workflow-manual-result` (create/show/latest) | `workflow_manual_result_state.rs` | `workflow_manual_result_components.rs` | `workflow_manual_result_cli.rs` | `workflow_manual_result_guards.rs` | `workflow_manual_results/` | `wmr_` | WorkflowManualResultId | `workflow_manual_result_validation.rs` | `WAVE37_WORKFLOW_MANUAL_RESULT_LOCK.md` | `wave-37-lock` |

---

## Supporting Code Rows

These modules support primary capabilities but do not have their own persistence root, ID prefix, CLI command, or lock doc.

| Module | Location | Supports | Role |
|--------|----------|----------|------|
| `workflow_approval_bridge.rs` | `crates/app/src/` | Action Outcome (29), Action Routing (27) | Bridges approval resolution from policy to workflow outcome |
| `workflow_session_bridge.rs` | `crates/app/src/` | Action Routing (27), Live Session (28) | Bridges session runner to workflow routing |
| `tool_intent_resolution.rs` | `crates/workflow/src/` | Readiness (25), Proposal (24) | Resolves tool intent to descriptive capability categories |
| `workflow_live_route_integration.rs` | `crates/app/tests/` | Live Session Bridge (28), Action Routing (27) | Integration test for live routing through session |
| `workflow_live_session_bridge.rs` | `crates/app/tests/` | Live Session Bridge (28) | Integration test for session bridge |

---

## Non-Workflow CLI Surfaces

These CLI commands exist in the binary but are out of scope for the workflow capability matrix:

| Command | Purpose |
|---------|---------|
| `trace-verify` | Trace integrity verification |
| `session-rebuild` | Session reconstruction |
| `run` | Desktop app launch |
| `eval` | Evaluation runner |

---

## ID Prefix Registry

| Prefix | Type | Constructor Location | Wave |
|--------|------|---------------------|------|
| `tpl_` | TaskPlanId | `crates/workflow/src/validation.rs` | 23 |
| `tpr_` | TaskPlanReviewId | `crates/workflow/src/plan_review.rs` | 23 |
| `wfp_` | WorkflowProposalId | `crates/workflow/src/workflow_proposal_validation.rs` | 24 |
| `wfr_` | WorkflowProposalReviewId | `crates/workflow/src/workflow_proposal_validation.rs` | 24 |
| `wfrd_` | WorkflowReadinessId | `crates/workflow/src/workflow_readiness_validation.rs` | 25 |
| `wfx_` | WorkflowExecutionId | `crates/workflow/src/workflow_run.rs` | 26 |
| `war_` | WorkflowActionRouteId | `crates/workflow/src/workflow_action_route_validation.rs` | 27 |
| `wao_` | WorkflowActionOutcomeId | `crates/workflow/src/workflow_action_outcome_validation.rs` | 29 |
| `wrc_` | WorkflowReconciliationId | `crates/workflow/src/workflow_reconciliation_validation.rs` | 30 |
| `wrr_` | WorkflowRunRevisionId | `crates/workflow/src/workflow_reconciliation_validation.rs` | 30 |
| `wcr_` | WorkflowContinuationReadinessId | `crates/workflow/src/workflow_continuation_validation.rs` | 31 |
| `wnap_` | WorkflowNextActionProposalId | `crates/workflow/src/workflow_continuation_validation.rs` | 31 |
| `wnar_` | WorkflowNextActionReviewId | `crates/app/src/main.rs` | 32 |
| `wrrd_` | WorkflowRoutingReadinessId | `crates/workflow/src/workflow_routing_readiness_gate.rs` | 32 |
| `wnaroute_` | WorkflowNextActionRoutingId | `crates/workflow/src/workflow_next_action_routing_gate.rs` | 33 |
| `wlc_` | WorkflowLoopControllerId | `crates/workflow/src/workflow_loop_controller.rs` | 34 |
| `wcc_` | WorkflowCommandComposerId | `crates/workflow/src/workflow_command_composer.rs` | 35 |
| `wcrv_` | WorkflowCommandReviewId | `crates/workflow/src/workflow_command_review.rs` | 36 |
| `wmr_` | WorkflowManualResultId | `crates/workflow/src/workflow_manual_result_validation.rs` | 37 |

**Note:** `wnar_` (WorkflowNextActionReviewId) is the only ID prefix constructed in the app crate rather than the workflow crate. All others are content-addressed in workflow validation modules.

---

## Authority Boundaries

```
Workflow owns evidence and progression.
Session owns execution.
Policy owns gates.
Tools own effects.
Trace owns authority.
Memory remains governed/derived.
Operators own manual action.
OpenWand records what it can prove.
```

---

## Lock Document Inventory

- **Pre-workflow lock docs (Waves 00–22):** 57 files
- **Workflow evidence lock docs (Waves 23–37):** 15 files
- **Doctrine lock docs (Waves 38–39):** 2 files
- **Total before Wave 40:** 74 files

---

## Test Baseline

2824 tests, zero failures, across 14 workspace crates and 131 integration test files.
