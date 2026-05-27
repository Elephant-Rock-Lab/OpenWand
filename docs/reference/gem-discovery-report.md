# 🎯 Gem Discovery Report
## 683 Projects Surveyed • 42 Deep-Explored • 12 Must-Borrow Gems

---

## 💎 TIER 1 — Must-Borrow Gems (Transformative)

These aren't just nice-to-haves. Each one represents a structural advantage over existing agent tools.

---

### 1. Loro CRDT — Version-Controlled Sessions That Collaborate
**What:** Rust CRDT library (v1.0). Makes JSON data collaborative and version-controlled. Rich text editing, movable trees/lists, time travel, shallow snapshots.

**Why It Matters:** This solves TWO problems at once:
- **Session branching** becomes native — fork a conversation, merge it back, time-travel to any point
- **Collaboration** becomes possible — two humans (or agents) can edit the same session simultaneously with automatic conflict-free merging

**How to Adapt:** Replace Craft Agents' JSONL session storage with a Loro document per session. Each message is a CRDT operation. Branching, merging, and undo/redo are free. The `taino-edit-dioxus` rich text editor could use Loro's rich text CRDT directly for collaborative editing.

**Source:** `loro-main/` — Rust + WASM + Swift, docs.rs/loro, loro.dev

---

### 2. Goose — A Rust Agent That Already Exists
**What:** Rust-based AI agent (Linux Foundation). Desktop app + CLI + API. 15+ providers, 70+ MCP extensions. Built entirely in Rust.

**Why It Matters:** You don't need to build a Rust agent from scratch. Goose already:
- Has MCP client support for 70+ extensions
- Runs as a native desktop app
- Supports 15+ LLM providers
- Is backed by the Linux Foundation (not going away)

**How to Adapt:** Use Goose as a reference implementation or potentially as a foundation. Study its MCP pool management, provider abstraction, and desktop app architecture. Key insight: goose uses `rmcp` (the official Rust MCP SDK) — confirming our matrix.

**Source:** `goose-main/` — github.com/aaif-goose/goose, Rust, Apache 2.0

---

### 3. Dolt — Git for Session Data
**What:** SQL database (MySQL-compatible) that you can fork, clone, branch, merge, and diff. Versions tables like Git versions files.

**Why It Matters:** Session storage is a first-class concern. Dolt gives you:
- `SELECT * FROM messages WHERE session_id = X AND branch = 'main'`
- `git log` style history of every session change
- `git diff` between session branches
- `git merge` to merge agent exploration back into main
- Full SQL querying over your session data

**How to Adapt:** Embed Dolt as the session store. Each session is a branch. Messages, tool calls, and artifacts are rows. You can query across all sessions with SQL. Branching is a native operation, not a hack. The Dolt developers literally say: "It's the best database for agent memory."

**Source:** `dolt-main/` — github.com/dolthub/dolt, Go (embeddable), AGPL-3.0

---

### 4. ESAA — "Agents Propose, Orchestrator Disposes"
**What:** Event Sourcing for Autonomous Agents. Append-only event log as source of truth. Agents emit validated JSON intentions — they CANNOT write files, mutate state, or append events directly. The orchestrator validates, persists, and projects.

**Why It Matters:** This is the correct architecture for a multi-agent system:
- Every agent action is an immutable event in an append-only log
- State is derived by replaying events (deterministic projection)
- Full audit trail of every decision every agent ever made
- Hash-verified integrity — you can prove the current state is correct

**How to Adapt:** Build the session/event system as an event store. Each message, tool call, and state change is an event. The current UI state is a projection (replay events → render). This makes undo, branching, replay, and debugging trivial. Works naturally with Loro CRDT for the data layer.

**Source:** `ESAA---Event-Sourcing-Agent-Architecture-main/` — TypeScript patterns, arxiv.org/pdf/2602.23193

---

