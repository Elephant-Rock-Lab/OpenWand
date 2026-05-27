# thClaws Agent Loop Deep Analysis

**Source:** `C:\Next AI\ref\thClaws-main\crates\core\src\agent.rs`
**Size:** 3,185 LOC | 153 source files | ~111K LOC total
**Date:** 2026-05-26

---

## Architecture Overview

```
┌──────────────────────────────────────────────────────────────┐
│                      Agent::run_turn()                       │
│                   (returns impl Stream)                       │
│                                                              │
│  ┌─────────┐    ┌──────────┐    ┌──────────────┐            │
│  │  User    │───▶│ History  │───▶│   Compact    │            │
│  │ Message  │    │ (Mutex)  │    │  (if over    │            │
│  └─────────┘    └──────────┘    │   budget)    │            │
│                                  └──────┬───────┘            │
│                                         │                    │
│                              ┌──────────▼──────────┐        │
│                              │  provider.stream()  │        │
│                              │  (with retry/backoff)│        │
│                              └──────────┬──────────┘        │
│                                         │                    │
│                              ┌──────────▼──────────┐        │
│                              │     assemble()      │        │
│                              │  (parse SSE stream) │        │
│                              └──────────┬──────────┘        │
│                                         │                    │
│                    ┌────────────────────┼────────────┐       │
│                    │                    │            │       │
│              ┌─────▼─────┐    ┌────────▼───┐  ┌────▼─────┐ │
│              │   Text    │    │  ToolUse   │  │ Thinking  │ │
│              │  (stream) │    │  blocks    │  │ (stream)  │ │
│              └─────┬─────┘    └────────┬───┘  └────┬─────┘ │
│                    │                   │            │       │
│                    │           ┌───────▼────────┐   │       │
│                    │           │  Approval Gate │   │       │
│                    │           │ (Ask/Plan/Auto)│   │       │
│                    │           └───────┬────────┘   │       │
│                    │                   │            │       │
│                    │           ┌───────▼────────┐   │       │
│                    │           │  Tool.call()   │   │       │
│                    │           │  (with hooks)  │   │       │
│                    │           └───────┬────────┘   │       │
│                    │                   │            │       │
│                    │           ┌───────▼────────┐   │       │
│                    │           │  ToolResult    │   │       │
│                    │           │  (truncated)   │   │       │
│                    │           └───────┬────────┘   │       │
│                    │                   │            │       │
│                    │       ┌───────────▼─────────┐  │       │
│                    │       │ Injection Queue     │  │       │
│                    │       │ Drain (mid-turn     │  │       │
│                    │       │  user messages)     │  │       │
│                    │       └───────────┬─────────┘  │       │
│                    │                   │            │       │
│              ┌─────▼─────┐    ┌───────▼──────┐     │       │
│              │  Persist  │    │   Push to    │     │       │
│              │ Assistant │    │   History    │     │       │
│              │ Message   │    │  as User msg │     │       │
│              └───────────┘    └───────┬──────┘     │       │
│                                       │            │       │
│                              ┌────────▼────────┐   │       │
│                              │  Loop back to   │   │       │
│                              │  stream() OR    │   │       │
│                              │  yield Done     │   │       │
│                              └─────────────────┘   │       │
└──────────────────────────────────────────────────────────────┘
```

---

## Core Agent Struct

```rust
pub struct Agent {
    provider: Arc<dyn Provider>,           // LLM provider (10+ supported)
    tools: ToolRegistry,                    // ~30+ built-in + MCP tools
    model: String,                          // Active model name
    system: String,                         // System prompt
    budget_tokens: usize,                   // Context window (auto from catalogue)
    max_tokens: u32,                        // Output token cap (8192 default)
    max_iterations: usize,                  // Tool loop cap (200 default)
    max_retries: usize,                     // Transient error retries (3)
    thinking_budget: Option<u32>,           // Thinking token budget
    permission_mode: PermissionMode,        // Auto / Ask / Plan
    approver: Arc<dyn ApprovalSink>,        // Approval UI callback
    history: Arc<Mutex<Vec<Message>>>,      // Conversation history
    cancel: Option<CancelToken>,            // Cooperative cancellation
    hooks: Option<Arc<HooksConfig>>,        // Lifecycle event hooks
    origin: AgentOrigin,                    // Main / SideChannel / Subagent
    model_override: Arc<Mutex<Option<String>>>,  // Hot-swap model mid-turn
    next_turn_chunk_timeout: Arc<Mutex<Option<Duration>>>, // Long-running override
    injection_queue: Arc<Mutex<VecDeque<String>>>, // Mid-turn user messages
}
```

