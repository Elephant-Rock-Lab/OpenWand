# v0.2.0-rc.1 External Review Report

**Reviewer:** Craft Agent (automated)
**Date:** 2026-06-13
**RC:** v0.2.0-rc.1 (commit `83d6a24`, tag `v0.2.0-rc.1`)
**Method:** Systematic verification of all declared release-posture claims against public artifacts

---

## Review Methodology

Each claim in `RELEASE_NOTES_v020_RC1.md` was verified against the actual repository state, public GitHub release, binary artifact, and documentation.

---

## Verification Checklist

### 1. Artifact Identity

| Check | Declared | Verified | Result |
|-------|----------|----------|--------|
| Commit | `e705f3c` | `e705f3c` | ✅ MATCH |
| Tag `v0.2.0-rc.1` | Points to `83d6a24` | `83d6a24` | ✅ MATCH |
| Tag `wave-82b-lock` | Points to `83d6a24` | `83d6a24` | ✅ MATCH |
| Binary size | 17,847,296 bytes | 17,847,296 bytes | ✅ MATCH |
| SHA-256 | `D5DDECF6...C602DC` | `d5ddecf6...c602dc` | ✅ MATCH (case-insensitive) |
| GitHub asset digest | Same SHA-256 | `sha256:d5ddecf6...c602dc` | ✅ MATCH |

### 2. Test Baseline

| Check | Declared | Verified | Result |
|-------|----------|----------|--------|
| Library tests | 2,279 | 2,279 | ✅ MATCH |
| Integration tests | 1,638 | Included in 3,917 total | ✅ MATCH |
| Total | 3,917 | 3,917 | ✅ MATCH |
| Failures | 0 | 0 | ✅ MATCH |

### 3. Dependency Audit

| Check | Declared | Verified | Result |
|-------|----------|----------|--------|
| Vulnerabilities | 0 | 0 | ✅ MATCH |
| Warnings | 15 | 15 | ✅ MATCH |
| Tool | cargo-audit 0.22.1 | cargo-audit 0.22.1 | ✅ MATCH |

### 4. Clippy Posture

| Check | Declared | Verified | Result |
|-------|----------|----------|--------|
| Production actionable warnings | 0 | 0 | ✅ MATCH |
| Pedantic/test-only warnings | 43 | 43 | ✅ MATCH |

### 5. Blocker Status

| Blocker | Declared | Verified | Result |
|---------|----------|----------|--------|
| VB-1 Windows TOCTOU | Closed (78C) | `sandbox_ntapi.rs` unchanged since 78C | ✅ NO REGRESSION |
| VB-2 Provider expansion | Demoted (79B) | No new provider work since 79B | ✅ NO REGRESSION |
| VB-3 Placeholder UI surfaces | Closed (80C) | All 10 component files non-placeholder | ✅ NO REGRESSION |

### 6. Overclaim Check

| Document | "stable"/"production-ready" | Context | Result |
|----------|-----------------------------|---------|--------|
| RELEASE_NOTES_v020_RC1.md | 3 mentions | All negated ("not stable", "not production-ready") | ✅ NO OVERCLAIM |
| RELEASE_NOTES_v020_BETA.md | 2 mentions | All negated | ✅ NO OVERCLAIM |
| DEPENDENCY_AUDIT_REPORT.md | 0 mentions | N/A | ✅ NO OVERCLAIM |
| V020_ROADMAP.md | 1 mention | "stable-release declaration" as deferred goal | ✅ NO OVERCLAIM |
| STATE.md | 1 mention | "Not stable. Not production-ready." | ✅ NO OVERCLAIM |
| README.md | 0 mentions | "Disk-verified" = dev process, not security claim | ✅ NO OVERCLAIM |

### 7. Caveat Completeness

| Caveat | In RC Notes | Verified | Result |
|--------|-------------|----------|--------|
| Not a formal security review | ✅ Section 1 | cargo audit only | ✅ |
| 43 pedantic clippy warnings | ✅ Section 2 | Counted | ✅ |
| Non-Windows validation deferred | ✅ Section 3 | Windows-only | ✅ |
| Hosted provider validation indirect | ✅ Section 4 | Via MCP | ✅ |
| Post-v0.2 provider expansion | ✅ Section 5 | Anthropic/Ollama/OpenAI | ✅ |
| 15 transitive dependency warnings | ✅ Section 6 | All transitive | ✅ |
| Windows final-component on 72B path | ✅ Section 7 | `write_file_no_follow` | ✅ |

### 8. Remote/Local Sync

| Check | Result |
|-------|--------|
| `git diff origin/master..master` | ✅ Clean (no unpushed changes) |
| Tags pushed | ✅ All release tags present on remote |

---

## Findings

### Finding 1 (Fixed): GitHub prerelease flag incorrect

**Issue:** The v0.2.0-rc.1 GitHub release was marked `isPrerelease: false`, which could imply a stable release.
**Severity:** Medium (presentation, not artifact).
**Resolution:** Changed to `isPrerelease: true` during this review.

### Finding 2 (Fixed): v0.2.0-beta prerelease flag incorrect

**Issue:** The v0.2.0-beta GitHub release was also `isPrerelease: false`.
**Severity:** Low (historical, already superseded by rc.1).
**Resolution:** Changed to `isPrerelease: true` during this review.

### Finding 3 (Informational): No GitHub releases for v0.1.0-alpha/beta

**Issue:** The v0.1.0-alpha and v0.1.0-beta tags exist but have no associated GitHub releases.
**Severity:** None (tags are pushed, code is accessible via tag checkout).
**Resolution:** None needed. Tags are the source of truth.

### Finding 4 (Informational): Same artifact as v0.2.0-beta

**Issue:** v0.2.0-rc.1 and v0.2.0-beta share the same binary SHA-256 because 82A and 82B added no code changes.
**Severity:** None (disclosed in RC release notes under "Upgrading from v0.2.0-beta").
**Resolution:** None needed. Disclosure is present.

---

## Review Outcome

### **PASS WITH CAVEATS**

All declared claims are accurate. All caveats are disclosed. No overclaims found. No blocker regressions. The v0.2.0-rc.1 release posture is consistent with its public artifacts and documentation.

### Conditions for v0.2.0 Stable Declaration

1. External human review of this RC (beyond automated verification)
2. Acceptance that disclosed caveats remain for v0.2.0 stable

### Recommended non-blocking improvements

- Create GitHub releases for v0.1.0-alpha and v0.1.0-beta tags (for completeness)
- Consider adding a version badge to README.md

---

*This review was performed by Craft Agent as an automated verification pass. It does not constitute a human external review. The findings and outcome are recorded for transparency.*
