# WAVE04B_GOVERNED_GIT_OBSERVATION_LOCK

**Status:** ✅ LOCKED  
**Commit:** 3bb0344  
**Tests:** 412 passing, 0 failures  

## What shipped

Four read-only git observation tools, governed through the policy spine without escalation for observation-only operations.

## Tools

| Tool | Effect | Policy | Risk |
|------|--------|--------|------|
| `local__git_status` | `Git` | Allow (Inform in Conversational) | Low |
| `local__git_diff` | `Git` | Allow | Medium |
| `local__git_log` | `Git` | Allow (Inform in Conversational) | Low |
| `local__git_branch` | `Git` | Allow (Inform in Conversational) | Low |

## Policy architecture

The critical fix: the generic `confirm_git` rule's matcher was narrowed to **exclude** observation tool names:

```rust
ToolMatcher::All {
    matchers: vec![
        ToolMatcher::ToolEffect { effect: ToolEffect::Git },
        ToolMatcher::Not {
            matcher: Box::new(ToolMatcher::AnyOf {
                matchers: vec![
                    ToolMatcher::ToolName { exact: "local__git_status" },
                    ToolMatcher::ToolName { exact: "local__git_diff" },
                    ToolMatcher::ToolName { exact: "local__git_log" },
                    ToolMatcher::ToolName { exact: "local__git_branch" },
                ],
            }),
        },
    ],
}
```

This was necessary because the policy engine **aggregates all matching rules** and takes `max(risk)` + `max(confirmation)`. Without the exclusion, the generic `High/Escalate` rule would override the exact-name `Low/Auto` rules.

## Safety properties

| Property | Mechanism |
|----------|-----------|
| No user-supplied subcommands | Fixed argv constructed by handler |
| No shell invocation | `tokio::process::Command::new("git")` |
| Worktree verification | `git rev-parse --is-inside-work-tree` |
| Path filter safety | Relative only, no `..`, no `-` prefix, syntactic boundary |
| Windows absolute path | `starts_with('/')` check (Windows doesn't flag `/foo` as absolute) |
| Output capping | `cap_byte_output` at 200 KiB |
| Timeout | 15s with explicit kill + reap |
| Distinct from shell_exec | `ToolEffect::Git` ≠ `ToolEffect::Execute` (structural test) |

## Intentional conservatism

`..` components in path filters are rejected outright, even harmless paths like `src/../README.md`. This is documented and intentional — canonicalization adds complexity that can hide escape routes.

## Builder

```rust
pub fn local_tools_with_git_observation() -> BuiltinToolProvider
```

Extends `local_tools_with_shell_exec()` with four git observation tools.

## Files changed

| File | Change |
|------|--------|
| `crates/tools/src/local.rs` | 4 descriptors, 4 handlers, `run_fixed_git_command`, `verify_git_worktree`, `validate_git_path_filter`, `local_tools_with_git_observation` builder |
| `crates/policy/src/builtin.rs` | 4 exact-name observation rules + narrowed generic Git matcher |
| `crates/policy/tests/git_observation_policy.rs` | 6 policy tests |
| `crates/session/tests/governed_git_observation.rs` | 2 session lifecycle tests |

## Test delta

- Tools crate: +12 tests (2 structural, 4 path validation, 2 limit clamping, 4 integration)
- Policy crate: +6 tests (4 observation allows, 1 generic conservative, 1 mutation exclusion)
- Session crate: +2 tests (closed lifecycle, failed lifecycle)
- Unit tests in git_observation_tests: +3 (descriptor effect, internal runner, limit clamping)
- Total: 389 → 412

## Governance boundary preserved

```
read-only observation tools → exact-name allow/inform rules
all other Git tools         → generic conservative Git rule (High/Escalate)
```

Future git mutation tools (`git_add`, `git_commit`, etc.) will hit the generic rule and require escalation. The observation rules cannot be accidentally expanded to cover mutation tools.

## Next

Deferred to future waves:
- Git mutation (add, commit, checkout, push, etc.)
- Remote operations (fetch, pull, push)
- Structured output (parsed status entries, diff summaries)
- Memory integration (observation episodes, diff-to-memory)
