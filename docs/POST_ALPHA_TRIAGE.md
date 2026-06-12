# Post-Alpha Feedback Triage

**Release:** v0.1.0-alpha
**Date:** 2026-06-12
**Purpose:** Define how external review feedback is triaged and resolved.

---

## Issue Categories

| Category | Label | Description |
|----------|-------|-------------|
| Bug | `bug` | Reproducible defect in documented behavior |
| Security | `security` | Vulnerability, sandbox escape, or trust boundary violation |
| Provider | `provider-validation` | LLM provider/model compatibility report |
| UX | `ux` | Usability issue or UI improvement suggestion |
| Validation | `validation` | External reviewer checklist results |
| Documentation | `docs` | Claim/code discrepancy or stale documentation |

## Severity Levels

| Severity | Criteria | Response Target |
|----------|----------|-----------------|
| **Critical** | Remote code execution, data exfiltration without local access | Immediate |
| **High** | Sandbox escape without concurrent adversary, claim falsification | 24 hours |
| **Medium** | Attack requiring local concurrent access, new TOCTOU variant | 1 week |
| **Low** | Cosmetic, documentation staleness, defense-in-depth | Next wave |
| **Info** | Provider compatibility, UX suggestion, validation pass | Batched |

## Triage Workflow

```
Issue filed
  → Label with category (bug/security/provider/ux/validation/docs)
  → Check against Known Residuals (below)
  → If matches known residual → close with "already documented" reference
  → If new finding → assign severity → add to next wave scope
```

## Known Residuals (Do NOT Re-Report)

The following are **accepted v0.1.0-alpha limitations** documented in RELEASE_NOTES.md
and docs/DEFERRED_RISKS.md. Issues matching these should be closed with a reference
to the existing documentation, unless the reporter provides **new evidence** that
expands the scope or severity.

### 1. Windows per-component TOCTOU micro-race

- **ID:** DEFERRED-008
- **Status:** Substantially hardened (73C), not fully eliminated
- **Detail:** Per-component `symlink_metadata()` + re-verify on Windows. Micro-race
  window remains between check and I/O at each component. Requires undocumented
  NT API (`NtCreateFile` with `RootDirectory`) to fully close.
- **Re-open if:** Reporter demonstrates actual exploitation (not theoretical),
  or discovers the race window is larger than documented.

### 2. App test-module clippy warnings (57)

- **Status:** Accepted cosmetic
- **Detail:** All in `#[cfg(test)]` blocks. Zero affect production code.
- **Re-open if:** Warning count increases, or a warning indicates a real correctness issue.

### 3. Transitive dependency warnings (15)

- **Status:** Accepted pending upstream
- **Detail:** 13 unmaintained + 2 unsound, all transitive via Dioxus desktop or Loro CRDT.
  Zero direct dependency issues. Zero vulnerabilities.
- **Re-open if:** A vulnerability is discovered in a transitive dependency, or a
  direct dependency gains a warning.

### 4. One-provider validation scope

- **Status:** Validated against LM Studio + google/gemma-4-12b only
- **Detail:** Remote/hosted providers (OpenAI API, Anthropic, etc.) not tested.
- **Re-open if:** This is not a defect — it is a scope limitation. Provider validation
  results from new providers are welcome as `[PROVIDER]` issues.

### 5. Desktop UI process-lifecycle-only validation

- **Status:** Only process lifecycle smoke-tested (starts, runs 3s, exits cleanly)
- **Detail:** Desktop UI functional correctness is NOT claimed. 9 workflow surfaces
  use 3-line placeholder stubs.
- **Re-open if:** Binary fails to start or crashes on launch on a supported platform.

## Response Commitments

| Finding Type | First Response | Resolution Target |
|-------------|---------------|-------------------|
| Critical security | Immediate | Emergency wave |
| High security | 24 hours | Next wave |
| Claim falsification | 24 hours | Next wave |
| Bug (reproducible) | 3 days | Next appropriate wave |
| Provider result | 1 week | Batched into provider matrix |
| UX feedback | 1 week | Batched into UI planning |
| Validation pass | 1 week | Batched into next audit |

## Severity Escalation

If a finding is initially classified as Medium but later evidence shows broader impact:

1. Re-evaluate severity with new evidence
2. If upgraded to Critical/High, prioritize in next wave
3. Record re-evaluation in issue comments with rationale

## Wave Integration

Findings are batched into waves by severity and category:

- **Critical/High:** Integrated into next planned wave
- **Medium:** Grouped into a remediation wave
- **Low/Info:** Batched into the next documentation or cleanup wave
- **Provider results:** Collected into a provider validation matrix (future doc)

## Issue Closure Reasons

| Reason | When Used |
|--------|-----------|
| `already-documented` | Matches a known residual exactly |
| `duplicate` | Same issue already reported |
| `wontfix-alpha` | Accepted limitation for alpha, may revisit for beta |
| `fixed-in-wave-XX` | Resolved in a specific wave |
| `needs-more-info` | Cannot reproduce without additional details |

---

*This triage system ensures external feedback is handled consistently and
distinguishes new defects from accepted alpha limitations.*
