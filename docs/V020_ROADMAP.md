# v0.2.0 Roadmap and Architecture Reset

**Date:** 2026-06-13
**Baseline:** v0.1.0-beta (`b29898b`)
**Scope:** Defines v0.2.0 themes, blockers, deferred items, and candidate waves

---

## v0.2.0 Themes

| Theme | Priority | Description |
|-------|:--------:|-------------|
| Security hardening | **P0** | Close Windows TOCTOU micro-race via NtCreateFile |
| Provider expansion | **P1** | Validate Anthropic adapter, add Ollama, expand hosted matrix |
| Desktop productization | **P1** | Complete placeholder UI surfaces, tab switching, visual validation |
| Cross-platform validation | **P2** | Linux/macOS build and test |
| Dependency posture | **P2** | Evaluate Dioxus/Loro updates, reduce transitive warnings |
| Code quality | **P3** | Resolve app clippy warnings, clean dead eval code |

---

## v0.2.0 Blockers (Must Resolve)

| # | Blocker | From | Resolution Path | Estimated Waves |
|---|---------|------|-----------------|:---------------:|
| VB-1 | ~~Windows TOCTOU micro-race closure~~ ✅ Closed 78C | DEFERRED-008 | `NtCreateFile` + `RootDirectory` + `FILE_OPEN_REPARSE_POINT` per component | 1 |
| VB-2 | ~~Anthropic adapter validation~~ → Post-v0.2 | DEFERRED-009 partial | Enable `anthropic-compatible` feature, validate against hosted API | Post-v0.2 |
| VB-3 | Placeholder UI surface completion | Known gaps | Implement 6 stub + 4 minimal surfaces | 2–3 |

---

## v0.2.0 Recommended (Should Resolve)

| # | Item | Resolution Path | Estimated Waves |
|---|------|-----------------|:---------------:|
| VR-1 | Tab switching validation | Windows UI Automation or manual test | ≤1 |
| VR-2 | OpenAI direct API validation | Configure API key, run existing test suite | ≤1 |
| VR-3 | Ollama local validation | Start Ollama, run existing test suite | ≤1 |
| VR-4 | Dependency audit refresh | `cargo audit`, evaluate Dioxus/Loro updates | ≤1 |
| VR-5 | App clippy cleanup | Remove dead eval code, fix warnings | 1 |

---

## v0.2.0 Deferred (Post-v0.2.0)

| Item | Reason |
|------|--------|
| Non-Windows platform binary build | Requires cross-compilation or CI |
| Visual/CSS styling validation | Needs vision model or manual review |
| Dioxus headless testing framework | Upstream dependency |
| Trace immutability verifier | Feature work, not security |
| openwand-content crate re-addition | Feature work, rendering needed |
| Concurrent session validation | Architecture supports it, not tested |
| Real LLM response rendering in UI | Requires provider + UI interaction |

---

## Placeholder UI Surface Inventory

### Stub Surfaces (3-line placeholders - 6 total)

| # | Surface | Workflow Capability | Complexity | Priority |
|---|---------|---------------------|:----------:|:--------:|
| 1 | `workflow_action_outcome_components` | Action outcome review | Medium | P1 |
| 2 | `workflow_command_composer_components` | Command composition | High | P1 |
| 3 | `workflow_command_review_components` | Command review gate | Medium | P1 |
| 4 | `workflow_continuation_components` | Continuation readiness | Medium | P1 |
| 5 | `workflow_loop_controller_components` | Loop control dashboard | High | P1 |
| 6 | `workflow_reconciliation_components` | Reconciliation gate | Medium | P1 |

### Minimal Surfaces (6-16 lines - 4 total)

| # | Surface | Lines | Priority |
|---|---------|------:|:--------:|
| 7 | `workflow_proposal_components` | 9 | P1 |
| 8 | `workflow_readiness_components` | 6 | P1 |
| 9 | `workflow_external_attestation_components` | 16 | P2 |
| 10 | `workflow_verification_readiness_components` | 16 | P2 |

