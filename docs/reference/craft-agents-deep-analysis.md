# Craft Agents OSS — Deep Architecture Analysis

**Codebase:** `C:\Next-Era\CraftAgents` | **Version:** 0.9.2 | **Date:** May 25, 2026

---

## Executive Summary

Craft Agents is a **1,350+ file TypeScript monorepo** (~313 test files) that builds an agent-native desktop app. It's an ambitious, well-engineered project that combines multiple AI SDKs (Claude Agent SDK, Pi AI SDK, GitHub Copilot SDK) with an Electron UI, a headless server mode, MCP integration, and a rich rendering pipeline. The architecture shows clear evolution from a Claude-centric tool to a multi-model platform — with the scars to prove it.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│                    Electron App                      │
│  ┌──────────┐  ┌──────────┐  ┌──────────────────┐  │
│  │  Main     │  │ Preload  │  │   Renderer (React)│  │
│  │ Process   │  │ Bridge   │  │   TipTap + Radix  │  │
│  └─────┬─────┘  └────┬─────┘  └────────┬─────────┘  │
│        │              │                 │             │
│        └──────────────┼─────────────────┘             │
│                       │ WebSocket RPC                 │
├───────────────────────┼──────────────────────────────┤
│              Server (Headless)                        │
│  ┌────────────┐  ┌────────────┐  ┌───────────────┐  │
│  │ Transport  │  │  Session   │  │   RPC         │  │
│  │ (WS RPC)   │  │  Manager   │  │   Handlers    │  │
│  └─────┬──────┘  └─────┬──────┘  └───────┬───────┘  │
│        │               │                  │           │
│  ┌─────┴───────────────┴──────────────────┴───────┐  │
│  │              @craft-agent/shared                │  │
│  │  ┌─────────┐ ┌─────────┐ ┌──────┐ ┌────────┐  │  │
│  │  │ Agents  │ │ Sources │ │ MCP  │ │ Skills │  │  │
│  │  │ Claude  │ │  MCP/API│ │ Pool │ │ Store  │  │  │
│  │  │ Pi      │ │  Local  │ │      │ │        │  │  │
│  │  │ Backend │ │         │ │      │ │        │  │  │
│  │  └─────────┘ └─────────┘ └──────┘ └────────┘  │  │
│  │  ┌──────────┐ ┌───────────┐ ┌──────────────┐  │  │
│  │  │ Config   │ │Credential │ │Automations   │  │  │
│  │  │ Prefs    │ │ Manager   │ │Scheduler     │  │  │
│  │  │ Models   │ │Encryption │ │Event Bus     │  │  │
│  │  └──────────┘ └───────────┘ └──────────────┘  │  │
│  └────────────────────────────────────────────────┘  │
│  ┌────────────────────┐  ┌────────────────────────┐  │
│  │  Subprocesses      │  │  WebUI / Viewer        │  │
│  │  Pi Agent Server   │  │  Vite + React          │  │
│  │  MCP Servers       │  │  Browser access        │  │
│  │  WhatsApp Worker   │  │                        │  │
│  └────────────────────┘  └────────────────────────┘  │
└──────────────────────────────────────────────────────┘
```

---

## 1. STRENGTHS

### 🏗️ 1.1 Clean Monorepo Architecture
- **11 packages + 4 apps**, each with clear responsibility boundaries
- `@craft-agent/core` is intentionally kept as a lightweight type-only layer — excellent for stability
- `@craft-agent/shared` holds all business logic; `server-core` handles transport/RPC; `ui` holds shared components
- Workspace deps use `workspace:*` protocol — clean internal linking
- Each package has its own `tsconfig.json` with proper isolation

### 🤖 1.2 Sophisticated Multi-Backend Agent System
- **Clean abstraction**: `BaseAgent` → `ClaudeAgent` / `PiAgent` with well-defined `AgentBackend` interface
- **Subprocess isolation**: Pi SDK runs in its own process via JSONL stdin/stdout — crashes don't take down the main process
- **Runtime model switching**: Can swap models mid-session with hot-reload (in-place config vs full restart detection via `buildRestartRequiredSignature`)
- **25+ session-scoped tools** with Zod schemas, a canonical tool registry, and per-backend proxy routing
- **Permission system**: 3 modes (safe/ask/allow-all) with bash command validation and pre-tool-use pipeline

### 🔌 1.3 Impressive MCP Integration
- **MCP Pool** (`mcp-pool.ts`): Shared pool of MCP client connections across sessions
- **Three source types**: MCP, API (REST), and local — unified interface
- **Auto-discovery**: Agent can read public API docs and auto-configure new sources
- **OAuth flows**: Google, Slack, Microsoft with PKCE — handled via relay redirect URI
- **Token refresh**: Both OAuth and custom renew-endpoint support

### 🎨 1.4 Rich Rendering Pipeline
- **TipTap editor** with math (KaTeX), code highlighting (Shiki), mermaid diagrams, file handling
- **Native rendering** of: mermaid SVGs, PDF previews, image previews, HTML sandboxed iframes, data tables/spreadsheets
- **Theme system** with full customization support
- **7 languages** (en, es, de, ja, zh-Hans, hu, pl) with comprehensive i18n tooling

### 📡 1.5 Transport & Communication
- **Custom WebSocket RPC protocol** with request/response correlation, event subscriptions, sequence ACK
- **Exponential backoff reconnection** built into the client
- **Codec layer** for serialization — clean separation of concerns
- Works identically for Electron renderer ↔ local server AND remote WebUI ↔ headless server

### 🔧 1.6 Production-Grade Tooling
- **Electron builder** configured for macOS (DMG), Windows (NSIS), Linux (AppImage) with per-platform binary handling
- **Docker support** with multi-stage build, non-root user, TLS termination
- **Auto-update** via electron-updater with generic update provider
- **Sentry** integration for crash reporting
- **Husky** pre-commit hooks with i18n linting, typecheck, and coverage checks

### 📚 1.7 Exceptional Documentation (CLAUDE.md files)
- `packages/shared/CLAUDE.md` is a **masterclass** in context documentation — 200+ lines of precise rules, gotchas, and constraints
- Documents every subsystem: i18n conventions, agent lifecycle, credential handling, automation matching, network interceptor quirks
- Explicit "Hard rules" sections prevent common mistakes
- Documents versioned migration paths (e.g., opus-4.6-sunset)

### 🤝 1.8 Automation & Scheduling System
- **Event-driven automations**: LabelAdd/Remove, SessionStatusChange, scheduled cron triggers
- **Cron matching** engine with full expression support
- **Webhook utilities** for external integrations
- **Security validation** for automation configs
- **Telegram topic routing** — sessions can be auto-routed to forum topics

### 🛡️ 1.9 Thoughtful Security
- **Credential encryption** via dedicated credential manager
- **Bash command validation** with a custom parser that blocks dangerous constructs
- **Permission gating** at tool-use time with pre-tool-use pipeline
- **Environment sanitization** — strips Claude-specific Bedrock routing vars from subprocess env
- **Sandboxed script execution** (`script-sandbox`) with network/filesystem isolation

---

## 2. WEAKNESSES

### 🔴 2.1 Claude SDK Lock-in & Divergence
**Severity: HIGH**

The Claude Agent SDK (since v0.2.113) now spawns a **native binary** instead of running under Bun. This means:
- **No `--preload` for interceptor** — the network interceptor only works with Pi
- Features that relied on interceptor for Claude (rich tool intent, fast-mode override, MalformedBodyError validation) are **tracked as Phase-2 TODOs**
- The CLAUDE.md itself admits: *"Features that used to live in the interceptor for Claude... are tracked as Phase-2 work... they'll need to move to SDK hooks or a local proxy"*
- This creates a **permanent feature gap** between Claude and Pi backends

### 🔴 2.2 "Shared" Package is a God Package
**Severity: HIGH**

`@craft-agent/shared` is **400 TypeScript files** containing:
- All agent logic (Claude, Pi, BaseAgent)
- Source management, credential management
- Config, preferences, models
- MCP pool, session storage, skill storage
- Automation system, i18n, validation
- Network interceptor, prompt building
- Tools, permissions, mode management

This violates the "shared" naming — it's actually the **entire backend**. The monorepo has 11 packages, but ~75% of the logic lives in one. This makes:
- **Compilation slow** — changing anything requires re-checking the whole package
- **Testing hard** — test isolation is difficult when everything is coupled
- **Refactoring risky** — circular dependency risk is constant (note the `BEDROCK_TO_BARE` duplication comment in models.ts)

### 🟡 2.3 No Clear Plugin/Extension API
**Severity: MEDIUM**

Despite the "agent-native, customizable" messaging:
- **No public API** for extending the agent with custom tools beyond MCP
- **No plugin system** — everything is compiled into the monorepo
- Skills are just markdown files (prompts), not code
- Sources are either MCP servers or REST APIs — there's no way to write a native TypeScript source plugin
- To customize behavior, you must **fork and edit source code**

### 🟡 2.4 Test Distribution is Uneven
**Severity: MEDIUM**

- **313 test files total** — respectable number
- But `packages/shared` has **142 tests** for **400 source files** (~35% file coverage)
- `packages/server-core` and `packages/server` have minimal test coverage
- No E2E tests, no integration tests for the WebSocket RPC transport
- The Electron app has **74 tests** for **532 source files** (~14% file coverage)
- No performance benchmarks or load tests for the session/mcp pool subsystems

### 🟡 2.5 Heavy Dependency Footprint
**Severity: MEDIUM**

- **60+ root dependencies** + 19+ Electron-specific
- Bundles: Sharp (native image processing), TipTap (massive editor), Radix UI (component library), Shiki (syntax highlighting), react-pdf, ws, undici...
- The **Bun runtime is bundled** into the Electron app alongside Node.js (Bun for main process, Node for WhatsApp worker)
- **Claude SDK binary** is ~210MB per platform
- Total packaged app size is likely 500MB+

### 🟡 2.6 Windows is a Second-Class Citizen
**Severity: MEDIUM**

Evidence throughout the codebase:
- Build scripts use `bash` and `osascript` (macOS-only)
- `electron:dev:menu` script: `bash scripts/electron-dev.sh`
- `electron:dev:logs`: uses `pgrep` and `osascript`
- Many scripts reference `$PWD` (Unix) without Windows alternatives
- `fresh-start.ts` and `sync-secrets.sh` are bash-only
- `electron-builder.yml` has workarounds for Windows EBUSY errors with `.exe` files
- The `build:main:win` script exists separately, acknowledging different build paths
- `powershell-parser.ps1` and `powershell-validator.ts` exist but feel bolted on

### 🟡 2.7 Configuration Sprawl
**Severity: MEDIUM**

Configuration lives in **many places**:
- `.env` (server secrets)
- `~/.craft-agent/` (runtime config)
- Workspace-level configs (sources, skills, automations, statuses, labels, preferences, permissions, theme, tool-icons)
- Session-level configs (plans, data)
- `config-defaults.json` (bundled defaults)
- Code-level defaults scattered across files

There's no unified config schema or validation across all layers. The `validators.ts` file exists but doesn't cover everything.

### 🟢 2.8 Tight Coupling Between Renderer and Backend
**Severity: LOW-MEDIUM**

- React components import directly from `@craft-agent/shared` types
- Event processor maps backend events to UI state — but the mapping is hand-coded, not generated
- Adding a new RPC handler requires changes in: server-core handler → transport types → renderer event processor → React hook → UI component

---

## 3. MISSED OPPORTUNITIES

### 💎 3.1 No Agent-as-a-Service / Multi-Tenant Mode
The headless server exists but is **single-user**. With the session + workspace architecture already in place, multi-tenant support would be a relatively small leap. This would unlock:
- Team/organization deployments
- Shared source pools with per-user credentials
- Usage metering per tenant
- The WebUI already exists — it just needs auth beyond bearer token

### 💎 3.2 No Telemetry / Observability Layer
- No structured logging framework (just `debug()` and `console.error`)
- No OpenTelemetry or metrics export
- No token usage dashboards
- No latency tracking for tool execution
- The `UsageTracker` exists but only counts tokens — doesn't track performance
- This is critical for anyone running this in production

### 💎 3.3 No Streaming Event Bus / Real-time Sync
- Sessions are file-based (JSONL persistence)
- No event sourcing or CQRS pattern
- No real-time sync between multiple clients viewing the same session
- The `PushService` exists but is simple pub/sub — no conflict resolution
- Could have been a collaborative agent workspace

### 💎 3.4 No Vector Database / RAG Integration
- The codebase has ripgrep-based search (`search.ts`) but no semantic search
- No embedding generation or storage
- No knowledge base / document indexing beyond file system
- Given the "connect to any API" messaging, a built-in RAG pipeline would be a killer feature

### 💎 3.5 No Tool Composition / Workflow Builder
- Tools are individual operations — no way to chain them into workflows
- Automations can trigger sessions but can't define multi-step pipelines
- No visual workflow builder (the UI groundwork exists with the rich renderer)
- No DAG-based execution engine
- Missed opportunity to be a n8n/Make.com competitor with AI at the core

### 💎 3.6 No Mobile Companion
- The WebUI exists but isn't responsive
- No mobile app (React Native would share significant logic)
- No push notification integration
- The messaging gateway (Telegram/WhatsApp) partially addresses this but is indirect

### 💎 3.7 No Marketplace / Sharing Ecosystem
- Sources and skills are local-only — no way to share/publish them
- No source template marketplace
- No skill registry
- The session viewer exists for sharing sessions but not reusable components
- Could have been the "npm for AI agent configurations"

### 💎 3.8 Underutilized Browser Integration
- The browser tool exists but is treated as a fallback
- No browser automation recordings (like Playwright scripts)
- No web scraping pipeline
- No visual regression testing
- The CDP integration (`browser-cdp.ts`) could power much more

### 💎 3.9 No A/B Testing or Prompt Optimization
- System prompts are hardcoded
- No way to A/B test prompt variants
- No prompt performance tracking
- No automatic prompt optimization based on outcomes
- The `call_llm` tool could have been the foundation for a prompt evaluation framework

### 💎 3.10 No Native Database Integration
- All persistence is file-based (JSON, JSONL)
- No SQLite (even though it's built into Bun)
- No migration system for data format changes (only code-level migrations)
- This limits scalability for heavy users with many sessions/sources

---

## 4. Code Quality Assessment

### Positive Patterns
- **Zod schemas everywhere** — runtime validation is consistent
- **Canonical registries** — tool defs, models, locales all have single sources of truth
- **Event-driven architecture** — clean separation between event emission and handling
- **Config watching** — hot reload without restart
- **Defensive coding** — error mappers, diagnostics, fallback paths

### Areas for Improvement
- **Magic strings** — some tool names, event types are stringly-typed
- **God classes** — `ClaudeAgent` is 2,800+ lines, `PiAgent` is 2,500+ lines
- **Comment density** — some files have more comments than code (CLAUDE.md is amazing but the inline comments in agents suggest complexity is high)
- **Inconsistent error handling** — some modules throw, some return errors, some use Result types

---

## 5. Technology Choices Verdict

| Choice | Assessment | Notes |
|--------|-----------|-------|
| Bun runtime | ✅ Good | Fast startup, native TS, but dual-runtime (Bun+Node) adds complexity |
| Electron | ⚠️ Acceptable | Heavy but necessary for desktop features; Tauri would be lighter |
| Claude Agent SDK | ⚠️ Mixed | Powerful but native binary limits extensibility |
| Pi AI SDK | ✅ Good | Flexible subprocess model, good for custom models |
| TipTap editor | ✅ Great | Best-in-class rich text for React |
| Radix UI | ✅ Good | Accessible primitives, unstyled |
| WebSocket RPC | ✅ Great | Custom protocol is well-designed |
| File-based storage | ⚠️ Limiting | Simple but doesn't scale |
| Zod | ✅ Excellent | Runtime validation everywhere |
| Jotai (state) | ✅ Good | Atomic state management, good for React |

---

## 6. Summary Scorecard

| Dimension | Score | Notes |
|-----------|-------|-------|
| Architecture | 8/10 | Clean layers, good abstractions, but shared package is overloaded |
| Code Quality | 7/10 | Well-documented, consistent patterns, but some god classes |
| Test Coverage | 5/10 | 313 tests but uneven distribution, no E2E |
| Extensibility | 5/10 | Great via MCP/API sources, but no plugin system for core |
| Documentation | 9/10 | Exceptional CLAUDE.md files, good inline docs |
| Production Readiness | 7/10 | Docker, auto-update, Sentry — but limited observability |
| Cross-Platform | 6/10 | macOS first-class, Windows works, Linux basic |
| Scalability | 4/10 | File-based storage, single-user server |
| Innovation | 8/10 | Agent-native concept is genuinely novel |
| Developer Experience | 7/10 | Good tooling, but heavy dependency tree |

**Overall: 7.0/10** — A strong foundation with clear vision, hampered by some architectural debt from rapid multi-model expansion and the inherent complexity of bridging multiple AI SDKs.

---

## 7. Recommendations for Building Your Own Project

If you're forking this codebase, prioritize:

1. **Split `@craft-agent/shared`** into 4-5 focused packages (agent, config, sources, mcp, sessions)
2. **Replace file storage** with SQLite (built into Bun) for sessions and config
3. **Build a plugin API** on top of the existing source/skill system
4. **Add observability** (structured logging, metrics, token dashboards)
5. **Invest in E2E testing** — the transport layer and agent lifecycle are undertested
6. **Consider Tauri** instead of Electron if you want a lighter desktop app
7. **Abstract the Claude SDK dependency** behind a more generic interface to avoid lock-in
