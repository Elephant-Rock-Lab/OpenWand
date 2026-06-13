# v0.4.0 Roadmap — Post-v0.3 Reset

**Created:** 2026-06-13 (Wave 87A)
**Status:** Planning
**Predecessor:** v0.3.0 stable (`4d2efd6`)

---

## v0.3.0 Stable State (Baseline)

| Metric | Value |
|--------|-------|
| Tests | 3,939 Windows / 3,934 Linux |
| Failures | 0 |
| Crates | 14 |
| Binary size | 18,941,440 bytes (~18.0 MB) |
| Dependency vulnerabilities | 0 |
| Dependency warnings | 15 (all transitive) |
| Workflow UI surfaces | 10/10 complete, 5/10 live-wired |
| Providers validated | 5 models, 2 families |
| Platforms validated | Windows (full), Linux (compile + tests) |
| SHA-256 | `A5B594A33495E8AE61FB96C77F66042247AEBA768A8E59580F4C6995431FAAC5` |

---

## v0.3.0 Caveats Carried Forward

These caveats from v0.3.0 remain in effect unless explicitly resolved:

| # | Caveat | v0.4.0 Target? |
|---|--------|----------------|
| 1 | Not a formal security review | No (external dependency) |
| 2 | 43 pedantic/test-only clippy warnings | Optional patch |
| 3 | Linux GUI runtime not validated | **v0.4.0 candidate** |
| 4 | macOS validation deferred | Deferred (no macOS env) |
| 5 | Hosted provider validation indirect | Post-v0.4 |
| 6 | Post-v0.3 provider expansion pending | Post-v0.4 |
| 7 | 15 transitive dependency warnings | Upstream-blocked |
| 8 | Windows final-component on 72B no-follow path | Accepted |

---

## Backlog Triage

### Category A: Workflow Execution Depth (v0.4.0 primary candidates)

v0.3.0 wired the inspector to *display* live workflow data. The surfaces observe stored records but do not drive the workflow lifecycle. The next logical step is to make the workflow lifecycle *executable* through the desktop.

| Item | Description | Complexity | Priority |
|------|-------------|------------|----------|
| Live approval resolution from desktop | Wire approval bridge to actual desktop interaction (approve/reject from UI) | High | P1 |
| Workflow run initiation from desktop | Create a workflow run from the desktop UI (currently CLI-only) | High | P1 |
| Real-time inspector updates | Inspector refreshes as workflow stages progress (currently load-on-select) | Medium | P2 |
| Evidence chain export from desktop | Export audit packet via UI button (currently CLI-only) | Medium | P2 |

**Assessment:** This is the highest-impact work. It transforms the desktop from an observation tool to an operational tool. However, it requires careful authority-boundary design — the UI must never gain direct execution authority. All operations must flow through existing CLI/session/policy gates.

### Category B: Platform Hardening (v0.4.0 environment-gated)

| Item | Description | Complexity | Priority |
|------|-------------|------------|----------|
| Linux GUI runtime validation | Launch desktop on native Linux display (requires X11/Wayland) | Medium | P2 |
| macOS compilation check | Compile workspace + desktop on macOS | Low | P3 |
| Linux release binary | Build optimized binary on Linux for distribution | Medium | P2 |

**Assessment:** Linux GUI runtime is the most valuable remaining platform gap. macOS remains environment-gated. These are confidence-builders, not architecture work.

### Category C: Provider Expansion (post-v0.4)

| Item | Description | Complexity | Priority |
|------|-------------|------------|----------|
| Anthropic Claude validation | API key + E2E test | Medium | P3 |
| Ollama local validation | OpenAI-compatible, same path as LM Studio | Low | P3 |
| Direct OpenAI validation | OpenAI-compatible adapter tested via Z.AI | Low | P3 |

**Assessment:** Not a v0.4.0 priority. OpenWand is not a provider-compatibility project. Batch when credentials are available.

### Category D: Security Hardening (v0.4.0 or later)

| Item | Description | Complexity | Priority |
|------|-------------|------------|----------|
| Trace verifier implementation | Runtime append-only enforcement (DEFERRED-004) | Medium | P2 |
| Fuzz testing | cargo-fuzz on sandbox, policy, memory | Medium | P2 |
| Supply chain integrity | sigstore, SLSA provenance | Medium | P3 |