### Completed Surfaces (reference)

8 surfaces are fully implemented (100+ lines each):
- `workflow_operator_console_components` (601 lines)
- `workflow_execution_components` (651 lines)
- `workflow_manual_result_components` (595 lines)
- `workflow_evidence_chain_inspector_components` (465 lines)
- `workflow_audit_packet_distribution_components` (327 lines)
- `workflow_routing_readiness_components` (298 lines)
- `workflow_manual_result_reconciliation_readiness_components` (295 lines)
- `workflow_next_action_routing_components` (284 lines)

---

## Provider Expansion Roadmap

### Current State

| Provider | Adapter | Feature Flag | Validated? |
|----------|---------|:------------:|:----------:|
| OpenAI-compatible (generic) | `openai_compatible.rs` | `openai-compatible` | ✅ LM Studio, Z.AI |
| Anthropic | `anthropic_compatible.rs` | `anthropic-compatible` | ⬜ Not validated |

### Target State

| Provider | Action | Wave Candidate |
|----------|--------|---------------|
| Anthropic (claude-sonnet-4) | Enable feature, validate hosted | Post-v0.2 |
| OpenAI direct (gpt-4o-mini) | Configure key, run existing suite | Post-v0.2 |
| Ollama (local) | Start server, run existing suite | Post-v0.2 |

### Provider Matrix — Current State (v0.2.0)

| Provider | Model | Type | Status |
|----------|-------|------|--------|
| LM Studio | gemma-4-12b, qwen2.5-0.5b | Local | ✅ Validated |
| Z.AI | glm-4.5-air, glm-5.1, glm-5-turbo | Hosted | ✅ Validated |
| OpenAI API | gpt-4o-mini | Hosted | Post-v0.2 |
| Anthropic | claude-sonnet-4 | Hosted | Post-v0.2 |
| Ollama | (various) | Local | Post-v0.2 |

**Provider validation closed for v0.2.0.** OpenAI-compatible adapter proven across
2 provider families, 5 models, local + hosted endpoints. Further provider expansion
is post-v0.2 compatibility hardening, not a release blocker.

---

## Security Roadmap

### Windows TOCTOU Closure (VB-1)

**Current state:** Per-component `symlink_metadata()` + re-verify (73C).
Micro-race window remains between check and I/O.

**Target state:** `NtCreateFile` with `RootDirectory` + `FILE_OPEN_REPARSE_POINT`.

**Implementation plan** (from `docs/WINDOWS_TOCTOU_FEASIBILITY.md`):

| Phase | Description |
|-------|-------------|
| 1 | Add `windows-sys` feature flag for `Wdk_Storage_FileSystem` to `openwand-tools` |
| 2 | Implement `windows_create_and_write_ntapi()` using `NtCreateFile` with `RootDirectory` |
| 3 | Handle directory creation via `NtCreateFile` with `FILE_CREATE` disposition |
| 4 | Add reparse point detection on opened handles |
| 5 | Fallback to 73C behavior if `NtCreateFile` unavailable |
| 6 | Add adversarial symlink race test (Windows) |
| 7 | Update DEFERRED-008 status to closed |

**Estimated:** 1 wave (78B or 79D).

---

## Architecture Assessment

### Crate Structure (14 crates)

```
core → trace → store → session → app
                    ↗         ↗
     memory → llm → session
     tools → session
     mcp-pool → tools
     policy → session, skills, goals
     skills → goals
     goals → app
     workflow → app (leaf crate, no downstream deps)
```

### Dependency Highlights

- `app` depends on all other crates (integration point)
- `workflow` is a leaf crate (no crate depends on it)
- `session` is the central hub (depends on core, llm, memory, policy, tools, trace)
- `tools` depends on `mcp-pool` (MCP server pool)
- `skills` and `goals` depend on `policy` (trust boundary)

