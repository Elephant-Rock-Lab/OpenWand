# OpenWand — Project State

## Version
0.1.0-alpha

## Status
**Wave 77A complete. Beta Gap Ledger and Roadmap Reset.**

Release: v0.1.0-alpha — `967dc96` — `v0.1.0-alpha`
Post-alpha: `f05694d` (`wave-76d-lock`) → `HEAD` (77A)

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

**Post-alpha baseline (Wave 76D):** 2,272 lib + 28 integration tests, 0 failures.
- openwand-core: 45
- openwand-session: 49 + 14 integration
- openwand-tools: 111
- openwand-app: 957 + 14 integration (8 CLI surface + 6 desktop interaction)

**Not yet clean:** `openwand-app` test-module clippy warnings (57 style lints in
`#[cfg(test)]` helpers) remain accepted as cosmetic. Zero affect production code.

## Post-Alpha Stabilization Arc

| Wave | Title | Tag | Deliverable |
|------|-------|-----|-------------|
| 76A | Post-Alpha Issue Intake | `wave-76a-lock` | 5 issue templates + triage guide |
| 76B | Windows TOCTOU Feasibility | `wave-76b-lock` | NT API feasibility document |
| 76C | Multi-Provider Matrix | `wave-76c-lock` | 2 local models validated (4/4 PASS each) |
| 76D | Desktop Interaction E2E | `wave-76d-lock` | 6 desktop interaction tests |

## Beta Gap Summary

**Beta-blocking items (4 of 10 unresolved):**

| # | Criterion | Status |
|---|-----------|--------|
| BC-1 | No unresolved release blockers | ✅ 6/6 resolved |
| BC-2 | At least one hosted provider validated | ⬜ Not done |
| BC-3 | Desktop UI interaction path validated | ⬜ Service/bridge only |
| BC-6 | Documentation current through 76A–76D | ✅ This wave (77A) |
| BC-7 | Beta release notes written | ⬌ Beta release wave |
| BC-8 | Windows TOCTOU path revisited | ✅ Documented (76B) |
| BC-9 | Multi-provider matrix expanded | ✅ 2 local models |
| BC-10 | Non-Windows platform testing | ⬜ Deferred |

**Beta path:** 77A (docs) → 77B (hosted provider) → 77C (desktop UX) → 77D (beta tag)

See `docs/BETA_GAP_LEDGER.md` for full gap analysis.

## Hard Boundaries (Global)
- HB-G1: Binary < 20MB
- HB-G2: Zero telemetry, zero cloud storage dependencies
- HB-G3: All data in `~/.openwand/`
- HB-G4: Zero `unsafe` in OpenWand production code (test-only env var manipulation
  excepted; dependencies may use it; Unix libc openat封装在 WorkspaceWriteHandle)
- HB-G5: `cargo clippy` zero warnings on 11 non-app production crates.
  `openwand-app` test-module style warnings accepted as cosmetic.
