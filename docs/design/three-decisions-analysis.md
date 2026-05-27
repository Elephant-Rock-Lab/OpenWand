# Three Open Decisions: Analysis & Recommendations

**Date:** 2026-05-26  
**Status:** Analysis complete — recommendations for user decision

---

## Decision 1: cqrs-es or Loro-only for Event Sourcing?

### What cqrs-es Actually Is

`cqrs-es` v0.5.0 — a lightweight CQRS + event sourcing framework targeting serverless architectures.

Core model:
```rust
trait Aggregate: Default + Serialize + DeserializeOwned {
    const TYPE: &'static str;
    type Command;
    type Event: DomainEvent;
    type Error: std::error::Error;
    type Services: Send + Sync;

    fn handle(&mut self, cmd: Self::Command, services: &Self::Services, sink: &EventSink<Self>)
        -> impl Future<Output = Result<(), Self::Error>> + Send;

    fn apply(&mut self, event: Self::Event);
}
```

It gives you:
- Aggregate trait with Command → Event → State transitions
- Event store abstraction (PostgreSQL, MySQL, DynamoDB, SQLite)
- CqrsFramework orchestrating command dispatch
- Separate read models via Query trait
- Snapshots + event replay

### What Loro Actually Gives You

Loro v1.12 — a CRDT framework with:

| Capability | What it does |
|---|---|
| `LoroDoc` | One collaborative document with typed containers (text, map, list, tree, movable_list, counter) |
| `subscribe` / `subscribe_root` | Reactive event system — fires on commit |
| `oplog_vv` / `state_frontiers` | Version vector tracking — knows what changed since when |
| `checkout` / `checkout_to_latest` | Time travel to any version frontier |
| `revert_to` | Undo to a specific version |
| `fork` / `fork_at` | Branch documents |
| `export(ExportMode::updates(&vv))` | Incremental delta export |
| `import` | Merge changes from any peer |
| `UndoManager` | Local undo/redo stack |
| `ChangeMeta` | Metadata per change (peer, timestamp, lamport) |

### The Real Question

Event sourcing and CRDT solve **different problems**:

| | Event Sourcing (cqrs-es) | CRDT (Loro) |
|---|---|---|
| **Purpose** | Rebuild aggregate state from event log | Merge concurrent edits without conflicts |
| **Granularity** | Domain events (Command → Event → State) | Op-level changes (insert char, set key, delete item) |
| **Replay** | Apply events sequentially to rebuild state | Checkout any version frontier |
| **Conflict** | Last-writer-wins or custom resolution | Mathematical merge (automatically correct) |
| **Collaboration** | Single writer, event stream | Multi-writer, offline-capable |
| **Schema** | Strongly typed (Command, Event, Aggregate) | Unstructured containers (map, list, text) |
| **Read models** | Separate projections from events | Subscribe to container changes |
| **Audit** | Every command produces typed events | Every op is tracked in version vector |

### What OpenWand Needs

OpenWand has **two** event sourcing needs:

**1. Session state** (conversation, messages, tool calls, edits)
- Multi-writer: LLM + user both produce changes
- Needs: branching (fork session), time travel, merge (fork back), undo/redo
- Already decided: **Loro CRDT** — this is the right tool

**2. Memory domain** (episodes, entities, facts, decisions)
- Single-writer: deterministic pipeline ingests episodes
- Needs: typed events (FactCreated, FactInvalidated, DecisionMade), audit trail, replay
- NOT multi-writer, NOT collaborative
- This is where cqrs-es patterns could apply

### Recommendation: Loro for sessions, NO framework for memory

**Don't use cqrs-es.** Here's why:

1. **Too heavy for what we need.** cqrs-es is designed for distributed serverless systems with separate read/write stores. OpenWand is a single-process local-first tool. We don't need CQRS — we don't have separate read and write paths at the database level.

2. **The memory domain is already designed.** The memory crate has its own typed pipeline: `Episode → Extract → Classify → Write`. This IS event sourcing — but bespoke to our domain. cqrs-es's `Aggregate` trait doesn't map cleanly onto "extract facts from episodes and apply temporal rules."

3. **Loro covers the collaboration case.** For session state (where multi-writer matters), Loro already gives us version tracking, undo, branching, and merge. We'd be duplicating capability.

4. **The patterns we want are simple.** What we actually need:
```rust
// This is the entire "event sourcing" OpenWand needs for memory:
pub enum MemoryEvent {
    EpisodeStored(Episode),
    FactCreated(Fact),
    FactInvalidated { fact_id: FactId, superseded_by: Option<FactId>, valid_to: DateTime<Utc> },
    FactRefined { fact_id: FactId, new_claim: String, new_confidence: f64 },
    DecisionCreated(Decision),
    DecisionSuperseded { old_id: DecisionId, new_id: DecisionId },
    EntityCreated(Entity),
    EntitySummaryUpdated { entity_id: EntityId, new_summary: String },
}
```

