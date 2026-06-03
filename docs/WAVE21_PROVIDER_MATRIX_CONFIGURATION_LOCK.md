# Wave 21 — Provider Matrix and Model Configuration Hardening — LOCK

**Committed:** Wave 21 batch commit
**Baseline:** 1555 tests (Wave 20 locked)
**Final:** 1613 tests (+58), zero failures

---

## What Shipped

### Provider Configuration System
- **`ProviderTargetConfig`** — serializable config with `id`, `provider_kind`, `model`, `endpoint`, `api_key_source`, capability flags, thinking budget
- **`ApiKeySource`** — `None` or `EnvVar { name }` — resolved at build time, never serialized
- **`validate_provider_config()`** — model non-empty, endpoint required for local, timeout bounds (1s–600s), env var name non-empty
- **TOML persistence** — `load_provider_configs(path)` reads `.openwand/providers.toml` with `[[target]]` entries
- **Wave 21 reads/validates only** — no config write path from UI

### Provider Registry
- **`ProviderRegistry`** — builds `Arc<dyn LlmClient>` from validated configs
- Dispatches on `ProviderKind`: `OpenAiCompatible`, `AnthropicCompatible`, `LocalOpenAiCompatible`, `Mock`
- API key resolution at build time from env vars
- Disabled targets excluded from `list_available_targets()`
- `build_target()` returns `LlmTarget` for session runner

### Anthropic Adapter
- **`AnthropicCompatibleClient`** — implements `LlmClient` trait
- Anthropic Messages API format: content blocks, tool_use, thinking
- SSE parsing: `message_start` → `content_block_start` → `content_block_delta` → `content_block_stop` → `message_delta` → `message_stop`
- Delta types: `text_delta`, `input_json_delta`, `thinking_delta`, `signature_delta`
- Auth: `x-api-key` header + `anthropic-version` header
- Error sanitization removes `sk-ant-` prefixed tokens

### OpenAI-Compatible Regression (Patch 2)
- 12 tests locking OpenAI adapter behavior: text delta, tool call buffering, malformed JSON rejection, usage metadata, cancellation, rate limit error, connection refused, local provider capability flags

### Secret Redaction (Patch 4)
Five surfaces locked:
1. `ProviderTargetConfig` Debug → `"***REDACTED***"`
2. `ProviderTargetSummary` → env var names only, never resolved values
3. Validation errors → field names only, never raw key values
4. Adapter errors → `sanitize_error_message()` strips API keys
5. UI rows → `api_key_display` is `"env:VAR_NAME"` or `"none"`

### Session Integration
- `build_session_runtime_with_provider()` — builds session from provider configs + target ID
- Provider registry output compatible with `Arc<dyn LlmClient>` expected by `SessionRunner`
- Tool calls still route through policy engine and tool executor

### UI Provider Config
- `provider_target_rows()` — builds display rows from summaries
- `provider_validation_lines()` — safe error display
- `provider_config_safety_warning()` — invariant text
- Desktop-gated render functions in `provider_components.rs`

### Smoke Tests
- Feature-gated: `real-provider-smoke` feature
- Three `#[ignore]` tests: OpenAI, Anthropic, Local
- Skip without env vars: `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `LOCAL_LLM_URL`
- CI never runs these

---

## Test Breakdown

| Area | Count | Feature gate |
|------|------:|:-------------|
| Provider config/validation/TOML/redaction | 15 | default |
| Provider registry | 7 | default (mock test behind `testing`) |
| Anthropic adapter | 5 | `anthropic-compatible` |
| OpenAI regression + local + secret guards | 12 | `openai-compatible` |
| Session provider integration | 6 | default |
| UI provider config + guards | 6 | default |
| Smoke skip (ignored) | 3 | `real-provider-smoke` |

**Default workspace run: 1555 → 1613 (+58)**

---

## New Files

| File | Purpose |
|------|---------|
| `crates/llm/src/provider_config.rs` | Config DTOs, validation, TOML loading, secret redaction |
| `crates/llm/src/provider_registry.rs` | Build clients from config |
| `crates/llm/src/adapters/anthropic_compatible.rs` | Anthropic Messages API adapter |
| `crates/llm/tests/openai_regression.rs` | OpenAI adapter regression tests |
| `crates/llm/tests/provider_smoke.rs` | Feature-gated real provider smoke tests |
| `crates/app/src/ui/provider_config.rs` | UI view helpers |
| `crates/app/src/ui/provider_components.rs` | Desktop-gated Dioxus render |
| `crates/app/tests/session_provider_integration.rs` | Session + provider wiring tests |

## Modified Files

| File | Change |
|------|--------|
| `crates/llm/Cargo.toml` | +toml dep, +anthropic-compatible feature, +real-provider-smoke feature |
| `crates/llm/src/lib.rs` | +provider_config, +provider_registry modules |
| `crates/llm/src/adapters/mod.rs` | +anthropic_compatible module |
| `crates/llm/src/adapters/openai_compatible.rs` | pub build_request_body |
| `crates/app/Cargo.toml` | +testing feature on openwand-llm dep |
| `crates/app/src/session_runtime.rs` | +build_session_runtime_with_provider() |
| `crates/app/src/ui/mod.rs` | +provider_config, +provider_components modules |

---

## TOML Config Format

```toml
[[target]]
id = "local-lmstudio"
provider_kind = "local_open_ai_compatible"
display_name = "LM Studio"
endpoint = "http://localhost:1234/v1"
model = "local-model"
api_key_source = { type = "none" }
timeout_ms = 60000
supports_tools = false
supports_streaming = true
supports_usage = false
enabled = true

[[target]]
id = "openai-main"
provider_kind = "open_ai_compatible"
display_name = "OpenAI"
model = "gpt-4.1-mini"
api_key_source = { type = "env_var", name = "OPENAI_API_KEY" }
timeout_ms = 60000
supports_tools = true
supports_streaming = true
supports_usage = true
enabled = true

[[target]]
id = "anthropic-main"
provider_kind = "anthropic_compatible"
display_name = "Anthropic"
model = "claude-sonnet-4-20250514"
api_key_source = { type = "env_var", name = "ANTHROPIC_API_KEY" }
timeout_ms = 120000
supports_tools = true
supports_streaming = true
supports_usage = true
supports_reasoning = true
enabled = true
```

---

## Central Invariant

```
Provider adapters stream model output.
SessionRunner owns the loop.
Policy gates tools.
ToolExecutor executes tools.
Trace records authority.
Providers never execute tools or mutate state directly.
```

---

## Honest Caveats

- No provider retry/circuit-breaker beyond existing `retryable()` flag
- Anthropic adapter handles current Messages API format; future changes may require updates
- Provider quality parity not guaranteed — different models produce different results
- UI provider config is view helpers only, not full form submission UX
- Secret management limited to env var references; no vault/keychain integration
- No provider benchmarking or quality comparison in this wave
- `build_session_runtime_with_provider()` added but not yet wired to CLI `run` command
