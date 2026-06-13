# OpenWand — Project State

## Version
0.2.0 (stable) — planning v0.3.0

## Status
**v0.2.0 stable. Post-v0.2 roadmap reset complete. v0.3.0 theme: live workflow wiring.**

Release: v0.2.0 — `ca325e2` — `v0.2.0`

Next: Wave 84A — Live workflow wiring.
Binary: 17,847,296 bytes (~17.0 MB), SHA-256 `D5DDECF63E9EEE92B36CB12EFB4A80CDA6FE4E7B1A88CC335A06503386C602DC`

Stable for v0.2.0 milestone scope. Not production-ready. Not formal security review.

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

Note: `openwand-content` is a stub crate (add() only). Will be implemented when rich rendering is needed.

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

**v0.2.0 stable baseline (Wave 82D):** 2,279 lib + 1,638 integration tests, 0 failures.
- openwand-core: 45
- openwand-session: 49 + integration
- openwand-tools: 111
- openwand-app: 970 + integration
- openwand-workflow: 728
- openwand-memory: 223
- openwand-trace: 41
- Total workspace: 3,917 tests, 0 failures

**Clippy posture:** 0 actionable production warnings. 43 pedantic/test-only warnings accepted.

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
