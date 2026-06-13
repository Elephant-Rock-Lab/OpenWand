# v0.3.0 Roadmap — Post-v0.2 Reset

**Created:** 2026-06-13 (Wave 83A)
**Status:** Planning
**Predecessor:** v0.2.0 stable (`ca325e2`)

---

## v0.2.0 Stable State (Baseline)

| Metric | Value |
|--------|-------|
| Tests | 3,917 (2,279 lib + 1,638 integration) |
| Failures | 0 |
| Crates | 14 |
| Binary size | 17,847,296 bytes (~17.0 MB) |
| Dependency vulnerabilities | 0 |
| Dependency warnings | 15 (all transitive) |
| Workflow UI surfaces | 10/10 complete |
| Providers validated | 5 models, 2 families |
| Platforms validated | Windows only |

---

## Backlog Triage

### Category 1: Provider Expansion (v0.3.0 candidates)

| Item | Source | Complexity | Priority |
|------|--------|------------|----------|
| Anthropic Claude validation | VB-2 demoted (79B) | Medium — adapter exists, needs API key + E2E test | P2 |
| Ollama local validation | VB-2 demoted (79B) | Low — OpenAI-compatible, same path as LM Studio | P2 |
| Direct OpenAI validation | VB-2 demoted (79B) | Low — OpenAI-compatible adapter tested via Z.AI | P3 |
| Provider auto-detection | New | Medium — detect running providers at startup | P3 |

**Assessment:** OpenWand is not a provider-compatibility project. These are confidence-builders, not architecture work. Batch as a single validation wave when credentials are available.

### Category 2: Platform Expansion (v0.3.0 candidates)

| Item | Source | Complexity | Priority |
|------|--------|------------|----------|
| Linux desktop testing | BC-10 deferred (77A) | Medium — GTK3/webkit2gtk available, needs Linux env | P2 |
| macOS desktop testing | BC-10 deferred (77A) | Medium — Cocoa/WebKit available, needs macOS env | P3 |
| Non-Windows filesystem sandbox | New | Low — Unix `openat` already implemented (73B), just needs testing | P2 |

**Assessment:** Unix sandbox is already implemented. Desktop UI on Linux should work since GTK3 bindings are compiled. Needs someone to run the binary on Linux/macOS.

### Category 3: Upstream Dependencies (v0.2.x patches)

| Item | Advisory | Complexity | Priority |
|------|----------|------------|----------|
| Dioxus 0.8+ when released | 12 GTK3 warnings + 1 unsound | None (wait for upstream) | Monitor |
| loro CRDT update | atomic-polyfill (1) | None (wait for upstream) | Monitor |
| kuchikiki/selectors update | fxhash, rand 0.7 (2) | None (wait for upstream) | Monitor |
| image crate update | paste (1) | None (wait for upstream) | Monitor |
| proc-macro-error → proc-macro2 | proc-macro-error (1) | None (wait for upstream) | Monitor |

**Assessment:** 15 warnings, all upstream-blocked. No action possible until framework authors release updates. Track quarterly.

### Category 4: Code Quality (v0.2.x patches)

| Item | Source | Complexity | Priority |
|------|--------|------------|----------|
| 43 pedantic clippy warnings | 81A accepted | Low — mechanical fixes in test code | P3 |
| README.md test baseline stale | Found in 83A | Trivial | P0 (fix now) |

### Category 5: Product UX Polish (v0.3.0 candidates)

| Item | Source | Complexity | Priority |
|------|--------|------------|----------|
| Workflow UI wiring to live data | Implied by 80A-80C | High — wire render functions to actual session state | P1 |
| Desktop tab navigation polish | New | Medium — keyboard shortcuts, focus management | P2 |
| Session list UX improvements | New | Medium — search, filter, multi-session | P3 |
| Memory panel real-time updates | New | Medium — signal-based live updates | P2 |

**Assessment:** The biggest gap is that workflow UI surfaces are implemented as render functions but not yet wired to live session data. This is the most impactful v0.3.0 work.

### Category 6: Workflow Execution Depth (v0.3.0 candidates)

| Item | Source | Complexity | Priority |
|------|--------|------------|----------|
| End-to-end workflow execution | Architecture | Very high — full proposal → execution → reconciliation cycle | P1 |
| Approval workflow live path | Architecture | High — wire approval bridge to live desktop | P2 |
| Trace verifier implementation | DEFERRED-004 | Medium — runtime append-only enforcement | P2 |
| Memory auto-extraction in live sessions | Architecture | Medium — wire HeuristicExtractor to session loop | P2 |

