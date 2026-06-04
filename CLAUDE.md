# CLAUDE.md — Working with OpenWand

## Project Overview
OpenWand is a standalone Rust AI agent desktop tool. 14 crates in a Cargo workspace. Dioxus UI. Loro CRDT sessions. rmcp for MCP. Governed workflow evidence ladder.

## Architecture

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
├── openwand-content    Rich content: syntect, mermaid-rs-renderer, comarak
├── openwand-workflow   Evidence ladder (leaf crate: 6 deps only)
└── openwand-app        CLI binary, lib, UI, persistence
```

## Key Commands
```bash
cargo build --workspace          # Build everything
cargo test --workspace --features "openwand-session/testing,openwand-session/sqlite-testing,openwand-memory/testing,openwand-memory/sqlite-testing"  # Full test suite
cargo run -p openwand-app        # Run the desktop app
```

## Architecture Rules
- Crates may only depend on crates listed *below* them in the dependency graph
- `openwand-workflow` is a **leaf crate** with exactly 6 dependencies: `serde`, `serde_json`, `blake3`, `chrono`, `thiserror`, `tracing`. No imports from `openwand-core`, `openwand-session`, `openwand-tools`, `openwand-policy`, `openwand-memory`, or `openwand-trace`.
- `app` is the only binary. Everything else is a library.
- Never use `unsafe` in OpenWand code.

## Workflow Evidence Ladder

The workflow crate builds an immutable evidence chain:

```
skills/goals → task plan → review → proposal → review → readiness
→ execution → routing → bridge → outcome → reconciliation
→ run revision → continuation → next-action review → routing readiness
→ routing gate → loop controller → command composer → command review
→ manual result capture
```

Each step is content-addressed (blake3), hash-bound to its predecessor, and guarded against unauthorized mutation. See [WAVES.md](WAVES.md) for the full wave index and ID prefix registry.

## Development Process

OpenWand follows **Disk-Verified Large-Wave Execution**:

```
Scale by wave.
Ground by disk.
Accept by tests.
Seal by evidence.
```

See [GOVERNANCE.md](GOVERNANCE.md) for the full operating doctrine.

- Every wave starts with disk reconnaissance — inspect files before planning
- No plan from memory alone; code on disk is authoritative
- Every wave ends with passing tests and a lock document
- See [ROADMAP.md](ROADMAP.md) for the forward path

## Naming Conventions
- Crate names: `openwand-<name>` (e.g., `openwand-core`, `openwand-session`)
- Binary: `openwand`
- Types: PascalCase (e.g., `AgentLoop`, `SessionStore`, `ActionResult`)
- Functions: snake_case (e.g., `dispatch_action`, `compute_fingerprint`)
- Files: snake_case (e.g., `loop_detector.rs`, `tiered_action.rs`)
- ID prefixes: see [WAVES.md](WAVES.md) for the full registry (e.g., `tpl_`, `wfx_`, `wmr_`)

## Testing
- Every public function gets a unit test
- Integration tests go in `tests/` at crate root
- Use `#[tokio::test]` for async tests
- Guard tests prove no forbidden authority (17 guard test files)
- Current baseline: 2824 tests, zero failures

## Commit Messages
Format: `crate: brief description` or `docs: brief description`

Examples:
- `session: add Loro CRDT document per session`
- `core: implement 3-tier action cascade`
- `docs: Wave 37 lock document`
- `docs: prepare GitHub publishing baseline`