### 5. ACE — Agents That Learn From Their Mistakes
**What:** Agentic Context Engine. Agents automatically learn from experience. Tracks failures, extracts strategies, builds a "Skillbook." No fine-tuning, no training data, no vector database. Proven: 2x consistency, 49% token reduction.

**Why It Matters:** Every agent tool today is stateless between sessions. ACE changes that:
- Agent hallucinates a "seahorse emoji"? ACE records the failure.
- Next time the question comes up, the learned strategy is injected.
- Over time, the agent gets better WITHOUT retraining.

**How to Adapt:** Add a "Skillbook" — a persisted collection of learned strategies. After each tool call or response, run a lightweight reflection step: "Did this work? What can we learn?" Store the lesson. On future similar contexts, inject relevant lessons. This is cheaper and more practical than RAG for self-improvement.

**Source:** `agentic-context-engine-main/` — Python, pip install ace-framework, kayba.ai

---

### 6. GOAL.md — Self-Measuring Autonomous Improvement
**What:** A single file pattern that turns any coding agent into an autonomous improver. Fitness function + improvement loop + action catalog. The agent constructs its own metrics.

**Why It Matters:** This is a meta-gem. The insight is:
- Give an agent a number to make go up
- Give it a loop to do it in
- It will improve itself overnight

The key innovation: when no natural metric exists, the agent constructs its own "ruler" first, then measures with it. Documentation quality, API trustworthiness, code health — all measurable once you construct the instrument.

**How to Adapt:** Build a "Goal" system into your tool. Users define what "better" means (or the agent constructs it). The agent runs improvement loops autonomously, measuring progress against the defined metric. This turns your tool from "chat interface" into "autonomous improvement engine."

**Source:** `goal-md-main/` — github.com/jmilinovich/goal-md, MIT

---

### 7. Det-ACP — Agent Governance Gateway
**What:** Deterministic Agent Control Protocol. Agents never execute tools directly. Every action flows through a control plane for evaluation, enforcement, and audit. Policy DSL, self-evolving policies.

**Why It Matters:** This solves the #1 fear of autonomous agents: "What if it does something destructive?" With Det-ACP:
- Block `.env` file access automatically
- Prevent credential exfiltration
- Enforce boundaries on what tools can do
- Every action is auditable
- Policies can self-evolve based on patterns

**How to Adapt:** Build a policy engine into the Rust core. Every tool call passes through a policy check before execution. Policies are YAML files with simple rules: "block reads on files matching *.env", "require confirmation for network requests", "allow file writes only in project directory". The Craft Agents "permission modes" (Explore/Ask/Execute) are a crude version of this — Det-ACP is the proper implementation.

**Source:** `deterministic-agent-control-protocol-main/` — TypeScript, npm @det-acp/core, MIT

---

### 8. Iroh — Peer-to-Peer Agent Networking
**What:** Rust P2P networking library. Dial by public key. QUIC-based. Hole-punching for direct connections. BLAKE3 content-addressed blob transfer. Gossip pub/sub. Eventually-consistent key-value store.

**Why It Matters:** This enables a fundamentally different architecture:
- Agents on YOUR laptop can talk to agents on YOUR GPU machine directly
- No server needed — P2P discovery and communication
- Content-addressed sharing — share session data, skills, and artifacts by hash
- Gossip protocol for multi-agent coordination

**How to Adapt:** Use Iroh as the networking layer between devices. Your laptop agent orchestrates, your GPU machine runs heavy inference, your phone receives notifications — all connected P2P without a central server. Combined with EasyTier (mesh VPN, also Rust), you get a secure agent mesh network.

**Source:** `iroh-main/` — Rust, docs.rs/iroh, iroh.computer, MIT/Apache 2.0

---

## 🥈 TIER 2 — Strong Patterns (Adopt the Concept)

