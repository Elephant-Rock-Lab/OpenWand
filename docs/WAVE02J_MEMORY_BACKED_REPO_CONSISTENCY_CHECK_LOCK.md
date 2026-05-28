# WAVE02J_MEMORY_BACKED_REPO_CONSISTENCY_CHECK_LOCK

**Status:** ✅ LOCKED  
**Commits:** 20dff51 → fb54ebe (8 commits)  
**Tests:** 592 passing, 0 failures  

## Lock condition

```
OpenWand can produce a deterministic, read-only RepoConsistencyReport
that compares current memory claims, superseded memory history,
conflict-grouped records, and observable repo structure,
with matching behavior across in-memory and SQLite-backed memory.
```

## What shipped

### Commit 1 — Report DTOs, summary invariants, and checker skeleton
- `RepoConsistencyReport`, `RepoConsistencySummary`, `RepoConsistencyFinding`
- `RepoConsistencyFindingKind`: 7 variants (Supported, StaleMemory, MissingInRepo, MissingInMemory, SupersededMemoryIgnored, ConflictRequiresReview, Unverifiable)
- `RepoConsistencyClock` trait (SystemClock + FixedClock for deterministic tests)
- `normalize_repo_path()` for stable cross-platform path comparison
- Summary invariant: `from_findings()` ensures counts always match

### Commit 2 — Read-only repo observation snapshot
- `RepoReadFs` trait: `read_to_string`, `read_dir`, `exists` — **no write methods**
- `StdRepoReadFs` (real filesystem), `StubRepoReadFs` (test-only)
- `observe_repo()`: parses Cargo.toml, discovers crates, src files, test files, docs, dependencies
- Deterministic line-by-line parsing (no TOML crate dependency)

### Commit 3 — Memory input loader
- `load_memory_inputs()`: loads CurrentState, ChangeHistory, ConflictSearch views
- Filters current claims to accepted-state only
- Derives conflict groups from records

### Commit 4 — Deterministic claim grammar v0
- `RepoClaimPattern`: CrateExists, WorkspaceContainsCrate, FileExists, ModuleExists, CrateDependsOn, Unsupported
- `parse_claim()`: deterministic parser, no fuzzy matching
- `match_claim()`: checks pattern against crate names, src files, dependencies
- Crate name matching supports prefix stripping: "core" matches "openwand-core"

### Commit 5 — Supersession and conflict handling
- Superseded claims → `SupersededMemoryIgnored` (not current truth)
- Conflict-grouped claims → `ConflictRequiresReview` (not promoted to Supported)
- Even if one conflict side matches repo, neither gets Supported

### Commit 6 — Missing-in-memory detection
- Repo observations without memory claims → `MissingInMemory`
- Capped to high-level observations: workspace crates, dependencies, docs
- Does NOT emit per-file findings
- Severity: Medium (crates/deps), Low (docs)

### Commit 7 — End-to-end tests
- 9 E2E tests including SQLite/in-memory parity
- Fixture repo with 2 crates, dependencies, docs

## Claim grammar v0

| Pattern | Example claim | Matches against |
|---------|---------------|-----------------|
| CrateExists | "crate core exists" | workspace crate names |
| WorkspaceContainsCrate | "workspace contains crate memory" | workspace crate names |
| FileExists | "file src/lib.rs exists" | observed src files |
| ModuleExists | "module core::events exists" | src/*.rs file endings |
| CrateDependsOn | "crate core depends on serde" | [dependencies] entries |
| Unsupported | anything else | — (becomes Unverifiable) |

## Retrieval modes used

| Mode | Purpose |
|------|---------|
| `CurrentState` | Active claims for matching |
| `ChangeHistory` | Superseded chain for ignored history |
| `ConflictSearch` | Conflict-grouped records for review |

## Read-only guarantee

- `RepoReadFs` has no write methods — compile-time enforcement
- Two observations produce identical snapshots
- No Git mutation, no file writes, no memory writes

## Accepted limitations

- No LLM semantic matching
- No automatic memory correction
- No broad code understanding
- No prompt assembly
- Only file/crate/module/dependency claims classified
- Unsupported claims are Unverifiable (not failures)
- `StaleMemory` may be rare in v0 — only appears if property/value claims are supported

## Test delta

543 → 592 = +49 tests across 8 commits.

## Next

**Wave 02k — Memory-Guided Prompt Assembly**

02k should consume:
- Supported claims (verified current truth)
- Superseded history (avoid stale facts)
- Conflict groups (surface ambiguities)
- Missing-in-memory (gaps to address)

Only after 02k verifies which memory records are fit for consumption should prompt construction begin.
