# OUTPUT_GUARD_INTEGRATION_LOCK

**Status:** ✅ LOCKED  
**Commits:** e66b3db → d646b68 (4 commits)  
**Tests:** 652 passing, 0 failures  

## What happened

The `openwand-controller` crate introduced a parallel safety system alongside OpenWand's existing governance. It was dissolved. The valuable piece — context-sensitive forbidden-action detection — was extracted into `openwand-policy` as a narrow output screening utility.

## What was deleted

| From controller | Why deleted |
|----------------|-------------|
| `EvidencePacket` / `EvidenceItem` / `EvidenceStatus` | Duplicated `EvidenceKind` from memory crate |
| `AgentRuntime` trait | Never called from runner |
| `SbcRuntimeController` | Dissolved into policy functions |
| `ControllerTrace` | Replaced by `GateEvent::OutputScreened` |
| `ControllerResult` | Replaced by `ScreenedOutput` |
| `PolicyProfile` | Duplicated `InteractionMode` + `RiskLevel` |
| `SessionControllerConfig` | Replaced by `OutputGuardConfig` |
| `compile_prompt` / `semantic_fact_matches` | Not used in runner path |
| `overclaim_traps` | Dead field |
| `repair_attempted` / `repair_accepted` | Always false |

The entire `openwand-controller` crate was removed.

## What was kept

The context-sensitive forbidden-action detector lives in `openwand-policy::output_guard`:
- `screen_output()`: checks text against forbidden action patterns
- `guard_output()`: replaces text with natural-language fallback if screening fires
- Context-sensitive detection: negation ("do not X"), quotation, capability description all pass
- Imperative detection: "use X to Y", "run X" are flagged
- Word boundary matching prevents partial matches

## Architecture

```text
Model generates text → streamed to user (live)
                       → screened by output_guard
                       → durable record uses screened text
                       → GateEvent::OutputScreened in trace
```

This is NOT pre-disclosure safety enforcement.
It is post-hoc durable-record correction.
Streaming remains live.

## Key design decisions

1. **Streaming stays ON.** Disabling streaming for 10-30 second local model inference makes the product feel broken. The tradeoff is honest: the user may briefly see raw text, but the durable record is clean.

2. **Natural-language fallback.** The fallback says "I've reviewed my response and corrected it. My original answer referenced operations that should be performed only with explicit approval (such as 'git pull')." Not a JSON blob.

3. **Narrow forbidden list.** Only cross-boundary operations: `git pull`, `git push`, `pip install`, `npm install`. NOT `write`, `edit`, `delete`, `rm` — those are tool actions already governed by ToolGate.

4. **Same trace family.** `GateEvent::OutputScreened` is a variant of `GateEvent`, not a new event family. Output screening is a gate decision, same as tool gating.

5. **No new crate.** The screening logic lives in `openwand-policy`, alongside the existing policy engine. No parallel governance system.

## Test delta

629 → 652 = +23 tests

## Gaps (honest list)

- The runner doesn't produce output guard events in the existing acceptance test run because the mock LLM returns safe text
- No integration test that proves a real run produces a `GateEvent::OutputScreened` trace event
- `OutputGuardConfig` is always `None` in the app service — the conservative default is not wired yet
- No UI for configuring forbidden actions
- The `ScreenedOutput.base_text` is stored but not surfaced in UI (for debugging only)
