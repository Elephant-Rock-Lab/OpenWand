# OpenWand

> Conjure results from intent.

OpenWand is a local-first, privacy-first AI agent desktop tool built entirely in Rust. Not a framework. Not an OS. **Your personal agent** — fast, small, extensible, and yours.

## Architecture

11 crates, one binary, zero compromise.

```
openwand (binary)
├── openwand-core       Agent loop, planning, recovery
├── openwand-session    Loro CRDT session store (branch, merge, time-travel)
├── openwand-memory     3-tier memory (user/session/agent + ACE Skillbook)
├── openwand-tools      Built-in tools (filesystem, shell, web, browser, spawn)
├── openwand-mcp-pool   MCP server pool via rmcp (official Rust SDK)
├── openwand-policy     Governance: budget governor, redaction, access control
├── openwand-llm        Multi-provider LLM routing with model cascade
├── openwand-skills     YAML + Markdown skill store with auto-discovery
├── openwand-goals      Fitness functions + autonomous improvement loops
└── openwand-content    Rich content: syntect, mermaid-rs-renderer, comrak
```

## Quick Start

```bash
# Build
cargo build --workspace

# Run
cargo run -p openwand-app

# Test everything
cargo test --workspace
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
2. **Fast** — <15MB binary, <3s to first response
3. **Safe** — every tool call passes through policy engine before execution
4. **Observable** — every action produces a structured, typed result
5. **Self-improving** — agents learn from mistakes via ACE Skillbook
6. **Extensible** — MCP servers add capabilities without touching core

## License

MIT
