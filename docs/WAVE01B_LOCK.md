# WAVE 01B — POLICY + LLM CONTRACTS — LOCK

**Status:** ✅ COMPLETE
**Date:** 2026-05-27

## Completed Commits

| # | Scope | Status |
|---|-------|--------|
| 9 | Policy DTOs + decision model | ✅ |
| 10 | BuiltinPolicyEngine behavior (Batch 1 rules) | ✅ |
| 11 | Fail-closed edge cases + authority boundary + mode floor tests | ✅ |
| 12 | LLM DTOs + LlmClient trait | ✅ |
| 13 | MockLlmClient + deterministic stream scripts | ✅ |
| 14 | ToolCallBuffer + malformed JSON protection | ✅ |

## Key Design Decisions

### Policy
- **Rule-declared confirmation is canonical** — `PolicyEffect::Allow { confirmation }` sets the level; `confirmation_for_risk()` is fallback only
- **Mode floor is the only post-rule adjustment** — `apply_mode_floor()` can only raise, never lower
- **MandatoryDeny rules outrank all** — higher priority, block dominance always wins
- **Fail-closed contract** — error → Block + Critical + Escalate; no matching rule → Block
- **filter_tools() is defense-in-depth only** — evaluate_tool_call() remains the authority boundary

### LLM
- **No System variant in LlmMessage** — system_prompt is a separate field on LlmRequest
- **No Error variant in LlmDelta** — all errors via `Result<LlmDelta, LlmError>`
- **ToolCallBuffer prevents partial tool calls** — malformed JSON never produces ToolCallComplete
- **API keys skipped in serde** — `LlmTarget::api_key` uses `#[serde(skip_serializing)]`
- **No Rig dependency yet** — DTO contract proven first, adapter later
- **No Rig types escape** — guard test confirms rig-core absent from dependency tree

## Test Count

| Crate | Unit | Integration | Dependency Guards | Total |
|---|---|---|---|---|
| openwand-core | 15 | 0 | 2 | 17 |
| openwand-trace | 19 | 0 | 2 | 21 |
| openwand-store | 0 | 9 | 0 | 9 |
| openwand-policy | 25 | 14 | 1 | 40 |
| openwand-llm | 23 | 0 | 2 | 25 |
| Scaffold crates (×6) | 6 | 0 | 0 | 6 |
| **Total** | **88** | **23** | **7** | **114** |

Note: LLM testing tests (11) require `--features testing` and are not counted in default workspace runs. With features: 25 LLM tests.

## Files Changed (01b)

### openwand-policy (new)
- `src/error.rs`, `src/tool.rs`, `src/request.rs`, `src/decision.rs`
- `src/rule.rs`, `src/risk.rs`, `src/mapping.rs`, `src/engine.rs`
- `src/builtin.rs`, `src/eval.rs`
- `tests/fail_closed.rs`, `tests/authority_boundary.rs`, `tests/mode_floor.rs`
- `tests/dependency_guards.rs`

### openwand-llm (new)
- `src/client.rs`, `src/request.rs`, `src/response.rs`, `src/error.rs`
- `src/tool_buffer.rs`, `src/testing.rs` (feature-gated)
- `tests/dependency_guards.rs`

### Documentation
- `docs/WAVE01B_POLICY_LOCK.md`
- `docs/WAVE01B_LOCK.md` (this file)

## 01b Acceptance Tests Satisfied

From `docs/WAVE01_ACCEPTANCE_TESTS.md`, 01b section:

- [x] `policy_read_allows_auto`
- [x] `policy_search_allows_or_informs`
- [x] `policy_unknown_blocks`
- [x] `policy_write_requires_confirmation`
- [x] `policy_delete_escalates_or_blocks`
- [x] `policy_fail_closed_on_error`
- [x] `policy_mode_floor_never_lowers_risk`
- [x] `llm_mock_stream_text`
- [x] `llm_mock_stream_tool_call_complete`
- [x] `llm_tool_buffer_malformed_json`
- [x] `llm_error_mapping`
- [x] `llm_no_rig_types_escape`

## Deferred to Later Waves

| Item | Target | Reason |
|---|---|---|
| Rig adapter implementation | Post-01b manual smoke | Contract proven, adapter needs real API keys |
| Circuit breaker / retry logic | Wave 02+ | Infrastructure concern |
| Reasoning normalization | Wave 02+ | Provider-specific, needs real streams |
| Thinking budget mapping | Wave 02+ | Provider-specific params |
| Usage → TokenUsageSnapshot conversion | Wave 02+ | Needs Rig types |
| User/project rule layering | Wave 02+ | BuiltinPolicyEngine sufficient for Batch 1 |
| LLM-assisted semantic gates | v2+ | Future policy capability |
| Rate limiting / budget gates | v2+ | Future gate family |

## Workspace Status

```
114 tests, zero failures, zero warnings.
5 crates with real code: core, trace, store, policy, llm.
```

---

**Wave 01b is closed. Next: Wave 01c — Tools + MCP Pool.**
