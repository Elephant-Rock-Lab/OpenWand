# OpenWand LLM Crate Design

**Date:** 2026-05-26  
**Status:** Design — locked  
**Crate:** `openwand-llm`  
**Depends on:** `openwand-core`, `rig-core`  
**Blocks:** Batch 1 inference  

---

## North Star

> Rig speaks to providers. OpenWand decides what provider output is allowed to become.

`openwand-llm` is a **provider-normalization crate**. It wraps Rig behind OpenWand's own trait. No Rig types escape this crate.

```
openwand-session
  → openwand-llm::LlmClient
      → Rig CompletionModel
          → OpenAI / Anthropic / Ollama / ...
```

---

## Crate Boundary

### Contains

- `LlmClient` trait (object-safe)
- `LlmRequest`, `LlmMessage`, `LlmToolDef`, `LlmTarget` — request DTOs
- `LlmDelta`, `LlmResponse`, `LlmStopReason` — response DTOs
- `LlmError` — normalized error types
- Rig adapter: `RigLlmClient` implementing `LlmClient`
- Provider enum dispatch (no dyn on Rig's CompletionModel — it's not object-safe)
- Tool-call buffering: accumulate deltas → emit `ToolCallComplete`
- Reasoning normalization: text/redacted/encrypted → `Reasoning { delta, redacted }`
- Thinking budget mapping: `ThinkingBudgetSnapshot` → provider-specific params
- Circuit breaker, retry logic
- `Usage` → `TokenUsageSnapshot` conversion

### Does NOT contain

- Agent loop (session owns this)
- Tool execution (tools/mcp-pool own this)
- Policy evaluation (policy owns this)
- MCP server management (mcp-pool owns this)
- Memory integration (memory owns this)
- Trace recording (session owns this)
- Conversation memory (Loro + trace own this)
- Vector store / embeddings (store + memory own this)

### Depends on

```
openwand-core    — TokenUsageSnapshot, ThinkingBudgetSnapshot, IDs
rig-core         — provider clients, CompletionModel, streaming
```

### Does NOT depend on

```
openwand-session, openwand-trace, openwand-memory,
openwand-policy, openwand-tools, openwand-mcp-pool,
openwand-store, loro, rmcp
```

---

## Crate Layout

```
openwand-llm/
  Cargo.toml
  src/
    lib.rs
    client.rs            — LlmClient trait
    request.rs           — LlmRequest, LlmTarget, LlmMessage, LlmToolDef, LlmContent
    response.rs          — LlmDelta, LlmResponse, LlmStopReason
    error.rs             — LlmError
    tool_buffer.rs       — accumulate ToolCallStart+Delta → ToolCallComplete
    reasoning.rs         — normalize Rig Reasoning → LlmDelta::Reasoning
    thinking_budget.rs   — ThinkingBudgetSnapshot → provider-specific additional_params
    usage.rs             — Rig Usage → TokenUsageSnapshot

    rig_adapter/
      mod.rs
      client.rs          — RigLlmClient (the main implementation)
      dispatch.rs        — RigProviderModel enum dispatch
      convert_request.rs — LlmRequest → Rig CompletionRequest
      convert_stream.rs  — Rig StreamingCompletionResponse → LlmStream
      convert_response.rs— Rig CompletionResponse → LlmResponse

  tests/
    conformance.rs       — every enabled provider must pass these
    stream_text.rs
    stream_tool_call.rs
    stream_reasoning.rs
    error_mapping.rs
    tool_buffer.rs
```

---

## Dependencies

```toml
[package]
name = "openwand-llm"
version.workspace = true
edition.workspace = true

[dependencies]
openwand-core = { path = "../core" }
rig-core = { version = "0.37", default-features = false, features = ["rustls"] }

async-trait = { workspace = true }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
tokio = { workspace = true, features = ["sync", "macros", "time"] }
tracing = { workspace = true }
thiserror = { workspace = true }
chrono = { workspace = true, features = ["serde"] }
async-stream = "0.3"
futures = "0.3"

[features]
default = ["openai", "anthropic", "ollama"]
openai = ["rig-core/openai"]
anthropic = ["rig-core/anthropic"]
ollama = ["rig-core/ollama"]
gemini = ["rig-core/gemini"]
openrouter = []
groq = ["rig-core/groq"]
xai = []
deepseek = []
```

