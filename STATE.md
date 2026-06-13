# OpenWand — Project State

## Version
0.1.0-beta

## Status
**Wave 81A complete. Code quality refreshed, clippy reduced from 390→0 actionable production warnings. All v0.2.0 blockers resolved.**

Release: v0.1.0-beta — `b29898b` — `v0.1.0-beta`

Next: Wave 81B — v0.2.0-beta Declaration.
Binary: 18,030,080 bytes (17.2 MB), SHA-256 `641F1E7B7AF0D1A40E63D767738B6B8F06AC95C2B5641E5CD21A030E16B2CB9C`

Not stable. Not production-ready. Accepted residuals documented.

## Workspace Structure
```
crates/
├── core/       Domain IDs, vocabulary, events, snapshots           (lib)
├── trace/      Generic trace substrate (TraceStore<E>)              (lib)
├── store/      Trace+Memory persistence, StoredEvent bridge         (lib)
├── session/    Loro CRDT session + SessionRunner                    (lib)
├── memory/     3-tier memory + ACE Skillbook                        (lib)
├── tools/      ToolExecutor + local tools + composite seam          (lib)
├── mcp-pool/   MCP server pool via rmcp + MockGateway               (lib)
├── policy/     Deterministic trust gate, BuiltinPolicyEngine        (lib)
├── llm/        Provider-normalized LLM boundary, SSE adapter        (lib)
├── skills/     YAML + Markdown skill store                          (lib)
├── goals/      Fitness functions + improvement                      (lib)
├── workflow/   Evidence ladder: 24 capabilities, leaf crate         (lib)
└── app/        CLI binary + desktop UI + evaluation + coordination  (bin)
```

Note: `openwand-content` removed from workspace at Wave 50A (scaffold since Wave 0,
zero implementation). Will be re-added when syntect/mermaid/comrak rendering is needed.

## Test Count

**Canonical verification commands:**
```bash
cargo check --workspace --all-targets --all-features
cargo build --workspace --all-targets --all-features
cargo test -p openwand-core --lib
cargo test -p openwand-session --lib --features testing
cargo test -p openwand-tools --lib
cargo test -p openwand-app --lib
cargo clippy -p openwand-core -p openwand-session -p openwand-tools \
  -p openwand-trace -p openwand-store -p openwand-memory \
  -p openwand-llm -p openwand-policy -p openwand-skills \
  -p openwand-goals -p openwand-workflow --all-features -- -D warnings
cargo audit
```

**Post-beta baseline (Wave 77D):** 2,271 lib + 161 integration tests, 0 failures.
- openwand-core: 45
- openwand-session: 49 + 122 integration
- openwand-tools: 111
- openwand-app: 957 + 39 integration

**Not yet clean:** `openwand-app` non-test-path warnings (~25) and test-module clippy
warnings remain accepted as cosmetic.

## Post-Alpha Stabilization Arc

| Wave | Title | Tag | Deliverable |
|------|-------|-----|-------------|
| 76A | Post-Alpha Issue Intake | `wave-76a-lock` | 5 issue templates + triage guide |
| 76B | Windows TOCTOU Feasibility | `wave-76b-lock` | NT API feasibility document |
| 76C | Multi-Provider Matrix | `wave-76c-lock` | 2 local models validated (4/4 PASS each) |
| 76D | Desktop Interaction E2E | `wave-76d-lock` | 6 desktop interaction tests |

## Beta Gap Summary

**Beta-blocking items (1 of 10 unresolved):**

| # | Criterion | Status |
|---|-----------|--------|
| BC-1 | No unresolved release blockers | ✅ 6/6 resolved |
| BC-2 | At least one hosted provider validated | ✅ Z.AI glm-4.5-air + glm-5.1 |
| BC-3 | Desktop UI interaction path validated | ✅ Windows UI Automation (77C) |
| BC-6 | Documentation current through 77C | ✅ This wave |
| BC-7 | Beta release notes written | ⬌ Beta release wave |
| BC-8 | Windows TOCTOU path revisited | ✅ Documented (76B) |
| BC-9 | Multi-provider matrix expanded | ✅ 2 local + 2 hosted models |
| BC-10 | Non-Windows platform testing | ⬜ Deferred |

**Beta path:** 77A (docs) → 77B (hosted provider ✅) → 77C (desktop UX ✅) → 77D (beta tag)

See `docs/BETA_GAP_LEDGER.md` for full gap analysis.

## Hard Boundaries (Global)
- HB-G1: Binary < 20MB
- HB-G2: Zero telemetry, zero cloud storage dependencies
- HB-G3: All data in `~/.openwand/`
- HB-G4: Zero `unsafe` in OpenWand production code (test-only env var manipulation
  excepted; dependencies may use it; Unix libc openat封装在 WorkspaceWriteHandle)
- HB-G5: `cargo clippy` zero warnings on 11 non-app production crates.
  `openwand-app` test-module style warnings accepted as cosmetic.