| Gem | What | Why Borrow It | How |
|---|---|---|---|
| **Letta/MemGPT** | Memory blocks (human, persona, core) | Structured, labeled memory sections that persist across sessions | Add `MemoryBlock` trait to agent runtime — human preferences, persona definition, learned facts |
| **mem0** | Multi-level memory (User → Session → Agent) | Different memory scopes for different purposes | Three-tier memory: user prefs (forever), session context (conversation), agent skills (learned) |
| **FalkorDB** | Graph database with sparse matrix adjacency | Purpose-built for "Agent Memory" — entities and relationships as first-class | Use as knowledge graph backend. Agent learns "Alice works at Company → Company uses React" as edges |
| **SpiceDB / OpenFGA** | Google Zanzibar authorization | "Can agent X access resource Y?" at 5ms p95 | Use for permission model: which agents can access which sessions, sources, skills |
| **Temporal + mcp-agent** | Durable workflow execution | Agent tasks survive crashes, restarts, network failures | Build session persistence as durable workflows — a crashed agent picks up exactly where it left off |
| **mmdr** | Pure Rust Mermaid rendering | 23 diagram types, 100-1400x faster than mermaid-cli | Already in our matrix — confirmed gem. Use directly. |
| **Deep Agents** | "Batteries-included" agent harness | Planning, filesystem, shell, sub-agents, auto-summarization | Adopt the "harness" pattern: opinionated defaults with override hooks |
| **Zellij** | Rust terminal multiplexer with WASM plugins | Plugin system, multiplayer, built-in web client | Study its WASM plugin architecture for your extension system |

---

## 🥉 TIER 3 — Worth Considering (Nice-to-Have)

| Gem | What | Why It's Interesting |
|---|---|---|
| **EvoSkill** | Evolutionary skill discovery | Automated prompt optimization through evolution — may be overkill for v1 |
| **Wave Terminal** | AI-integrated terminal, durable SSH | Durable sessions pattern — but Dolt + ESAA covers this better |
| **Company OS** | Conversations → structured knowledge | Interesting pattern but too specific to meeting transcription |
| **Olares** | Personal cloud OS | Ambitious concept but Kubernetes-based — too heavy for our use case |
| **d3-force-3d** | 3D force-directed graph layout | Could visualize session trees beautifully, but premature for v1 |
| **Ratatui** | Rust TUI framework | Could build a terminal companion to the GUI — nice debug tool |
| **Fabric** | Organized prompt marketplace | The "Pattern" library concept is good for skills — but our skill system already covers this |
| **Agentic Inbox** | AI email on Cloudflare Workers | Per-session SQLite isolation pattern — Dolt does this better |
| **TEN Framework** | Real-time multimodal AI | Voice/video agent interaction — fascinating but not v1 |
| **Khoj** | Personal AI second brain | Automated research + notifications — good feature to add later |
| **Context Engineering** | 1400-paper survey of context design | Essential reading, not a library — study the discipline |
| **Ghostty / WezTerm** | Terminal emulators (Zig/Rust) | Study architecture patterns for embedding a terminal |

---

## 🏗️ Architecture Insights (Cross-Cutting Patterns)

### Pattern 1: Event Sourcing is the Correct Session Model
Three independent projects converge on the same answer:
- **ESAA** — event sourcing for agent actions
- **KurrentDB** — event-native database
- **Craft Agents** — JSONL append-only logs

The right answer: append-only event log, derived state via projection. Dolt gives you SQL querying on top. Loro gives you CRDT merging on top.

### Pattern 2: Memory Has Three Levels
**mem0**, **Letta**, and **ACE** all independently converge on:
1. **User memory** — who you are, what you prefer (permanent)
2. **Session memory** — what happened in this conversation (temporary)
3. **Agent memory** — what the agent learned about how to work better (evolving)

### Pattern 3: Agents Need Governance, Not Just Permissions
**Det-ACP**, **SpiceDB**, and **ESAA** all address the same problem differently:
- Det-ACP: policy gateway that intercepts every tool call
- SpiceDB: relationship-based authorization queries
- ESAA: agents can't mutate state directly

