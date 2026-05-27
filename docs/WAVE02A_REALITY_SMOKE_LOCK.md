# WAVE 02A — REALITY SMOKE — LOCK

**Status:** ✅ COMPLETE
**Date:** 2026-05-27
**Commits:** 32–33

## Summary

OpenWand can breathe with real I/O. A real LLM provider (Qwen 2.5 Coder 7B via LM Studio) drove the Wave 01 spine without breaking any abstractions.

## Verification

| Metric | Value |
|---|---:|
| Automated tests | 187 (unchanged) |
| Manual smoke tests | 2 passed |
| Warnings | 0 |

## Built

- `openwand-llm`: OpenAI-compatible adapter (reqwest + SSE)
  - `OpenAiCompatibleClient` implements `LlmClient`
  - Parses SSE into LlmDelta (Text, ToolCallStart/ArgsDelta/Complete, Done)
  - Works with LM Studio, Ollama, any `/v1/chat/completions` endpoint
  - No Rig dependency — direct reqwest
- `openwand-app`: CLI composition root
  - Wires SqliteStore + OpenAiCompatibleClient + CompositeToolExecutor + BuiltinPolicyEngine + StubMemoryStore
  - clap CLI with `--base-url`, `--model`, `--api-key`, `--db`
- `RunConfig.llm_target`: session runner now receives provider config from outside

## Manual Smoke Results

### Smoke 1: real_llm_text_only_turn

```
Provider: http://100.64.0.1:1234/v1
Model:    qwen2.5-coder-7b-instruct
User:     Say hello in one sentence.

Result:
  Stop reason:   Natural
  Steps:         0
  Tools called:  0
  Messages:      2 (user + assistant "Hello!")
  Loro:          fresh
  SQLite trace:  2 entries (session.user_message_injected, inference.completed)
```

### Smoke 2: real_llm_read_tool_turn

```
User: List the files in the current directory. Use the file_list tool.

Result:
  Model chose text response over tool call (7B model behavior)
  System handled it correctly — no crash, no stuck state
  Tool definitions were visible to the model (mentioned "file_list" in response)
```

## Key Discovery

The real model works through the spine. The adapter correctly:
- Sends OpenWand's `LlmMessage` → OpenAI format
- Streams SSE chunks → `LlmDelta::Text` deltas
- Records `inference.completed` events to SQLite trace
- Loro projection stays fresh after real streaming

## Architecture Validated

```text
Real user input
→ real LLM provider (LM Studio / Qwen 7B)
→ real streamed LlmDelta
→ deterministic policy gate (empty rules = allow)
→ SQLite trace append (BLAKE3 hashes)
→ Loro projection (fresh)
→ reload possible (trace is authoritative)
```

## Locked Boundary

```text
openwand-llm → reqwest (behind openai-compatible feature)
openwand-llm ↛ rig-core
openwand-app → all crates + clap + reqwest
```

## Deferred

| Item | Reason |
|---|---|
| Tool call through real model | 7B model chose not to use tools — need stronger model or better prompting |
| Rig integration | Direct reqwest proved simpler; Rig deferred to when we need its agent abstractions |
| Dioxus UI | Wave 03+ |
| MCP real transport | Wave 03+ |
| Memory extraction | Wave 03+ |

## Final Statement

Wave 02a is locked. OpenWand has crossed from verified architecture to running system. A real model drove the spine. The abstractions held.