### Category 7: Security Review Preparation (v0.3.0 or later)

| Item | Source | Complexity | Priority |
|------|--------|------------|----------|
| Formal security review | Caveat 1 | External dependency | P3 |
| Supply chain integrity | Caveat 1 | Medium — sigstore, SLSA | P3 |
| fuzz testing | New | Medium — cargo-fuzz on sandbox, policy, memory | P2 |

---

## v0.3.0 Milestone Proposal

**Theme:** From static surfaces to live workflows.

**Definition:** v0.3.0 connects the completed workflow UI surfaces to live session data, making the desktop product functional end-to-end. The operator should be able to observe a real workflow lifecycle — proposal, readiness, execution, outcome, reconciliation — through the desktop UI.

### Resolved v0.3.0 Blockers

| ID | Name | Description | Status | Wave |
|----|------|-------------|--------|------|
| VC-1 | Live workflow wiring | 5 workflow surfaces connected to live session data | ✅ RESOLVED | 84A-84C |
| VC-2 | Linux desktop validation | Desktop feature compiles on Linux (Ubuntu WSL2) with GTK/webkit2gtk | ✅ VALIDATED | 85A |
| VC-3 | Unix filesystem sandbox testing | `openat`-based sandbox tested on Linux: 3,934 tests, 0 failures | ✅ VALIDATED | 85A |

### Cross-Platform Bugs Found and Fixed (Wave 85A)

| Bug | Impact | Fix |
|-----|--------|-----|
| Missing `OsStr` import in sandbox.rs | Linux compilation failure | Added `use std::ffi::OsStr` |
| Unused `AsRawFd`/`FromRawFd` import | Linux warning | Removed |
| Windows-specific path tests running on Linux | 2 test failures | `#[cfg(windows)]` gated |
| Symlink test assertion too strict for Linux | 1 test failure | Accept both `SymlinkDetected` and `PathContainmentError` |
| Git path test with Windows paths on Linux | 1 test failure | `#[cfg(windows)]` gated second assertion |
| `chain_hash_display` import not cfg-gated | Linux warning | `#[cfg(any(feature = "desktop", test))]` gated |

### Candidate Wave Sequence

| Wave | Description | Depends On |
|------|-------------|------------|
| 83A | Post-v0.2 roadmap reset (this wave) | — |
| 83B | README/docs staleness sweep | 83A |
| 84A | Live workflow wiring I: proposal + readiness | 83A |
| 84B | Live workflow wiring II: execution + outcome | 84A |
| 84C | Live workflow wiring III: reconciliation + loop | 84B |
| 85A | Linux desktop validation (if environment available) | 84C |
| 85B | Unix sandbox E2E testing on Linux | 85A |
| 86A | Provider expansion batch (if credentials available) | 83A |
| 86B | v0.3.0 release preparation | 85A + 86A |
| 86C | v0.3.0 declaration | 86B |

**Note:** 85A/85B require a Linux environment. 86A requires provider credentials. These are environment-gated and may slip.

---

## v0.2.x Patch Line (Optional)

If urgent fixes are needed before v0.3.0:

| Version | Content |
|---------|---------|
| v0.2.1 | README staleness fix + docs consistency | 
| v0.2.2 | Clippy pedantic cleanup (if desired) |

v0.2.x patches are optional. The v0.2.0 stable caveats are sufficient unless a real defect is discovered.

---

## Preserved Caveats from v0.2.0

These caveats remain in effect for v0.3.0 unless explicitly resolved:

1. Not a formal security review
2. 43 pedantic/test-only clippy warnings (may be cleaned in v0.2.x)
3. Non-Windows platform validation deferred (VC-2 targets this)
4. Hosted provider validation indirect
5. Post-v0.2 provider expansion (may be partially addressed)
6. 15 transitive dependency warnings (upstream-blocked)
7. Windows final-component on 72B no-follow path

---

*This roadmap defines v0.3.0 priorities. It does not commit to specific wave content or ordering. Actual implementation depends on environment access (Linux, provider credentials), emerging priorities, and external feedback on v0.2.0. It adds no feature behavior, no new authority, no policy change, no prompt change, and no unsupported production-readiness claim.*