### Architectural Concerns

| Concern | Severity | Resolution |
|---------|:--------:|------------|
| `app` depends on `session` with `testing` feature in production | Medium | Gate behind `#[cfg(test)]` or separate dev-dep |
| Anthropic adapter exists but is feature-gated and untested | Low | Enable and validate in v0.2.0 |
| 739-line adapter file (anthropic_compatible.rs) | Low | May need modularization |
| `workflow` has no downstream consumers | Low | By design - leaf crate |

---

## Candidate Wave Sequence (v0.2.0)

### Phase 1: Security (78A-78B)

| Wave | Title | Scope |
|------|-------|-------|
| 78A | v0.2.0 Roadmap and Architecture Reset | This document |
| 78B | Windows NtCreateFile TOCTOU Closure | ~~Implement~~ `NtCreateFile` + `RootDirectory`, close VB-1 |
| 78C | Windows NtCreateFile Implementation | ✅ Closed VB-1. Hybrid NtCreateFile dirs + CreateFileW file. ~430 lines |

### Phase 2: Provider Scope Closure (79A–79B)

| Wave | Title | Scope |
|------|-------|-------|
| 79A | Hosted OpenAI-Compatible API Validation | Validate glm-5-turbo via Z.AI, expand matrix |
| 79B | Provider Scope Closure and Product Surface Pivot | Close provider arc, demote VB-2, pivot to surfaces |

### Phase 3: Desktop Productization (80A–80C)

| Wave | Title | Scope |
|------|-------|-------|
| 80A | Placeholder Surfaces I (proposal, readiness, reconciliation) | Implement 3 medium-complexity surfaces |
| 80B | Placeholder Surfaces II (action outcome, continuation, loop controller) | Implement 3 surfaces |
| 80C | Placeholder Surfaces III (command composer, command review) | Implement 2 high-complexity surfaces |

### Phase 4: Quality and Validation (81A-81B)

| Wave | Title | Scope |
|------|-------|-------|
| 81A | Code Quality and Dependency Refresh | App clippy cleanup, cargo audit refresh |
| 81B | v0.2.0-beta Declaration | Release notes, artifact identity, tag |

**Estimated total:** 8 waves (78A–81B).

---

## v0.2.0 Entry Criteria

| # | Criterion | Status |
|---|-----------|--------|
| VB-1 | ~~Windows TOCTOU micro-race closed~~ ✅ Closed 78C | ✅ Done |
| ~~VB-2~~ | ~~Anthropic adapter validated~~ → Post-v0.2 | ✅ Demoted |
| VB-3 | All placeholder UI surfaces implemented | ✅ Closed 80C |

**v0.2.0 release requires VB-3 resolved. VB-2 (provider expansion) demoted to post-v0.2
compatibility hardening — OpenAI-compatible path proven across local + hosted providers.**

---

## Metrics Comparison

| Metric | v0.1.0-alpha | v0.1.0-beta | v0.2.0 Target |
|--------|:-----------:|:-----------:|:------------:|
| Test count | 2,266 + 22 | 2,271 + 161 | 2,400+ |
| Provider families validated | 1 (local) | 2 (local + hosted) | 2 (sufficient) |
| Models validated | 1 | 4 | 5 |
| Placeholder surfaces | 10 | 10 | 0 |
| Platforms tested | 1 (Windows) | 1 (Windows) | 1-3 |
| Windows TOCTOU status | Reduced residual | Reduced residual | Fully closed |
| Anthropic support | Adapter exists | Adapter exists | Adapter exists (unvalidated) |
| Desktop UX validation | Process lifecycle | UI Automation | UI Automation |

---

*This roadmap defines v0.2.0 priorities and candidate waves. It does not commit to
specific wave content or ordering. Actual implementation may differ based on
available credentials, platform access, and emerging priorities. It adds no feature
behavior, no new authority, no policy change, no prompt change, and no stable-release
claim.*
