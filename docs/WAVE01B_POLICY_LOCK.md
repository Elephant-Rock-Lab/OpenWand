# WAVE 01B — POLICY HALF-LOCK

**Status:** ✅ Policy half complete
**Date:** 2026-05-27

## Policy Commits Complete

| # | Scope | Status |
|---|-------|--------|
| 9 | Policy DTOs + decision model | ✅ Accepted |
| 10 | BuiltinPolicyEngine behavior | ✅ Accepted |
| 11 | Fail-closed edge cases + authority boundary + mode floor tests | ✅ Accepted |

## Design Decisions Locked

### 1. Rule-declared confirmation is canonical

```
PolicyEffect::Allow { risk, confirmation } supplies the rule's confirmation level.
confirmation_for_risk() is a fallback/helper, not the primary evaluation path.
apply_mode_floor() is the only post-rule adjustment.
```

This lets each rule encode its own confirmation level independently of global risk mapping, supporting exceptions without schema changes.

### 2. filter_tools() is defense-in-depth only

A tool removed from the LLM prompt surface must still be evaluated if the LLM somehow emits it. `evaluate_tool_call()` remains the authority boundary. Tested: `policy_filter_tools_is_not_authority_boundary` and `policy_filtered_tool_call_still_evaluates`.

### 3. MandatoryDeny rules outrank all

- Structurally: higher priority than BuiltinDefault
- Semantically: block dominance always wins
- Test: `policy_all_mandatory_deny_rules_cannot_be_weakened`

### 4. Fail-closed contract

- `PolicyEvaluation::fail_closed(error)` → Block + Critical + Escalate
- No matching rule → Block (not Allow)
- `PolicyError::Internal` → safe_message() hides implementation details
- Malformed arguments → still evaluated, not auto-allowed

## Test Count (Policy)

| Category | Count |
|---|---|
| Unit tests (inside src/) | 25 |
| Fail-closed integration tests | 6 |
| Authority boundary tests | 3 |
| Mode floor tests | 5 |
| Dependency guards | 1 |
| **Total policy** | **40** |

## Files

| File | Purpose |
|---|---|
| `error.rs` | PolicyError (4 variants + safe_message) |
| `tool.rs` | PolicyToolCall, PolicyToolDescriptor, PolicyToolSource |
| `request.rs` | PolicyRequest, PolicyContext, ToolFilterRequest |
| `decision.rs` | GateDecision, GateFinding, GateFindingResult, PolicyEvaluation |
| `rule.rs` | PolicyRule, PolicyRuleId, RuleClass, ToolMatcher, PolicyEffect |
| `risk.rs` | confirmation_for_risk, apply_mode_floor, risk_order |
| `mapping.rs` | Re-exports from risk |
| `engine.rs` | PolicyEngine trait |
| `builtin.rs` | batch1_rules() — 13 rules |
| `eval.rs` | BuiltinPolicyEngine impl + confirmation_rank |
| `tests/fail_closed.rs` | 6 fail-closed tests |
| `tests/authority_boundary.rs` | 3 authority boundary tests |
| `tests/mode_floor.rs` | 5 mode floor tests |
| `tests/dependency_guards.rs` | 1 guard test |

## Remaining 01b Work

| Item | Status |
|---|---|
| LLM DTOs + LlmClient trait | ⬚ Commit 12 |
| Mock LLM + stream contract tests | ⬚ Commit 13 |
| Tool-call buffer behavior | ⬚ Commit 14 |
| 01b full wave lock | ⬚ Commit 15 |

## Workspace Status

```
89 tests, zero failures, zero warnings.
4 crates with real code: core, trace, store, policy.
```

---

**Policy half of 01b is locked. Next: Commit 12 — LLM DTOs + LlmClient trait.**
