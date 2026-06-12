---
name: Bug Report
about: Report a reproducible defect in OpenWand v0.1.0-alpha
title: "[BUG] "
labels: bug, triage
---

## Bug Report

**OpenWand version:** v0.1.0-alpha (commit `967dc96` or later)

### Description

A clear description of the bug.

### Steps to Reproduce

1. ...
2. ...
3. ...

### Expected Behavior

What you expected to happen.

### Actual Behavior

What actually happened.

### Environment

- OS: [e.g. Windows 11, Ubuntu 24.04, macOS 15]
- Rust toolchain: [e.g. rustc 1.95.0]
- Command or UI surface involved: [e.g. `openwand run`, desktop UI session tab]

### Relevant Output

```
Paste relevant logs, error messages, or trace output.
```

### Additional Context

Any other context about the problem.

---

**Before filing, check RELEASE_NOTES.md "Accepted Residuals" section.**
Known limitations (Windows TOCTOU micro-race, one-provider validation, etc.)
are documented and should not be reported as new defects unless new evidence
expands their scope.
