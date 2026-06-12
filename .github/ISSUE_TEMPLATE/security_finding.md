---
name: Security Finding
about: Report a security concern or vulnerability in OpenWand
title: "[SECURITY] "
labels: security, triage
---

## Security Finding

**OpenWand version:** v0.1.0-alpha (commit `967dc96` or later)

### Severity Assessment

- [ ] Critical: Remote code execution, data exfiltration without local access
- [ ] High: Local privilege escalation, sandbox escape without concurrent adversary
- [ ] Medium: Attack requiring local concurrent filesystem access or specific configuration
- [ ] Low: Informational, defense-in-depth improvement

### Description

Describe the security concern. Include the attack scenario, prerequisites,
and potential impact.

### Affected Component

- [ ] Filesystem sandbox (`crates/tools/src/sandbox.rs`)
- [ ] Tool execution (`crates/tools/src/local.rs`, `crates/tools/src/file_patch.rs`)
- [ ] Session/agent loop (`crates/session/src/runner.rs`)
- [ ] Policy engine (`crates/policy/`)
- [ ] LLM adapter (`crates/llm/`)
- [ ] Memory/store (`crates/memory/`, `crates/store/`)
- [ ] Desktop UI (`crates/app/src/ui/`)
- [ ] Other: ________

### Reproduction Steps

1. ...
2. ...
3. ...

### Known Residuals Context

Review `docs/DEFERRED_RISKS.md` and `RELEASE_NOTES.md` before filing.
The following are **already documented** and should not be re-reported
unless you have new evidence that expands their scope:

- **DEFERRED-008**: Windows per-component TOCTOU micro-race in intermediate
  directory traversal (substantially hardened in 73C, not fully eliminated)
- **Transitive dependency warnings**: 15 warnings (13 unmaintained + 2 unsound)
  via Dioxus desktop and Loro CRDT — none in direct dependencies
- **Static symlink escapes**: Already blocked at validation time (69A)

### Proposed Fix (Optional)

If you have a suggestion for how to address this.

### Disclosure Preferences

- [ ] Public discussion in this issue is acceptable
- [ ] I prefer coordinated disclosure — please contact me at: ________