---

## Agent Loop — Step by Step

### Phase 1: Initialization
1. Push user message to `history` (behind `Mutex`)
2. Compose system prompt: base + plan reminder + todos reminder
3. Read `model_override` slot (skill may have swapped model)

### Phase 2: Iteration Loop (0..max_iterations)

**Step 2a: Compact History**
- Estimate system prompt tokens + reserve 1K for tool definitions
- Calculate `messages_budget = budget_tokens - system_tokens - tools_reserve`
- Call `compact(history, budget)` — only runs if over budget
- Fire `PreCompact` / `PostCompact` hooks if compaction happened

**Step 2b: Build Stream Request**
- Model = override or default
- Messages = compacted history
- Tools = registry definitions
- max_tokens = capped by model's actual max_output

**Step 2c: Provider Stream with Retry**
- Exponential backoff: 1s, 2s, 4s on transient errors
- Config errors (bad key, bad model) skip retry — won't fix themselves
- Backoff sleeps are cancel-aware (`tokio::select!` against cancel token)

**Step 2d: Assemble Events**
- Parse SSE stream into typed events:
  - `Text(String)` — streamed assistant text
  - `Thinking(String)` — reasoning content (DeepSeek, etc.)
  - `ToolUse { id, name, input }` — complete tool call
  - `ToolParseFailed { id, name, error }` — malformed tool JSON
  - `Done { stop_reason, usage }` — stream complete

**Step 2e: Persist Assistant Message**
- Thinking first, then text, then tool_uses
- Redact consumed images from history (prevent re-shipping base64)

**Step 2f: No Tool Uses → Done**
- Check if stop_reason == "max_tokens" → escalate output budget to 64000 and retry
- Clear model_override, chunk_timeout_override
- Yield `AgentEvent::Done`

**Step 2g: Execute Each Tool**
For each `ToolUse` block:

1. **Parse error check** — short-circuit with error result
2. **Unknown tool check** — error result
3. **Plan mode gate** — block mutating tools in Plan mode
4. **Approval window gate** — block UpdatePlanStep/ExitPlanMode while awaiting approval
5. **Approval gate** — if `Ask` mode and tool requires approval:
   - Send `ApprovalRequest` to approver
   - If denied → `ToolCallDenied` event + `permission_denied` hook
6. **Pre-tool hook** — `fire_pre_tool_use` (fire-and-forget)
7. **Execute tool** — `tool.call_multimodal(input).await`
8. **Truncate to disk** — if result > 50KB, save to temp file with preview
9. **Post-tool hook** — `fire_post_tool_use` / `fire_post_tool_failure`
10. **MCP widget fetch** — if tool supports UI resource, fetch iframe HTML
11. **Yield `ToolCallResult`**

**Step 2h: Drain Injection Queue**
- Pull any user messages typed mid-tool-execution
- Fold into same user message as tool results (keeps user/assistant alternation valid)
- Yield `UserMessageInjected` for each

**Step 2i: Push tool results as User message → Loop back to 2a**

### Phase 3: Iteration Cap Hit
- Yield `Done { stop_reason: "max_iterations" }`

---

## Key Design Patterns Worth Stealing

### 1. Event-Stream Architecture
The entire agent loop returns `impl Stream<Item = Result<AgentEvent>>`. The GUI/REPL/WebSocket all consume the same stream. This is elegant — one loop, all surfaces.

**For OpenWand:** We should adopt this pattern. The Dioxus UI subscribes to the same `AgentEvent` stream.

### 2. Injection Queue (Mid-Turn Steering)
Users can type messages while the agent is executing tools. These get folded into the next tool_result user message. This solves "the agent is doing X but I want to redirect it" without interrupting.

**For OpenWand:** Critical UX feature. Implement in our `session` crate.

### 3. Output Token Escalation
If the model hits `max_tokens` stop reason, escalate from 8192 → 64000 and retry the iteration (popping the partial assistant message). Smart — prevents truncated responses.

**For OpenWand:** Adopt this pattern.

### 4. Tool Result Truncation-to-Disk
Tool results > 50KB get saved to a temp file, with a 2000-char preview + file path in the message. Prevents context bloat.

