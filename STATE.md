# OpenWand — Project State

## Version
0.1.0-alpha

## Status
**Wave 76B complete. Windows TOCTOU Residual Hardening Feasibility.**

Release: v0.1.0-alpha — `967dc96` — `v0.1.0-alpha`

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

**Baseline (Wave 70A lock):** 1,146 tests, 0 failures.
- openwand-core: 45
- openwand-session: 51
- openwand-tools: 93
- openwand-app: 957

**Not yet clean:** `openwand-app` test-module clippy warnings (57 style lints in
`#[cfg(test)]` helpers) remain accepted as cosmetic. Zero affect production code.

## Wave History (Post-50A)

| Wave | Goal | Tag | Tests | Status |
|------|------|-----|------:|--------|
| 50A | Gap Remediation I | `wave-50a-lock` | ~3,416 | ✅ |
| 51A | Gap Remediation II | `wave-51a-lock` | ~3,420 | ✅ |
| 52A | Design System Foundation | `wave-52a-lock` | 642 | ✅ |
| 53A | Operator Console Desktop Surface | `wave-53a-lock` | 655 | ✅ |
| 54A | Evidence Chain Inspector Surface | `wave-54a-lock` | 668 | ✅ |
| 55A | Audit Packet Review & Distribution | `wave-55a-lock` | 686 | ✅ |
| 56A | Manual Result Ladder Surface | `wave-56a-lock` | 727 | ✅ |
| 57A | Workflow Routing & Next-Action | `wave-57a-lock` | 752 | ✅ |
| 58A | Workflow Execution Timeline | `wave-58a-lock` | 774 | ✅ |
| 59A | Desktop UI Shell Refactor | `wave-59a-lock` | 778 | ✅ |
| 60A | Desktop Session Shell Refactor | `wave-60a-lock` | 795 | ✅ |
| 61A | Desktop Bootstrap Boundary | `wave-61a-lock` | 806 | ✅ |
| 62A | Skills & Goals Readiness | `wave-62a-lock` | 836 | ✅ |
| 63A | Context Projection Wiring | `wave-63a-lock` | 888 | ✅ |
| 64A | Context Explainability & Preview | `wave-64a-lock` | 880 | ✅ |
| 65A | Context Audit Trace | `wave-65a-lock` | 955 | ✅ |
| 66A | Deterministic Eval Harness | `wave-66a-lock` | 986 | ✅ |
| 67A | Real-Model Boundary Eval | `wave-67a-lock` | 1,006 | ✅ |
| 68A | Eval Readiness & Reporting | `wave-68a-lock` | 1,032 | ✅ |
| 69A | Filesystem Sandbox | `wave-69a-lock` | 1,125 | ✅ |
| 69B | Approval Workspace Binding | `wave-69b-lock` | 1,135 | ✅ |
| 69C | Canonical Build & Desktop Compile | `wave-69c-lock` | 1,135 | ✅ |
| 69D | Truthful Verification Commands | `wave-69d-lock` | 1,141 | ✅ |
| 69E | Production Trace Attribution | `wave-69e-lock` | 1,141 | ✅ |
| 69F | Release Hardening & Residual Risk | `wave-69f-lock` | 1,141 | ✅ |
| 69G | RC Truth Ledger & Publication Baseline | `wave-69g-lock` | 1,144 | ✅ |

Full wave history (Waves 00–49A): see `WAVES.md` and `docs/WAVE*_LOCK.md`.

## Hard Boundaries (Global)
- HB-G1: Binary < 20MB
- HB-G2: Zero telemetry, zero cloud storage dependencies
- HB-G3: All data in `~/.openwand/`
- HB-G4: Zero `unsafe` in OpenWand production code (test-only env var manipulation
  excepted; dependencies may use it)
- HB-G5: `cargo clippy` zero warnings on 11 non-app production crates.
  `openwand-app` test-module style warnings accepted as cosmetic.
