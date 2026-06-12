---
name: Provider Validation Result
about: Report validation results for a specific LLM provider/model combination
title: "[PROVIDER] "
labels: provider-validation, triage
---

## Provider Validation Result

**OpenWand version:** v0.1.0-alpha (commit `967dc96` or later)

### Provider Information

| Field | Value |
|-------|-------|
| Provider name | [e.g. OpenAI, Anthropic, LM Studio, Ollama] |
| Endpoint type | [OpenAI-compatible / Anthropic API / MCP / Other] |
| Model tested | [e.g. gpt-4o, claude-3.5-sonnet, llama-3.1-8b] |
| Auth method | [API key / OAuth / local / none] |
| Endpoint URL | [e.g. https://api.openai.com/v1, localhost:11434] |

### Test Results

Run the real-provider validation tests if possible:
```
OPENWAND_TEST_BASE_URL=<endpoint> \
OPENWAND_TEST_API_KEY=<key> \
OPENWAND_TEST_MODEL=<model> \
cargo test -p openwand-session --features testing \
  --test real_provider_validation -- --ignored
```

| Test | Result |
|------|--------|
| `real_provider_completes_simple_turn` | PASS / FAIL / SKIP |
| `real_provider_trace_records_attribution` | PASS / FAIL / SKIP |
| `real_provider_read_tool_works` | PASS / FAIL / SKIP |
| `real_provider_sandbox_refuses_escape` | PASS / FAIL / SKIP |

### Observations

Any notable behavior, errors, or incompatibilities observed during testing.

### Tool Calling Support

- [ ] Model supports tool calling (function calling)
- [ ] Model does not support tool calling
- [ ] Unknown

### Additional Context

Any other relevant information about the provider or model behavior.