These events are stored in the decision ledger. They're not replayed to rebuild state — the storage engine holds the current state. The events are for **audit**, not for state reconstruction.

### The Verdict

| Component | Event sourcing mechanism |
|---|---|
| Session state (`openwand-session`) | **Loro CRDT** — already decided, already in Cargo.toml |
| Memory mutations (`openwand-memory`) | **Decision ledger** — typed `MemoryEvent` enum, append-only log, for audit not replay |
| Workflow state (`openwand-workflow`) | **Loro CRDT** — one document per mod, sessions subscribe to keys |

No external CQRS framework needed. The patterns are simple enough to implement directly.

---

## Decision 2: HelixDB vs CozoDB for Storage

### What HelixDB Actually Is

HelixDB v2.0 — a graph-vector database built in Rust.

**Architecture:**
- Storage engine: LMDB (memory-mapped, ultra-low latency)
- Data model: Graph nodes + edges + vector embeddings + KV + documents
- Query: Custom DSL with JSON API (`POST /v1/query`)
- Built-in: Embeddings, MCP tools, vector search, keyword search, graph traversal
- Y Combinator backed (W25 batch)

**Critical facts from source code review:**

1. **HelixDB is server-only.** The Rust SDK (`helix-db` crate) is an HTTP client using `reqwest`. It connects to `http://localhost:6969`. There is no embedded mode. No `kv-rocksdb` feature. No in-process API.

2. **HelixDB is AGPL licensed.** From their README: "HelixDB is licensed under The AGPL (Affero General Public License)." AGPL requires any network user to receive source code. For a local-first desktop tool, this might be fine (no network serving), but it's a legal gray area and a friction point for any future distribution.

3. **HelixDB requires a running server process.** `helix run dev` starts a server. This violates HB-G1 (no external deps, single binary) and the "local-first, no separate service" constraint.

4. **HelixDB is very new.** YC W25 (early 2025). v2.0. The query DSL is still evolving. The codebase structure suggests active development with potential for breaking changes.

### What CozoDB Actually Is

CozoDB v0.7.6 — an embedded relational-graph database using Datalog.

**Architecture:**
- Storage engines: SQLite, RocksDB, Sled, TiKV (5 options)
- Data model: Relations with Datalog queries
- Query: Datalog (declarative, compositional, recursive)
- Built-in: Graph algorithms (PageRank, shortest path, community detection, k-core), HNSW vector search, FTS, MinHash-LSH near-duplicate detection, time travel
- License: MPL-2.0 (OSI-approved)
- Runs fully embedded in-process

### The Comparison

| | HelixDB | CozoDB |
|---|---|---|
| **Embedded** | ❌ Server-only (HTTP client) | ✅ In-process |
| **License** | ❌ AGPL (viral) | ✅ MPL-2.0 (OSI) |
| **External deps** | ❌ Requires running server | ✅ Zero — single binary |
| **Graph** | Nodes, edges, traversal | Relations + recursive Datalog + algorithms |
| **Vectors** | HNSW built-in | HNSW built-in |
| **FTS** | Keyword search | Full-text search + BM25 |
| **Time travel** | No | Per-relation opt-in |
| **Near-duplicate** | No | MinHash-LSH built-in |
| **Maturity** | v2.0, YC W25, active API churn | v0.7.6, stable, academic origin |
| **Binary size** | N/A (server) | Light (SQLite backend) |
| **Query lang** | Custom DSL via JSON HTTP | Datalog (learning curve but powerful) |
| **Community** | Larger, YC-backed, Discord | Smaller, academic, GitHub |

### The Verdict

**This is not a contest. HelixDB is eliminated.**

HelixDB fails three hard requirements:
1. **Not embeddable** — requires a separate server process
2. **AGPL license** — legal risk for any distribution
3. **Not local-first** — violates the core product promise

CozoDB is the only candidate that satisfies all constraints. It's embedded, MPL-2.0, graph + vectors + FTS + time travel in one library.

But recall: CozoDB is one of **three** candidates in the benchmark (CozoDB, SurrealDB, SQLite). The memory crate design uses a `MemoryStore` trait, and the benchmark decides.

**HelixDB is out. CozoDB, SurrealDB, and SQLite remain in the benchmark.**

---

## Decision 3: Awaken's 9-Phase Execution vs Custom Agent Loop

### What Awaken's 9 Phases Actually Are

From the source code (`phase.rs`):

