# WAVE 02H — HONESTY CORRECTION — LOCK

**Status:** ✅ COMPLETE
**Date:** 2026-05-27
**Scope:** Fix premature locks, rename misleading types, fix retrieval quality, add automated binary E2E

## What was wrong

1. **02f was mislocked** — lock condition was not met when locked (stubs still in binaries)
2. **KeywordExtractor** was misleading — it's a heuristic rule-matcher, not semantic extraction
3. **Retrieval was full-substring** — required entire query to be substring of claim; useless for real queries
4. **No automated binary E2E** — all "E2E" tests constructed library objects; none spawned the actual binary

## What was fixed

### 02f lock amended
- Added `⚠️ STATUS CORRECTION — PREMATURE LOCK` banner
- Marked as `SUPERSEDED / INVALID LOCK`
- Listed what was actually proven vs what was claimed

### KeywordExtractor → HeuristicExtractor
- Renamed to `HeuristicExtractor` with honest doc comments
- Documented as "deterministic v0 placeholder, NOT semantic extraction"
- Listed limitations: no semantic understanding, promotes entire message, can't distinguish importance
- `KeywordExtractor` kept as deprecated type alias for backward compat

### Token-based retrieval
- Replaced full-substring `LIKE '%query%'` with token-level matching
- Tokenization: lowercase → split on non-alphanumeric → filter stopwords → normalize plurals
- Scoring: (matched_tokens / query_tokens) × confidence
- Shared `tokenize()` function in `query.rs` (used by both InMemory and SqliteMemory stores)
- **Real improvement proven**: "What programming language should I use for my new project?" now matches "Remember that I always use Rust for new projects" via shared tokens: `use`, `new`, `project`

### Automated binary E2E
- `tests/e2e_binary.sh` — spawns real `openwand.exe` twice
- Turn 1: "Remember that I always use Rust for new projects" → asserts memory accepted
- Turn 2: "What programming language should I use for my new project?" → asserts LLM mentions Rust
- Checks LLM reachability, binary existence, correct exit codes
- No manual steps required

## Real E2E results (automated)

```
Turn 1: "Remember that I always use Rust for new projects"
  → Memory: 1 record accepted ✓
Turn 2: "What programming language should I use for my new project?"
  → LLM: "You mentioned that you always use Rust for new projects.
           Therefore, Rust is the programming language you should use." ✓
```

## Tests: 267 total, 0 failures