The right answer: combine Det-ACP's policy gateway with SpiceDB's relationship model.

### Pattern 4: P2P Beats Client-Server for Agent Communication
**Iroh**, **EasyTier**, and **Tailscale** all prove that direct P2P connections are better than routing through a server. For multi-device agent setups (laptop orchestrates, GPU runs inference), P2P is the natural fit.

### Pattern 5: "Fitness Function" Enables Autonomous Improvement
**GOAL.md** and **ACE** both demonstrate that giving agents a measurable target + a loop creates autonomous improvement. This is the "go to sleep, wake up to better code" pattern.

---

## ⚠️ WHAT NOT TO BORROW (Traps)

| Temptation | Why It's a Trap |
|---|---|
| **Using LangGraph/LangChain** | You're building in Rust. These are Python-only. The patterns are worth studying, the libraries are not. |
| **Building your own CRDT** | Loro v1.0 exists and is production-grade in Rust. Don't reinvent. |
| **Building your own graph database** | FalkorDB, Dolt, and SurrealDB all exist. Pick one. |
| **Embedding Kubernetes (Olares pattern)** | Way too heavy for a personal agent tool. Your users don't want to run k8s. |
| **Building a multi-channel messaging layer (OpenClaw pattern)** | 24+ channel integrations is a maintenance nightmare. Start with 2-3. |
| **Using Cedar/OPA for v1 permissions** | Det-ACP's simpler policy DSL is better for v1. Add Zanzibar later if you need multi-user. |
| **Trying to build everything at once** | The gem list is inspiring but dangerous. Start with Tier 1 items 1-5, add others incrementally. |

---

## 🗺️ The Synthesis: What Your Tool Should Actually Be

Based on every gem discovered across 683 projects, here's the emergent architecture:

```
┌─────────────────────────────────────────────────────────────────────┐
│                    YOUR AGENT TOOL (Rust)                           │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │  Dioxus Desktop UI                                          │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │    │
│  │  │ taino-edit   │  │ Session Tree │  │ Goal Dashboard   │  │    │
│  │  │ (Loro CRDT)  │  │ (Dolt + mmdr)│  │ (GOAL.md pattern)│  │    │
│  │  └──────────────┘  └──────────────┘  └──────────────────┘  │    │
│  └───────────────────────────┬─────────────────────────────────┘    │
│                              │                                      │
│  ┌───────────────────────────▼─────────────────────────────────┐    │
│  │  Rust Core (tokio + axum)                                   │    │
│  │                                                             │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │    │
│  │  │ Event Store  │  │ Policy       │  │ Memory Engine    │  │    │
│  │  │ (ESAA model) │  │ Engine       │  │ (ACE Skillbook + │  │    │
│  │  │ + Dolt/SQLite│  │ (Det-ACP)    │  │  mem0 hierarchy) │  │    │
│  │  └──────────────┘  └──────────────┘  └──────────────────┘  │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │    │
│  │  │ rmcp         │  │ Agent Loop   │  │ P2P Mesh         │  │    │
│  │  │ (MCP SDK)    │  │ (Goose/      │  │ (Iroh +          │  │    │
│  │  │              │  │  pi_agent)   │  │  EasyTier)       │  │    │
│  │  └──────────────┘  └──────────────┘  └──────────────────┘  │    │
│  └─────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
```

### The Differentiator Stack (What No Other Agent Tool Has)

1. **CRDT-backed sessions** (Loro) — fork, merge, time-travel, collaborate
2. **Event-sourced audit trail** (ESAA) — every action recorded, replayable, verifiable
3. **Self-improving agents** (ACE) — learn from mistakes, accumulate strategies
4. **Fitness-function autonomy** (GOAL.md) — define "better," agent improves while you sleep
5. **P2P agent mesh** (Iroh) — agents across your devices, no server needed
6. **Policy-governed execution** (Det-ACP) — safe autonomous operation with guardrails

No single existing tool has more than 2 of these 6. Your tool can have all 6.