```
RunStart          ← once per run
  StepStart       ← per step (iteration)
    BeforeInference
    AfterInference
    ToolGate
    BeforeToolExecute
    AfterToolExecute
  StepEnd
RunEnd            ← once per run
```

Each phase has:
- Plugin hooks (before/after)
- State store mutations (typed keys, commit batches)
- Event sink (streaming to caller)
- Cancellation tokens
- Checkpoint support (persist state for resume)
- PhaseContext with messages, agent spec, run identity

The `loop_runner` orchestrates:
1. `RunStart` → initialize lifecycle, register state keys
2. Loop:
   a. `StepStart` → prepare context messages, apply throttling
   b. `BeforeInference` → plugins inject context, override model params
   c. **Inference** → call LLM, stream response
   d. `AfterInference` → extract tool calls, update token counts
   e. `ToolGate` → permission check, intercept tool calls
   f. `BeforeToolExecute` → pre-execution hooks
   g. **Tool Execute** → run tools, collect results
   h. `AfterToolExecute` → post-execution hooks
   i. `StepEnd` → commit state, check termination
3. `RunEnd` → finalize, cleanup

Additional capabilities:
- **Circuit breaker** on inference failures
- **Suspension** (tool calls can suspend for human approval)
- **Compaction** (compress conversation history when context grows)
- **Tool filtering** (plugins can modify which tools are visible)
- **Resume** (restart a suspended run)
- **Parallel tool execution** (feature-gated)
- **State machine tracking** (`RunLifecycle`: Running → StepCompleted → Done/Waiting)

### What OpenWand Needs

OpenWand's agent loop needs to:
1. Receive user input
2. Send to LLM with context (memory retrieval, session history, tool descriptions)
3. Parse response (text, tool calls, reasoning)
4. If tool calls: route through tool gate (policy check), execute, collect results
5. If text: stream to user
6. Loop until natural end or user stop
7. Record episodes in memory
8. Respect mode (Direct/Conversational/Auto/Custom)
9. Handle MCP tool routing via rmcp

### What thClaws Does (for comparison)

thClaws's `agent.rs` (3,185 LOC) has a simpler loop:
```
Plan mode: user message → plan → show plan → await approval → execute
Execute mode: stream LLM response → parse tool calls → run tool gate → execute → collect → loop
```

thClaws's innovations (from our earlier analysis):
- Event stream architecture (AgentEvent enum)
- Injection queue (prepend user messages)
- Output token escalation (switch to bigger model when stuck)
- Truncate-to-disk (persist conversation to disk, reload summary)
- Plan mode state machine
- Model hot-swap
- Cooperative cancellation

### Analysis: Adopt, Adapt, or Build?

**The 9-phase model is excellent.** Here's why:

| Phase | What it enables for OpenWand |
|---|---|
| `RunStart` / `RunEnd` | Session lifecycle hooks — capture episodes, update memory |
| `StepStart` | Context assembly — inject memory retrieval results, apply interaction mode |
| `BeforeInference` | Dynamic system prompt assembly (mode-dependent), tool list filtering |
| `AfterInference` | Token tracking, episode capture (assistant message), streaming events |
| `ToolGate` | Policy enforcement — the deterministic gate from Trust Architecture |
| `BeforeToolExecute` | MCP routing decision, confirmation prompt (Conversational mode) |
| `AfterToolExecute` | Episode capture (tool result), memory ingestion trigger |
| `StepEnd` | Decision ledger commit, workflow state check |

**But don't adopt Awaken's implementation.** Here's why:

1. **Awaken is a framework, not a library.** It has its own state management (`StateStore` with `MutationBatch`), plugin system, registry, observability, and persistence layer. Adopting it means adopting all of it, or spending weeks extracting the phase runner from its ecosystem.

2. **Awaken targets multi-agent orchestration.** Its registry, agent resolution, handoff system, and skill dispatch are designed for switching between specialized agents. OpenWand is single-agent (at least initially).

3. **Awaken uses `genai` for LLM calls.** OpenWand should use `rig` (already decided). Different provider abstraction.

4. **Awaken's state management conflicts with Loro.** Awaken has its own `StateStore` with typed keys and mutation batches. OpenWand uses Loro CRDT for session state. These are incompatible.

### Recommendation: Build a custom loop inspired by Awaken's phases

**Steal the phase model. Build the implementation.**

```rust
pub enum Phase {
    RunStart,
    StepStart,
    BeforeInference,
    AfterInference,
    ToolGate,
    BeforeToolExecute,
    AfterToolExecute,
    StepEnd,
    RunEnd,
}
```

But with OpenWand-specific wiring:

