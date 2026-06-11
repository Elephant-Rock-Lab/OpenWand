# OpenWand

> Conjure results from intent.

OpenWand is a local-first, privacy-first AI agent desktop tool built entirely in Rust. Not a framework. Not an OS. **Your personal agent** — fast, small, extensible, governed, and yours.

## Architecture

14 crates, one binary, zero compromise.

```
openwand (binary)
├── openwand-core       Agent loop, planning, recovery, mode
├── openwand-trace      Authority, append, query, stream, store
├── openwand-store      Persistence layer
├── openwand-session    Session runner, config, testing harness
├── openwand-memory     3-tier memory (user/session/agent + ACE Skillbook)
├── openwand-tools      Built-in tools (filesystem, shell, web, browser, spawn)
├── openwand-mcp-pool   MCP server pool via rmcp (official Rust SDK)
├── openwand-policy     Governance: budget governor, redaction, access control
├── openwand-llm        Multi-provider LLM routing with model cascade
├── openwand-skills     YAML + Markdown skill store with auto-discovery
├── openwand-goals      Fitness functions + autonomous improvement loops
├── openwand-content    Rich content: syntect, mermaid-rs-renderer, comrak
├── openwand-workflow   Evidence ladder: 44 modules, 6 dependencies, leaf crate
└── openwand-app        CLI binary, lib, UI, persistence
```

## Quick Start

```bash
# Build
cargo build --workspace

# Run
cargo run -p openwand-app

# Test everything
cargo test --workspace --features "openwand-session/testing,openwand-session/sqlite-testing,openwand-memory/testing,openwand-memory/sqlite-testing"
```

## Stack

| Layer | Technology |
|-------|-----------|
| UI | Dioxus 0.7 (desktop) |
| Rich Text | taino-edit-dioxus |
| Sessions | Loro CRDT |
| MCP | rmcp (official Rust SDK) |
| Diagrams | mermaid-rs-renderer (mmdr) |
| Syntax | syntect |
| Memory | rusqlite |
| Trust | blake3 + ring |
| Runtime | tokio |

## Principles

1. **Local-first** — all data in `~/.openwand/`, zero cloud dependencies
2. **Fast** — compact binary, fast startup
3. **Safe** — every tool call passes through policy engine before execution
4. **Observable** — every action produces a structured, typed result
5. **Self-improving** — agents learn from mistakes via ACE Skillbook
6. **Extensible** — MCP servers add capabilities without touching core
7. **Governed** — workflow evidence ladder with hash-bound append-only records
8. **Disk-verified** — every development wave starts from repository truth

## Workflow Evidence Ladder

OpenWand's workflow system builds an append-only evidence chain (structural hash-chaining;
immutability enforcement deferred to verifier, not yet implemented):

```
skills/goals → task plan → review → proposal → review → readiness
→ execution → routing → bridge → outcome → reconciliation
→ run revision → continuation → next-action review → routing readiness
→ routing gate → loop controller → command composer → command review
→ manual result capture
```

Each step is content-addressed (`blake3`), hash-bound to its predecessor, and guarded against
unauthorized mutation. No runtime verifier enforces append-only at the store level yet.
The workflow crate is a leaf dependency with exactly 6 crates: `serde`, `serde_json`, `blake3`,
`chrono`, `thiserror`, `tracing`.

## Development Doctrine

From Wave 38 onward, OpenWand follows **Disk-Verified Large-Wave Execution**:

```
Scale by wave.
Ground by disk.
Accept by tests.
Seal by evidence.
```

No wave plan is valid until the relevant repository state has been inspected. No wave is accepted without passing tests and evidence of no forbidden mutations.

See [GOVERNANCE.md](GOVERNANCE.md), [ROADMAP.md](ROADMAP.md), and [WAVES.md](WAVES.md).

## Test Baseline

**2824 tests, zero failures** (Wave 38 locked)

## License

MIT
