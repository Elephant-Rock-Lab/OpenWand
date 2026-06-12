---
name: Validation Checklist
about: External reviewer checklist for validating OpenWand v0.1.0-alpha
title: "[VALIDATION] "
labels: validation, triage
---

## External Validation Checklist

Reviewer: [Your name/handle]
Date: [Date of review]
OpenWand version: v0.1.0-alpha (`967dc96`)

### Build Verification

- [ ] `cargo build --workspace --all-targets --all-features` succeeds
- [ ] `cargo build -p openwand-app --features desktop` succeeds
- [ ] `cargo build -p openwand-app --release` produces working binary

### Test Verification

- [ ] `cargo test --workspace` passes (2,266 lib + 22 integration, 0 failures)
- [ ] `cargo test -p openwand-app --tests` passes (CLI surface + integration)

### Claim Accuracy

Review RELEASE_NOTES.md against actual code:

- [ ] Test counts match actual `cargo test` output
- [ ] Security hardening claims match `crates/tools/src/sandbox.rs` code
- [ ] "Not Claimed" section accurately reflects validation scope
- [ ] Accepted residuals match `docs/DEFERRED_RISKS.md`

### Document Consistency

- [ ] `RELEASE_NOTES.md` wave history matches `WAVES.md`
- [ ] `docs/RELEASE_CANDIDATE_LEDGER.md` tag count matches `git tag -l`
- [ ] `docs/RC_VALIDATION_REPORT.md` test counts match actual
- [ ] `docs/FINAL_AUDIT_REPORT.md` findings are resolved
- [ ] `STATE.md` reflects current release state

### Security Review

- [ ] `resolve_workspace_path()` rejects traversal/absolute/symlink escapes
- [ ] `write_file_no_follow()` uses no-follow flags on final component
- [ ] `WorkspaceWriteHandle` walks components with O_NOFOLLOW (Unix)
- [ ] Windows per-component reparse point detection is implemented
- [ ] No backend crate imports in UI components

### Honest Scoping

- [ ] No document claims "production-ready"
- [ ] No document claims "all providers validated"
- [ ] No document claims "race-proof" on any platform
- [ ] Windows TOCTOU residual is disclosed in every relevant document
- [ ] Alpha classification is prominent

### Findings

List any discrepancies found between documentation and actual state:

1. ...
2. ...

### Overall Assessment

- [ ] PASS — documents and code are internally consistent
- [ ] PASS WITH NOTES — minor discrepancies noted above
- [ ] FAIL — significant overclaims or inconsistencies found (detail below)

### Additional Notes

Any other observations from the review.