| Awaken concept | OpenWand equivalent |
|---|---|
| `PhaseRuntime` + hooks | Custom phase runner with `PhaseHook` trait |
| `StateStore` + `MutationBatch` | Loro CRDT documents |
| `genai` executor | `rig` via `openwand-llm` |
| `EventSink` | Loro event subscriptions + UI signal |
| `AgentResolver` | Not needed (single agent) |
| `Plugin` system | Simplified hook registration |
| `ThreadRunStore` checkpoint | Loro snapshots |
| `CancellationToken` | tokio `CancellationToken` (same) |
| Circuit breaker | From CC Switch (already designed) |
| Compaction | Loro shallow snapshots |
| Tool suspension | Interaction mode (Conversational: ask user) |
| Tool filtering | rmcp capability-based routing |

### The Custom Loop Design

```
RunStart
  ├─ Initialize Loro document for this run
  ├─ Load session context (memory retrieval)
  ├─ Determine interaction mode
  └─ Register MCP tools via rmcp

Step loop:
  StepStart
    ├─ Assemble context: session history + memory retrieval + mode prompt
    └─ Apply thinking budget (from CC Switch)

  BeforeInference
    ├─ Inject system prompt (mode-dependent)
    ├─ Filter tool list (policy-dependent)
    └─ Stream start signal to UI

  INFERENCE (rig → LLM)
    ├─ Stream text deltas to UI
    └─ Collect tool calls

  AfterInference
    ├─ Record episode (assistant message)
    ├─ Track token usage
    └─ If no tool calls → natural end

  ToolGate (deterministic)
    ├─ Policy check (is this tool allowed?)
    ├─ Confirmation check (Conversational mode → ask user)
    ├─ Risk assessment (from Trust Architecture)
    └─ Reject / approve / escalate

  BeforeToolExecute
    ├─ Route to MCP server (rmcp) or local tool
    └─ Record tool call episode

  TOOL EXECUTE
    ├─ Execute tool
    └─ Collect result

  AfterToolExecute
    ├─ Record tool result episode
    ├─ Trigger memory ingestion (async)
    └─ Stream result to UI

  StepEnd
    ├─ Commit decision ledger
    ├─ Check termination (max steps, user stop, token budget)
    └─ Loop or end

RunEnd
  ├─ Final memory ingestion
  ├─ Commit decision ledger
  ├─ Update session metadata
  └─ Signal UI
```

### What to Steal from Each Source

| From Awaken | From thClaws | From CC Switch | From Craft Agents |
|---|---|---|---|
| 9-phase model | Event stream (`AgentEvent`) | Circuit breaker (3-state) | Spawn session concept |
| Phase hooks | Injection queue | Thinking budget rectifier | Tool permission levels |
| State machine tracking | Truncate-to-disk | Cache breakpoint injection | Streaming UI updates |
| Suspension/resume | Model hot-swap | Failover queue | Message types |
| Compaction | Cooperative cancellation | Thinking optimizer | Session persistence |
| Tool filtering | Output token escalation | | |

### Where This Lives

The agent loop goes in `openwand-session` (or a new `openwand-agent` crate if it grows large enough). It depends on:

- `openwand-core` (types, errors)
- `openwand-llm` (rig-based inference)
- `openwand-memory` (episode ingestion, retrieval)
- `openwand-tools` (tool registry, execution)
- `openwand-mcp-pool` (rmcp routing)
- `openwand-policy` (tool gate, risk assessment)
- `loro` (session state, CRDT)

---

## Summary of Recommendations

| Decision | Recommendation | Confidence |
|---|---|---|
| **cqrs-es vs Loro-only** | **Neither framework.** Loro for sessions, bespoke `MemoryEvent` enum + decision ledger for memory. No external CQRS framework. | High |
| **HelixDB vs CozoDB** | **HelixDB eliminated** (not embedded, AGPL, server-only). CozoDB remains in benchmark alongside SurrealDB and SQLite. | Very high |
| **Awaken 9-phase vs custom** | **Steal the phase model, build the implementation.** Don't adopt Awaken's framework — too much baggage. The 9 phases are the right shape. Wire them to Loro + rig + rmcp + our memory pipeline. | High |

### What This Unlocks

With these three decisions resolved:

1. **No new dependencies** for event sourcing (Loro already in tree, decision ledger is bespoke)
2. **Benchmark narrows** to 3 engines: CozoDB, SurrealDB, SQLite (HelixDB eliminated)
3. **Agent loop design** is clear: 9-phase model, custom wiring, steal from 4 sources
4. **Architecture is complete** for the core crates (session, memory, llm, tools, mcp-pool, policy)

The only remaining open question from the original list is **when to build the workflow engine** (Batch 1 or later), which is a scheduling decision, not an architecture decision.
