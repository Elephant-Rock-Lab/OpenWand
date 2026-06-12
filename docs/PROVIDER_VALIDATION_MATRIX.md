# Multi-Provider Validation Matrix — v0.1.0-alpha

**Date:** 2026-06-12
**OpenWand version:** v0.1.0-alpha (`967dc96`)
**Test harness:** `crates/session/tests/real_provider_validation.rs` (4 tests, `#[ignore]`)

---

## Matrix Results

| Provider Family | Model | Endpoint | Auth | Simple Turn | Trace Attribution | Tool Use | Sandbox Refuse | Overall |
|----------------|-------|----------|------|:-----------:|:-----------------:|:--------:|:--------------:|:-------:|
| LM Studio | google/gemma-4-12b (12B, Q4_K_M) | localhost:8766 | local/none | ✅ PASS | ✅ PASS | ✅ PASS | ✅ PASS | ✅ PASS |
| LM Studio | bartowski/qwen2.5-0.5b-instruct (0.5B, Q8_0) | localhost:8766 | local/none | ✅ PASS | ✅ PASS | ✅ PASS | ✅ PASS | ✅ PASS |
| OpenAI API | gpt-4o / gpt-4o-mini | api.openai.com | API key | ⬜ SKIP | ⬜ SKIP | ⬜ SKIP | ⬜ SKIP | ⬜ SKIP |
| Anthropic | claude-sonnet-4 | api.anthropic.com | API key | ⬜ SKIP | ⬜ SKIP | ⬜ SKIP | ⬜ SKIP | ⬜ SKIP |
| Ollama | (various) | localhost:11434 | none | ⬜ SKIP | ⬜ SKIP | ⬜ SKIP | ⬜ SKIP | ⬜ SKIP |

### Result Legend

| Symbol | Meaning |
|--------|---------|
| ✅ PASS | Test executed and passed |
| ❌ FAIL | Test executed and failed |
| ⬜ SKIP | Provider not configured / no credentials |
| ⛔ UNSUPPORTED | Provider/model does not support required features |

---

## Test Details

### Test 1: `real_provider_completes_simple_turn`

Session reaches real LLM, sends a user message, receives a response, turn completes
with `Natural` stop reason. Trace contains inference events.

**What it proves:** The OpenAI-compatible adapter works end-to-end with the provider.
The session runner completes the full agent loop: inject message → build request →
stream response → record trace.

### Test 2: `real_provider_trace_records_attribution`

Trace contains inference events. Provider and model names are derived from
`RunConfig.llm_target` (not from the response).

**What it proves:** Trace identity derivation works with real providers. Attribution
matches `LlmTarget` configuration.

### Test 3: `real_provider_read_tool_works`

Session is given read-only tools (`batch1_local_tools`). The model MAY call a tool
or MAY respond directly. Either outcome is acceptable for small models.

**What it proves:** Tool definitions are sent correctly. The session handles tool-call
and no-tool-call responses from real models. Non-deterministic model behavior is
tolerated.

### Test 4: `real_provider_sandbox_refuses_escape`

User prompt asks the model to read `/etc/passwd`. The sandbox blocks traversal.
The session completes (the model's request is refused by the sandbox, not by the
model).

**What it proves:** Sandbox containment works under real inference. The model cannot
bypass path validation even when explicitly asked.

---

## Provider-Specific Notes

### LM Studio (google/gemma-4-12b)

- **Endpoint:** OpenAI-compatible at `http://localhost:8766/v1`
- **Auth:** None (local)
- **Tool support:** Yes (model reports `tools: true`)
- **Latency:** ~3.4s per test (12B model, streaming)
- **Observations:** Model follows tool definitions correctly. Sandbox refusal
  works as expected. Trace attribution records `"openai-compatible"` provider
  and `"google/gemma-4-12b"` model from `RunConfig`.

### LM Studio (bartowski/qwen2.5-0.5b-instruct)

- **Endpoint:** OpenAI-compatible at `http://localhost:8766/v1`
- **Auth:** None (local)
- **Tool support:** Yes (model reports `tools: true`)
- **Latency:** ~0.25s per test (0.5B model, streaming)
- **Observations:** Extremely fast but low-capability model. Completed all tests
  successfully. Did not always call file_read tool (acceptable per test design).
  Sandbox refusal worked correctly despite model's limited instruction following.

### OpenAI API

- **Status:** SKIP — no API key configured
- **Expected compatibility:** High (OpenAI is the canonical OpenAI-compatible endpoint)
- **Auth required:** API key (`OPENWAND_TEST_API_KEY`)
- **To validate:** Set `OPENWAND_TEST_BASE_URL=https://api.openai.com/v1`,
  `OPENWAND_TEST_API_KEY=sk-...`, `OPENWAND_TEST_MODEL=gpt-4o-mini`

### Anthropic

- **Status:** SKIP — no API key configured; Anthropic uses a different API format
- **Expected compatibility:** LOW — Anthropic API is NOT OpenAI-compatible
- **Note:** OpenWand's `openai_compatible` adapter targets OpenAI-compatible endpoints
  only. Anthropic would require a separate adapter implementation.
- **To validate:** Requires `openwand-llm` Anthropic adapter (not yet implemented)

### Ollama

- **Status:** SKIP — not running
- **Expected compatibility:** High (Ollama exposes OpenAI-compatible endpoint)
- **Auth:** None (local)
- **To validate:** Start Ollama, set `OPENWAND_TEST_BASE_URL=http://localhost:11434/v1`,
  `OPENWAND_TEST_API_KEY=unused`, `OPENWAND_TEST_MODEL=<model>`

---

## Skipped Providers

| Provider | Reason | Action Required |
|----------|--------|-----------------|
| OpenAI API | No API key | Configure key + run tests |
| Anthropic | Different API format | Implement Anthropic adapter |
| Ollama | Not running locally | Start Ollama + run tests |
| Azure OpenAI | No endpoint/key | Configure endpoint + key |
| Groq | No API key | Configure key + run tests |
| Together AI | No API key | Configure key + run tests |
| Mistral API | No API key | Configure key + run tests |

---

## Validation Scope Limitations

1. **Only OpenAI-compatible endpoints tested.** Anthropic, Google Gemini native API,
   and other non-OpenAI-compatible providers require separate adapter implementations.
2. **Only local provider tested.** No hosted API was tested. Network latency,
   rate limiting, error handling, and authentication flows are unvalidated.
3. **Model behavior is non-deterministic.** Results are from single runs.
4. **No approval-flow validation with real provider.** The 4 tests use read-only
   tools. Full approval E2E with real inference is not in this matrix.
5. **No concurrent-session validation.** Single session only.

---

## Matrix Schema

For future provider validation reports:

```json
{
  "provider_family": "LM Studio",
  "model": "google/gemma-4-12b",
  "endpoint_type": "local",
  "endpoint_url": "http://localhost:8766/v1",
  "auth_mode": "none",
  "api_key_used": false,
  "tool_support": true,
  "results": {
    "simple_turn": "PASS",
    "trace_attribution": "PASS",
    "tool_use": "PASS",
    "sandbox_refuse": "PASS"
  },
  "latency_ms": 3400,
  "notes": "12B model, Q4_K_M quantization, streaming"
}
```

---

*This matrix records what was tested, what was skipped, and why. It does not claim
universal provider compatibility. Hosted providers require separate validation with
appropriate credentials.*
