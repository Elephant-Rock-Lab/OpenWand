# Wave 17a: Bare-Remote Runtime Push Closure — Lock

**Commit:** `63b3862`
**Date:** 2026-06-03
**Status:** LOCKED
**Tests:** 1427 total, zero failures

## Scope

Prove the real `LocalPushExecutionBackend` push path against a local bare git remote: an approved exact fast-forward push updates only the intended existing branch ref, creates no tags or new branches, leaves unrelated remote refs unchanged, confirms the post-push ref via ls-remote, blocks missing and non-fast-forward remote refs, and leaves local HEAD, index, and worktree unchanged. No new product surface. No new CLI. No new governance model.

## Wave 17 Caveat Status

**CLOSED.** The Wave 17 lock doc stated:

> Runtime git integration tests using local bare remotes are not yet included.

Wave 17a closes that gap.

## Lock Statement

Wave 17a closes the Wave 17 caveat by proving the real `LocalPushExecutionBackend` against a local bare remote. The governed push path updates exactly one existing branch ref, creates no branches or tags, leaves unrelated refs untouched, confirms post-push state via ls-remote, and leaves local HEAD/index/worktree unchanged.

## Fixture Design

```
tempdir/
  work/          # non-bare working repository
  remote.git/    # bare repository created via git init --bare
  seed/          # optional seed clone for non-FF test
```

Test fixture helpers may invoke git commands required to construct isolated local repositories. These fixture commands are not part of the governed backend command surface. The production backend command-surface guard applies only to `LocalPushExecutionBackend`.

## Test Coverage (9 tests)

| Test | What it proves |
|------|---------------|
| `successful_push_updates_existing_bare_remote_branch` | Pre-push old → post-push new on exact ref |
| `successful_push_does_not_create_new_remote_branch` | No unexpected refs appear |
| `successful_push_does_not_create_tag` | `refs/tags/*` unchanged |
| `successful_push_does_not_change_unrelated_remote_branch` | `refs/heads/other-branch` unchanged |
| `blocked_push_does_not_change_bare_remote_ref` | Blocked → remote + local unchanged |
| `missing_remote_branch_blocks_without_creation` | No fallback branch creation |
| `non_fast_forward_bare_remote_blocks` | Diverged remote → blocked, ref unchanged |
| `post_push_ls_remote_confirms_new_commit` | `observe_remote_ref` after push == proposed commit |
| `successful_push_leaves_local_head_index_worktree_unchanged` | HEAD/index/worktree before == after |

## What Did Not Ship

No new CLI, DTOs, persistence format, policy predicates, allowed git commands, force push, tag push, branch creation, fetch, pull, release, remote delete, remote rollback, live rollback, arbitrary shell, or general git execution.

## Honest Caveats After Wave 17a

Wave 17a proves git behavior against a local bare remote. It still does not prove:
- Provider-host branch protection
- Remote CI status
- Signed-commit policy
- Organization policy
- PR requirements
- Post-push provider-side audit

Those remain future provider-observation or post-push verification seams.
