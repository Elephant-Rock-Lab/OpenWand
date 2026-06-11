# Remote Publication Checklist

**Status:** Prepared, not executed. Pending user decision.
**Prepared in:** Wave 70D
**RC artifact:** `wave-70b-lock` / `d6fa1f0`
**Packaging metadata:** `wave-70c-lock` / `e50356d`

---

## Pre-Push Verification

- [ ] Confirm working tree clean: `git status`
- [ ] Confirm HEAD matches `wave-70c-lock`: `git rev-parse HEAD`
- [ ] Confirm 27 commits ahead: `git rev-list origin/master..HEAD --count`
- [ ] Run full test suite: `cargo test --workspace --all-targets --all-features`
- [ ] Run clippy strict: `cargo clippy --all-features -- -D warnings` (11 non-app crates)
- [ ] Verify release binary: `Get-FileHash target/release/openwand.exe -Algorithm SHA256`
  - Expected: `826C5F87CCCD40DC35D58E472E9D8FD3A943F8F0B632508A73B06917061A6159`

## Push Steps

- [ ] Push master branch: `git push origin master`
- [ ] Push all RC-era tags: `git push origin wave-52a-lock wave-53a-lock ... wave-70c-lock`
  - Or push all tags: `git push origin --tags`
- [ ] Verify remote state: `git log origin/master --oneline -5`

## Post-Push Confirmation

- [ ] Confirm remote HEAD matches local HEAD
- [ ] Confirm tags visible on remote
- [ ] Confirm no unintended force-push

## What This Does NOT Do

- Does not create a GitHub Release
- Does not upload binary artifacts anywhere
- Does not announce or distribute
- Does not change the RC determination or deferred items
