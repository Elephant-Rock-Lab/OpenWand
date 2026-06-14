# OpenWand v0.8.0 Roadmap

**Theme:** Operational hardening and platform closure.

**Status:** Planning (Wave 108A)

---

## Context

v0.7.0 completed the external assurance arc: externally persisted checkpoint
anchors, automated scan evidence, structured authority review, and reviewer-facing
evidence report export. The architecture arc reached:

```
Control → Observe → Operate → Verify → Harden → Externally Anchor
```

Two long-standing deferrals remain: Linux GUI runtime validation (VG-3, now VH-1)
and provider validation expansion (VG-4, now VH-2). v0.8.0 addresses operational
hardening and platform closure — making the runtime and reviewer handoff credible.

---

## Architecture Arc

```
v0.2  governed execution substrate          Control
v0.3  live observation                      Observe
v0.4  desktop operation requests            Operate
v0.5  read-only verification                Verify
v0.6  evidence-backed assurance hardening   Harden
v0.7  external assurance                    Externally Anchor
v0.8  operational hardening and platform closure
```

---

## Blockers (VH series)

| Blocker | Description | Priority |
|---------|-------------|----------|
| VH-1 | Linux GUI runtime validation | P1 (core) |
| VH-2 | Provider validation expansion | P2 (strategic) |
| VH-3 | External review packet | P1 (core) |
| VH-4 | Evidence report UX integration | P2 |
| VH-5 | Release/process hardening | P2 |

---

### VH-1: Linux GUI Runtime Validation

**Problem:** OpenWand compiles on Linux (WSL2 Ubuntu, Rust 1.96.0, webkit2gtk-4.1)
but the GUI has never been runtime-validated. No display server is available in
the current development environment.

**Goal:** Validate the desktop GUI launches and renders on Linux.

**Approach options:**

| Option | Description | Feasibility |
|--------|-------------|-------------|
| A: WSLg | Use WSLg (WSL2 GUI support) for display smoke test | Medium — requires WSLg setup |
| B: Xvfb | Use Xvfb (virtual framebuffer) for headless render test | Low — Dioxus/wry may need real GPU |
| C: CI pipeline | Add Linux CI job with Xvfb or container display | Medium — requires CI infrastructure |
| D: Continue defer | Compile-validation remains the gate | Current state |

**Recommended scope:** Try Option A or B. If neither works, continue to defer
with honest documentation.

**If unresolvable:** Continue to defer. Compile-validation remains the gate.
This does not block the core assurance claims.

---

### VH-2: Provider Validation Expansion

**Problem:** Provider validation is limited to 5 models across 2 provider
families (LM Studio and Z.AI). Direct OpenAI, Anthropic, and Ollama coverage
remains deferred.

**Goal:** Add at least one direct provider validation if strategically needed.

**Scope:** Depends on API key availability and strategic priority. Marked P2.

---

### VH-3: External Review Packet

**Problem:** v0.7.0's evidence report export (106A) generates a JSON file, but
a reviewer needs more than just JSON. They need a bundle: report + authority
review + scan results + release notes + caveats.

**Goal:** Package all assurance evidence into a single reviewer-ready bundle.

**Candidate scope:**
- `openwand review-packet --session <id> --operations <ops.json> [--anchor <file>] --output <dir>`
- Generates evidence report JSON
- Copies SECURITY_SCAN_RESULTS.md, AUTHORITY_REVIEW.md, release notes
- Includes README explaining how to interpret the bundle
- Optional: ZIP archive for distribution

**Authority boundary:** Packaging is observation. The exporter copies and
aggregates existing artifacts. It does not create new assurance facts.

---

### VH-4: Evidence Report UX Integration

**Problem:** Evidence report is CLI-only. Desktop users have no way to generate
reports from the UI.

**Goal:** Surface evidence-report generation in desktop or a guided CLI flow.

**Scope:** This may be a desktop UI surface or an improved CLI flow. Marked P2.
Depends on whether desktop integration is strategically valuable.

---

### VH-5: Release/Process Hardening

**Problem:** The release process is manual: run tests, build, record SHA,
write notes, commit, tag, push. There's no repeatable release script.

**Goal:** Create a repeatable release workflow.

**Candidate scope:**
- `scripts/release-prep.sh`: run tests, clippy, audit, build, record metrics
- `scripts/release-declare.sh`: commit state, tag, push
- Refresh cargo audit and clippy as part of every release

**Authority boundary:** Scripts run existing commands. No new authority.

---

## Proposed Wave Sequence

| Wave | Title | Blocker | Description |
|------|-------|---------|-------------|
| 108A | Post-v0.7 Roadmap Reset | — | This document; VH blockers proposed |
| 109A | Linux GUI Smoke Test | VH-1 | Attempt WSLg/Xvfb display validation |
| 110A | External Review Packet | VH-3 | Bundle evidence into reviewer-ready package |
| 111A | Release Process Hardening | VH-5 | Repeatable release workflow scripts |
| 112A | v0.8.0 Release Preparation | — | Reconcile blockers, release notes |
| 112B | v0.8.0 Declaration | — | Tag v0.8.0 |

**Note:** VH-2 (provider expansion) and VH-4 (UX integration) are strategic
and may be deferred. VH-1 is environment-gated and may not be resolvable.

---

## Deferred Items (from prior arcs)

| Item | Origin | Status |
|------|--------|--------|
| Linux GUI runtime | VE-3 / VF-4 / VG-3 / VH-1 | Environment-gated; compile-validated only |
| Direct OpenAI/Anthropic/Ollama | Provider matrix caveat | Deferred; LM Studio + Z.AI validated |
| macOS runtime | Platform caveat | No environment |
| Full signature-based anchoring | DEFERRED-004 | VH-1 external anchor done; signatures may need v0.9+ |
| Stable API guarantee | v0.5.0 caveat | Post-production-readiness |
| Production readiness | Global caveat | Not claimed for any version |

---

## Success Criteria for v0.8.0

1. **Linux GUI runtime validated OR honestly deferred** with new evidence
2. **External review packet exists** as a reviewer-ready bundle
3. **Release process is repeatable** with scripts
4. **No new write authority for verifiers**
5. **Legacy behavior preserved** — no retroactive failures

---

## What v0.8.0 is NOT

- Not production-ready
- Not a formal external security audit
- Not full cryptographic non-repudiation
- Not cross-platform runtime validation (unless VH-1 succeeds)
- Not stable API guarantee