Pin Rig exactly for Batch 1: `rig-core = "=0.37.0"`. Rig is moving fast; treat it as an integration dependency with conformance tests, not a stable internal architecture.

---

## Core Trait

```rust
// client.rs

/// OpenWand's LLM client trait. Object-safe.
/// The only LLM abstraction that leaves this crate.
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Stream a completion request.
    /// Returns a stream of LlmDelta items.
    async fn chat_stream(
        &self,
        request: LlmRequest,
    ) -> Result<LlmStream, LlmError>;

    /// Non-streaming completion. For tests and fallback.
    async fn complete(
        &self,
        request: LlmRequest,
    ) -> Result<LlmResponse, LlmError>;

    /// Check that a specific provider target is reachable.
    async fn health_check(
        &self,
        target: &LlmTarget,
    ) -> Result<(), LlmError>;

    /// Report capabilities for a given target.
    fn capabilities(&self, target: &LlmTarget) -> LlmCapabilities;
}

/// Stream of LLM deltas. All errors go through Result Err, never through a delta variant.
pub type LlmStream = Pin<Box<dyn Stream<Item = Result<LlmDelta, LlmError>> + Send>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCapabilities {
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub supports_reasoning: bool,
    pub supports_vision: bool,
    pub max_context_tokens: Option<u64>,
    pub supported_features: Vec<String>,
}
```

---

## Request Types

### LlmRequest

```rust
// request.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    /// Which provider and model to use.
    /// This is a session-level decision, not hidden client state.
    pub target: LlmTarget,

    /// System prompt — separate from messages for trace/debug reproducibility.
    /// PromptAssemblySnapshot hashes this independently.
    pub system_prompt: String,

    /// Conversation history (no system messages — those go in system_prompt).
    pub messages: Vec<LlmMessage>,

    /// Tool definitions visible to the model.
    pub tools: Vec<LlmToolDef>,

    /// Thinking/reasoning budget.
    pub thinking_budget: Option<ThinkingBudgetSnapshot>,

    /// Maximum response tokens.
    pub max_tokens: Option<u64>,

    /// Sampling temperature.
    pub temperature: Option<f64>,

    /// Whether the model should use tools.
    pub tool_choice: Option<LlmToolChoice>,

    /// Provider-specific escape hatch.
    /// Example: Anthropic thinking params, OpenAI reasoning params.
    pub provider_options: serde_json::Value,
}
```

