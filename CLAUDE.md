# CLAUDE.md — Working with OpenWand

## Project Overview
OpenWand is a standalone Rust AI agent desktop tool. 11 crates in a Cargo workspace. Dioxus UI. Loro CRDT sessions. rmcp for MCP.

## Key Commands
```bash
cargo build --workspace          # Build everything
cargo test --workspace           # Test everything
cargo clippy --workspace         # Lint (must pass with zero warnings)
cargo run -p openwand-app        # Run the desktop app
```

## Architecture Rules
- Crates may only depend on crates listed *below* them in the dependency graph
- Dependency order: `session` → `memory` → `policy` → `llm` → `tools` → `mcp-pool` → `skills` → `goals` → `content` → `core` → `app`
- `app` is the only binary. Everything else is a library.
- Never use `unsafe` in OpenWand code.

## Naming Conventions
- Crate names: `openwand-<name>` (e.g., `openwand-core`, `openwand-session`)
- Binary: `openwand`
- Types: PascalCase (e.g., `AgentLoop`, `SessionStore`, `ActionResult`)
- Functions: snake_case (e.g., `dispatch_action`, `compute_fingerprint`)
- Files: snake_case (e.g., `loop_detector.rs`, `tiered_action.rs`)

## Testing
- Every public function gets a unit test
- Integration tests go in `tests/` at crate root
- Use `#[tokio::test]` for async tests
- Target: >80% coverage

## Commit Messages
Format: `crate: brief description`

Examples:
- `session: add Loro CRDT document per session`
- `core: implement 3-tier action cascade`
- `app: wire chat UI to agent loop`