**For OpenWand:** Adopt this pattern.

### 5. Image Redaction After Consumption
Once the model has "seen" an image in a tool result, strip the base64 from history on the next iteration. Prevents a single screenshot from inflating every subsequent turn.

**For OpenWand:** Adopt this pattern.

### 6. Plan Mode State Machine
Three states with strict gates:
- `(Plan, no plan)` → exploration only (Read/Grep/Glob/Ls)
- `(Plan, plan submitted)` → model waits, no tools allowed
- `(not-Plan, plan exists)` → execution mode, focused on one step at a time

**For OpenWand:** This is a complete plan-mode specification. Adopt the state machine.

### 7. Model Hot-Swap Mid-Turn
`model_override` slot lets skills swap the active model between iterations. Cleared at turn end.

**For OpenWand:** Useful for our skills system.

### 8. Cooperative Cancellation
Cancel token is threaded through:
- Retry backoff sleeps (`tokio::select!`)
- Subagent collection loops
- Side-channel spawns

**For OpenWand:** Essential for user control.

### 9. Layer-2 Focused Execution (Plan Steps)
When executing a plan, the model sees:
- Current step: title + full description
- Remaining steps: titles ONLY (no descriptions)
- Completed steps: single comma-separated line

This prevents the model from "reasoning ahead" or jumping steps.

**For OpenWand:** Brilliant UX pattern. Borrow exactly.

### 10. Cross-Step Output Channel (M6.3)
Completed steps can store an `output` field that gets surfaced to the model on subsequent steps, even after history compaction has removed the original context.

**For OpenWand:** Critical for multi-step workflows where step N+1 depends on step N's output.

---

## What thClaws Gets WRONG (OpenWand Advantages)

| Aspect | thClaws | OpenWand (Planned) |
|---|---|---|
| **Session model** | Append-only JSONL, no branching | Loro CRDT with branching + merge |
| **Memory** | Flat markdown files | Temporal knowledge graph (CozoDB) |
| **Context store** | In-memory `Arc<Mutex<Vec<Message>>>` | CRDT-backed persistent store |
| **Concurrency** | `Arc<Mutex>` everywhere | CRDT concurrent edits (no locks) |
| **GUI** | Tauri/webview | Dioxus native |
| **Architecture** | Monolithic 111K LOC crate | Multi-crate workspace (11 crates) |
| **Query power** | Grep/read for KMS | Datalog queries on knowledge graph |
| **Time travel** | None | Both sessions (Loro) and KG (CozoDB) |

---

## Exact API Surface for OpenWand's Agent Loop

Based on thClaws analysis, here's what OpenWand should implement:

```rust
// ow-session/src/agent.rs

pub enum AgentEvent {
    IterationStart { iteration: usize },
    Text(String),
    Thinking(String),
    ToolCallStart { id: String, name: String, input: Value },
    ToolCallResult { id: String, name: String, output: Result<String, String> },
    ToolCallDenied { id: String, name: String },
    UserMessageInjected { text: String },
    PlanStepTransition { step_id: String, status: StepStatus },
    Done { stop_reason: Option<String>, usage: Usage },
}

pub struct Agent {
    // Provider (via Rig)
    provider: Arc<dyn rig::Provider>,
    // Tool registry
    tools: ToolRegistry,
    // Loro CRDT document (replaces Arc<Mutex<Vec<Message>>>)
    session_doc: LoroDoc,
    // Model config
    model: String,
    system: String,
    budget_tokens: usize,
    max_tokens: u32,
    max_iterations: usize,
    // Permission policy (with WASM sandbox)
    policy: PolicyEngine,
    // Cancel token
    cancel: CancelToken,
    // Hooks
    hooks: HooksConfig,
    // Mid-turn injection queue
    injection_queue: Arc<Mutex<VecDeque<String>>>,
}

impl Agent {
    pub fn run_turn(&self, user_msg: String) 
        -> impl Stream<Item = Result<AgentEvent>> + Send + 'static 
    { ... }
}
```

**Key differences from thClaws:**
1. `LoroDoc` replaces `Arc<Mutex<Vec<Message>>>` — CRDT, no locks
2. `PolicyEngine` includes WASM sandbox from ironclaw patterns
3. `rig::Provider` replaces custom provider trait
4. `PlanStepTransition` event for plan mode (more granular)