**Assessment:** Trace verifier is the most architecturally relevant. Fuzz testing adds confidence. Both are candidates for v0.4.0 if the execution depth work is deferred.

### Category E: Code Quality (v0.3.x patch line)

| Item | Description | Complexity | Priority |
|------|-------------|------------|----------|
| 43 pedantic clippy warnings | Mechanical fixes in test code | Low | P3 |
| Test-only unsafe cleanup | 2 `unsafe` blocks in `#[cfg(test)]` for env var manipulation | Low | P3 |
| openwand-content stub | Implement or remove the stub crate | Low | P3 |

**Assessment:** All cosmetic. Can be addressed in a patch wave or deferred indefinitely.

### Category F: Upstream Dependencies (ongoing monitoring)

| Item | Advisory | Status |
|------|----------|--------|
| Dioxus 0.8+ | 12 GTK3 warnings + 1 unsound | Monitor — wait for upstream |
| loro CRDT update | atomic-polyfill (1) | Monitor — wait for upstream |
| kuchikiki/selectors | fxhash, rand 0.7 (2) | Monitor — wait for upstream |

**Assessment:** 15 warnings, all upstream-blocked. No action possible until framework authors release updates. Track quarterly.

---

## v0.4.0 Milestone Proposal

**Theme:** From observation to operation.

**Definition:** v0.4.0 makes the workflow lifecycle actionable through the desktop UI. The operator should be able to initiate workflow runs, resolve approvals, and export evidence — all from the desktop interface — while the UI maintains its strict read-only authority boundary (all operations delegate to existing CLI/session/policy gates).

### Proposed v0.4.0 Blockers

| ID | Name | Description | Environment-Gated? |
|----|------|-------------|-------------------|
| VD-1 | Live workflow execution depth | Desktop can initiate workflow runs, resolve approvals, and export evidence through delegated authority | No |
| VD-2 | Linux GUI runtime validation | Desktop launches and renders on a native Linux display | Yes (Linux display) |
| VD-3 | Trace verifier implementation | Runtime append-only trace enforcement | No |

**Note:** VD-1 is the primary blocker. VD-2 is environment-gated. VD-3 is security hardening that may slip to v0.5.0 if VD-1 scope expands.

### Authority Boundary for v0.4.0

```text
The desktop UI may REQUEST operations through existing authority gates.
It may NOT:
  - Import backend crates directly
  - Execute tools
  - Approve/reject without policy gate
  - Append trace
  - Write memory
  - Create workflow records without CLI delegation
  - Bypass sandbox
  - Mutate session state
```

### Candidate Wave Sequence

| Wave | Description | Depends On |
|------|-------------|------------|
| 87A | Post-v0.3 roadmap reset (this wave) | — |
| 88A | Workflow run initiation from desktop (delegated to CLI) | 87A |
| 88B | Approval resolution from desktop (delegated to session bridge) | 88A |
| 88C | Evidence export from desktop (delegated to CLI) | 88B |
| 89A | Real-time inspector refresh | 88C |
| 89B | Trace verifier implementation | 87A |
| 90A | Linux GUI runtime validation (if environment available) | 88C |
| 90B | v0.4.0 release preparation | 89A + 90A |
| 90C | v0.4.0 declaration | 90B |

**Note:** 90A requires a Linux display environment. VD-2 may be deferred if no native Linux desktop is available.

---

## v0.3.x Patch Line (Optional)

If urgent fixes are needed before v0.4.0:

| Version | Content |
|---------|---------|
| v0.3.1 | Clippy pedantic cleanup (if desired) |
| v0.3.2 | Additional provider validation (if credentials available) |

v0.3.x patches are optional. The v0.3.0 stable caveats are sufficient unless a real defect is discovered.

---

*This roadmap defines v0.4.0 priorities. It does not commit to specific wave content or ordering. Actual implementation depends on environment access (Linux display, provider credentials), emerging priorities, and external feedback on v0.3.0. It adds no feature behavior, no new authority, no policy change, no prompt change, and no unsupported production-readiness claim.*