### LlmTarget

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LlmTarget {
    pub provider: LlmProvider,
    pub model: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LlmProvider {
    OpenAI,
    Anthropic,
    Ollama,
    OpenRouter,
    Gemini,
    Groq,
    XAI,
    DeepSeek,
    Custom { name: String },
}
```

### LlmMessage

No `System` variant — system prompt is a separate field on `LlmRequest`.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmMessage {
    User { content: Vec<LlmContent> },
    Assistant { content: Vec<LlmContent> },
    Tool { tool_call_id: String, content: String, is_error: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmContent {
    Text(String),
    Reasoning(String),
    ToolCall {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },
}
```

### LlmToolDef

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmToolDef {
    pub name: String,
    pub description: String,
    pub parameters_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmToolChoice {
    Auto,
    None,
    Required,
}
```

---

## Response Types

### LlmDelta

```rust
// response.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmDelta {
    /// Streaming text content delta.
    Text {
        delta: String,
    },

    /// Streaming reasoning/thinking delta.
    Reasoning {
        delta: String,
        /// True if the reasoning content was redacted by the provider.
        redacted: bool,
    },

    /// A tool call has started. Name may arrive late.
    ToolCallStart {
        id: String,
        name: Option<String>,
    },

    /// Partial JSON argument data for a tool call.
    /// Buffer these — ToolGate cannot evaluate partial JSON.
    ToolCallArgsDelta {
        id: String,
        delta: String,
    },

    /// A complete tool call with full arguments.
    /// This is what openwand-session converts into a pending ToolCall
    /// and routes through openwand-policy.
    ToolCallComplete {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },

    /// Stream completed.
    Done {
        stop_reason: LlmStopReason,
        usage: Option<TokenUsageSnapshot>,
        /// Provider-assigned message ID (e.g. OpenAI msg_ ID).
        provider_message_id: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmStopReason {
    Stop,
    ToolCall,
    Length,
    ContentFilter,
}
```

### LlmResponse (non-streaming)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: Vec<LlmContent>,
    pub usage: TokenUsageSnapshot,
    pub stop_reason: LlmStopReason,
    pub provider_message_id: Option<String>,
}
```

---

## Error Types

```rust
// error.rs

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    /// Network failure. retryable indicates whether retry is safe.
    #[error("Network error: {message}")]
    Network {
        message: String,
        retryable: bool,
    },

    /// Provider returned an error (rate limit, content filter, etc.)
    #[error("Provider error ({provider}): {message}")]
    Provider {
        provider: String,
        message: String,
        retryable: bool,
    },

    /// The request was invalid (bad model name, missing required field, etc.)
    #[error("Invalid request: {message}")]
    RequestInvalid {
        message: String,
    },

    /// Failed to decode provider response.
    #[error("Decode error: {message}")]
    Decode {
        message: String,
    },

    /// Stream error during delivery. partial=true means some deltas were delivered.
    #[error("Stream error: {message}")]
    Stream {
        message: String,
        partial: bool,
    },

    /// Request was cancelled (by user or circuit breaker).
    #[error("Cancelled")]
    Cancelled,

    /// Feature not supported by this provider.
    #[error("Unsupported: {provider} does not support {feature}")]
    Unsupported {
        provider: String,
        feature: String,
    },
}
```

Session failure taxonomy: LLM failure is **recoverable/retryable**. Unlike trace append failure, which is a hard stop.

---

## Tool-Call Buffering

Tool-call deltas arrive in fragments. ToolGate needs complete calls. The adapter buffers:

```rust
// tool_buffer.rs

pub struct ToolCallBuffer {
    calls: HashMap<String, BufferedToolCall>,
}

struct BufferedToolCall {
    id: String,
    name: Option<String>,
    args_chunks: Vec<String>,
}

impl ToolCallBuffer {
    pub fn new() -> Self { ... }

    pub fn handle_start(&mut self, id: String, name: Option<String>) {
        self.calls.insert(id.clone(), BufferedToolCall {
            id,
            name,
            args_chunks: Vec::new(),
        });
    }

    pub fn handle_args_delta(&mut self, id: &str, delta: String) {
        if let Some(call) = self.calls.get_mut(id) {
            call.args_chunks.push(delta);
        }
    }

    /// Try to finalize a tool call. Returns None if args are incomplete.
    pub fn try_complete(&mut self, id: &str) -> Option<ToolCallCompleteResult> {
        let call = self.calls.remove(id)?;
        let name = call.name?;
        let args_json: String = call.args_chunks.into_iter().collect();
        let arguments = serde_json::from_str(&args_json).ok()?;
        Some(ToolCallCompleteResult { id: call.id, name, arguments })
    }

    /// Finalize all remaining tool calls at stream end.
    pub fn drain_remaining(&mut self) -> Vec<ToolCallCompleteResult> { ... }
}
```

---

## Thinking Budget Mapping

Different providers handle reasoning/thinking differently:

```rust
// thinking_budget.rs

pub fn thinking_budget_params(
    provider: &LlmProvider,
    budget: &ThinkingBudgetSnapshot,
) -> serde_json::Value {
    match provider {
        LlmProvider::Anthropic => {
            match budget {
                ThinkingBudgetSnapshot::Off => serde_json::json!({
                    "thinking": { "type": "disabled" }
                }),
                ThinkingBudgetSnapshot::Tokens(n) => serde_json::json!({
                    "thinking": { "type": "enabled", "budget_tokens": n }
                }),
                _ => {
                    let tokens = budget_to_tokens(budget);
                    serde_json::json!({
                        "thinking": { "type": "enabled", "budget_tokens": tokens }
                    })
                }
            }
        }
        LlmProvider::OpenAI | LlmProvider::OpenRouter => {
            match budget {
                ThinkingBudgetSnapshot::Off => serde_json::json!({}),
                _ => {
                    let effort = budget_to_effort(budget);
                    serde_json::json!({ "reasoning": { "effort": effort } })
                }
            }
        }
        // Other providers: pass through in provider_options
        _ => serde_json::json!({}),
    }
}

fn budget_to_tokens(budget: &ThinkingBudgetSnapshot) -> u32 {
    match budget {
        ThinkingBudgetSnapshot::Low => 4096,
        ThinkingBudgetSnapshot::Medium => 16384,
        ThinkingBudgetSnapshot::High => 32768,
        ThinkingBudgetSnapshot::Max => 65536,
        ThinkingBudgetSnapshot::Tokens(n) => *n,
        ThinkingBudgetSnapshot::Off => 0,
    }
}

fn budget_to_effort(budget: &ThinkingBudgetSnapshot) -> &'static str {
    match budget {
        ThinkingBudgetSnapshot::Low => "low",
        ThinkingBudgetSnapshot::Medium => "medium",
        ThinkingBudgetSnapshot::High | ThinkingBudgetSnapshot::Max => "high",
        _ => "medium",
    }
}
```

---

## Usage Conversion

```rust
// usage.rs

impl From<rig_core::completion::Usage> for TokenUsageSnapshot {
    fn from(usage: rig_core::completion::Usage) -> Self {
        Self {
            input: usage.input_tokens,
            output: usage.output_tokens,
            reasoning: if usage.reasoning_tokens > 0 {
                Some(usage.reasoning_tokens)
            } else {
                None
            },
            cache_read: if usage.cached_input_tokens > 0 {
                Some(usage.cached_input_tokens)
            } else {
                None
            },
            cache_write: if usage.cache_creation_input_tokens > 0 {
                Some(usage.cache_creation_input_tokens)
            } else {
                None
            },
        }
    }
}
```

---

## Rig Adapter

### Enum Dispatch

Rig's `CompletionModel` is not object-safe (associated types). Use enum dispatch:

```rust
// rig_adapter/dispatch.rs

pub enum RigProviderModel {
    #[cfg(feature = "openai")]
    OpenAi(rig_core::providers::openai::CompletionModel),

    #[cfg(feature = "anthropic")]
    Anthropic(rig_core::providers::anthropic::CompletionModel),

    #[cfg(feature = "ollama")]
    Ollama(rig_core::providers::ollama::CompletionModel),

    // Future providers added as needed
}
```

### Client

```rust
// rig_adapter/client.rs

pub struct RigLlmClient {
    providers: RwLock<HashMap<LlmProvider, ProviderHandle>>,
    circuit_breaker: CircuitBreaker,
}

struct ProviderHandle {
    client: Box<dyn ProviderClientAccess>,
    base_url: Option<String>,
}

#[async_trait]
impl LlmClient for RigLlmClient {
    async fn chat_stream(
        &self,
        request: LlmRequest,
    ) -> Result<LlmStream, LlmError> {
        // 1. Circuit breaker check
        self.circuit_breaker.check(&request.target.provider)?;

        // 2. Get or create provider model
        let model = self.get_or_create_model(&request.target)?;

        // 3. Convert OpenWand request → Rig request
        let rig_request = convert_request(&request)?;

        // 4. Call Rig streaming
        let rig_stream = model.stream(rig_request).await
            .map_err(|e| self.classify_error(e, &request.target.provider))?;

        // 5. Convert Rig stream → LlmStream with buffering
        let provider = request.target.provider.clone();
        let stream = convert_stream(rig_stream, provider);

        Ok(stream)
    }

    async fn complete(
        &self,
        request: LlmRequest,
    ) -> Result<LlmResponse, LlmError> {
        self.circuit_breaker.check(&request.target.provider)?;

        let model = self.get_or_create_model(&request.target)?;
        let rig_request = convert_request(&request)?;

        let response = model.completion(rig_request).await
            .map_err(|e| self.classify_error(e, &request.target.provider))?;

        convert_response(response)
    }

    async fn health_check(
        &self,
        target: &LlmTarget,
    ) -> Result<(), LlmError> {
        // Attempt a minimal completion
        let model = self.get_or_create_model(target)?;
        let request = model.completion_request("ping").max_tokens(1).build();
        let _ = model.completion(request).await
            .map_err(|e| LlmError::Provider {
                provider: target.provider_name(),
                message: e.to_string(),
                retryable: true,
            })?;
        Ok(())
    }

    fn capabilities(&self, target: &LlmTarget) -> LlmCapabilities {
        match &target.provider {
            LlmProvider::OpenAI => LlmCapabilities {
                supports_streaming: true,
                supports_tools: true,
                supports_reasoning: true,
                supports_vision: true,
                max_context_tokens: Some(128000),
                supported_features: vec!["structured_output".into(), "reasoning".into()],
            },
            LlmProvider::Anthropic => LlmCapabilities {
                supports_streaming: true,
                supports_tools: true,
                supports_reasoning: true,
                supports_vision: true,
                max_context_tokens: Some(200000),
                supported_features: vec!["extended_thinking".into()],
            },
            LlmProvider::Ollama => LlmCapabilities {
                supports_streaming: true,
                supports_tools: true,
                supports_reasoning: false,
                supports_vision: false,
                max_context_tokens: None, // model-dependent
                supported_features: vec![],
            },
            _ => LlmCapabilities::default(),
        }
    }
}
```

### Stream Conversion

```rust
// rig_adapter/convert_stream.rs

pub fn convert_stream<R>(
    mut rig_stream: StreamingCompletionResponse<R>,
    provider: LlmProvider,
) -> LlmStream
where
    R: Clone + Unpin + GetTokenUsage + Send + 'static,
{
    let mut tool_buffer = ToolCallBuffer::new();

    Box::pin(async_stream::stream! {
        while let Some(chunk) = rig_stream.next().await {
            match chunk {
                Ok(StreamedAssistantContent::Text(text)) => {
                    yield Ok(LlmDelta::Text { delta: text.text });
                }

                Ok(StreamedAssistantContent::Reasoning(reasoning)) => {
                    let redacted = reasoning.content.iter().any(|c| {
                        matches!(c, ReasoningContent::Redacted { .. })
                    });
                    yield Ok(LlmDelta::Reasoning {
                        delta: reasoning.display_text(),
                        redacted,
                    });
                }

                Ok(StreamedAssistantContent::ToolCallDelta { id, content, .. }) => {
                    match content {
                        ToolCallDeltaContent::Name(name) => {
                            tool_buffer.handle_start(id, Some(name));
                        }
                        ToolCallDeltaContent::Delta(delta) => {
                            tool_buffer.handle_args_delta(&id, delta);
                        }
                    }
                }

                Ok(StreamedAssistantContent::ToolCall { tool_call, .. }) => {
                    // Complete tool call received in one shot
                    yield Ok(LlmDelta::ToolCallComplete {
                        id: tool_call.id,
                        name: tool_call.function.name,
                        arguments: tool_call.function.arguments,
                    });
                }

                Ok(StreamedAssistantContent::Final(response)) => {
                    // Drain any remaining buffered tool calls
                    for complete in tool_buffer.drain_remaining() {
                        yield Ok(LlmDelta::ToolCallComplete {
                            id: complete.id,
                            name: complete.name,
                            arguments: complete.arguments,
                        });
                    }

                    let usage = response.token_usage().map(TokenUsageSnapshot::from);
                    yield Ok(LlmDelta::Done {
                        stop_reason: LlmStopReason::Stop,
                        usage,
                        provider_message_id: None,
                    });
                }

                Err(e) => {
                    yield Err(classify_stream_error(e, &provider));
                }

                _ => {} // ReasoningDelta handled via buffering if needed
            }
        }
    })
}
```

---

## Circuit Breaker

Simple circuit breaker per provider. Not the full CC Switch implementation, but the same principle:

```rust
pub struct CircuitBreaker {
    states: RwLock<HashMap<LlmProvider, CircuitState>>,
}

struct CircuitState {
    status: CircuitStatus,
    failure_count: u32,
    last_failure: Option<Instant>,
    half_open_at: Option<Instant>,
}

enum CircuitStatus {
    Closed,     // normal
    Open,       // blocking requests
    HalfOpen,   // allowing one probe request
}
```

Rules:
- 5 consecutive failures → Open (block requests for 30s)
- After 30s → HalfOpen (allow one request)
- Success → Closed
- Any failure in HalfOpen → Open

---

## Provider Access Trait (Internal)

```rust
// Internal trait for creating Rig models without exposing Rig types
trait ProviderClientAccess: Send + Sync {
    fn create_model(&self, model: &str) -> RigProviderModel;
    fn provider(&self) -> LlmProvider;
}

#[cfg(feature = "openai")]
impl ProviderClientAccess for rig_core::providers::openai::Client {
    fn create_model(&self, model: &str) -> RigProviderModel {
        RigProviderModel::OpenAi(self.completion_model(model))
    }
    fn provider(&self) -> LlmProvider { LlmProvider::OpenAI }
}

// Similar for anthropic, ollama, etc.
```

---

## Session Integration

Session calls `LlmClient` through the trait. Never sees Rig types:

```rust
// In openwand-session (NOT in this crate)

let request = LlmRequest {
    target: LlmTarget {
        provider: self.config.provider(),
        model: self.config.model(),
        base_url: self.config.base_url(),
        api_key: self.config.api_key(),
    },
    system_prompt,
    messages: self.build_llm_messages(),
    tools: self.build_tool_defs(),
    thinking_budget: self.config.thinking_budget,
    max_tokens: self.config.max_tokens,
    temperature: None,
    tool_choice: None,
    provider_options: serde_json::Value::Null,
};

let stream = self.llm.chat_stream(request).await?;

// Stream produces LlmDelta items:
//   Text { delta }          → emit AgentEvent::TextDelta
//   Reasoning { delta }     → emit AgentEvent::ReasoningDelta
//   ToolCallComplete { .. } → add to pending_tool_calls
//   Done { stop_reason }    → end of inference
```

---

## Conformance Tests

Every enabled provider must pass the same tests:

```rust
#[tokio::test]
async fn stream_text() {
    let client = test_client();
    let request = text_request("Say hello.");
    let mut stream = client.chat_stream(request).await.unwrap();

    let mut text = String::new();
    while let Some(delta) = stream.next().await {
        if let Ok(LlmDelta::Text { delta }) = delta {
            text.push_str(&delta);
        }
    }
    assert!(!text.is_empty());
}

#[tokio::test]
async fn stream_tool_call() {
    let client = test_client();
    let request = tool_call_request("What is 2+2?", vec![calculator_tool_def()]);
    let mut stream = client.chat_stream(request).await.unwrap();

    let mut tool_calls = vec![];
    while let Some(delta) = stream.next().await {
        match delta {
            Ok(LlmDelta::ToolCallComplete { id, name, arguments }) => {
                tool_calls.push((id, name, arguments));
            }
            Ok(LlmDelta::Done { .. }) => break,
            _ => {}
        }
    }
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].1, "calculator");
}

#[tokio::test]
async fn error_is_normalized() {
    let client = test_client_with_bad_key();
    let request = text_request("test");
    let result = client.chat_stream(request).await;
    assert!(matches!(result, Err(LlmError::Provider { .. })));
}

#[tokio::test]
async fn no_rig_types_leak() {
    // Verify that the public API of openwand-llm
    // contains no types from the rig crate
    // (compile-time check via trait bounds)
}
```

---

## Batch 1 Scope

| Aspect | Batch 1 | Later |
|---|---|---|
| Providers | OpenAI + Anthropic + Ollama | Gemini, OpenRouter, Groq, xAI, DeepSeek, custom |
| Modes | Streaming + non-streaming fallback | Same |
| Tools | Schema exposed, never executed inside LLM crate | Same |
| Events | Text, Reasoning, ToolCallStart, ToolCallArgsDelta, ToolCallComplete, Done | Same |
| Errors | Normalized LlmError with retryable flag | Same |
| Circuit breaker | Per-provider, simple | Per-provider, CC Switch patterns |
| Retry | Basic (3 retries, exponential backoff) | Configurable, with fallback routing |
| Thinking budget | Anthropic extended thinking + OpenAI reasoning | Per-provider mapping |
| Provider options | `serde_json::Value` escape hatch | Typed per-provider options |

---

## Summary

| Decision | Locked |
|---|---|
| Use Rig CompletionModel directly, not Agent/Chat/Prompt | ✅ |
| Hide all Rig types behind OpenWand DTOs | ✅ |
| Enum dispatch (not dyn) because CompletionModel is not object-safe | ✅ |
| System prompt is a separate field, not in LlmMessage | ✅ |
| ToolCallComplete for buffered tool calls (no partial JSON to ToolGate) | ✅ |
| No Error variant in LlmDelta — only `Err(LlmError)` | ✅ |
| No mutable swap_model on trait — target in request | ✅ |
| No MCP in this crate | ✅ |
| Pin Rig version, conformance-test every provider | ✅ |
| Batch 1: OpenAI + Anthropic + Ollama only | ✅ |
| Circuit breaker per provider | ✅ |

**Estimated LOC:** ~1,900 (trait + DTOs ~500, Rig adapter ~600, buffering/conversion ~400, circuit breaker ~200, tests ~400)
