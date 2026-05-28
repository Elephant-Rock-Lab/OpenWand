# WAVE02K_MEMORY_GUIDED_PROMPT_ASSEMBLY_LOCK

**Status:** ✅ LOCKED  
**Commits:** f91d723 → 2906f4c (7 commits)  
**Tests:** 629 passing, 0 failures  

## Lock condition

```
02k assembles prompt context from RepoConsistencyReport, not from raw ranked search results.
Every injected memory item carries provenance explaining why it was included.
Unverifiable claim text never appears in prompt output.
```

## What shipped

### Commit 1 — Assembly DTOs + PromptInclusionReason + source provenance
- `PromptInclusionReason`: 4 variants (RepoSupported, SupersededHistory, ConflictReview, MissingMemoryGap)
- `SupportedMemoryClaim`: source_provenance + inclusion_reason + repo_evidence_key
- `SupersededMemoryClaim`: source_provenance + inclusion_reason
- `ConflictPromptClaim`: individual claim with source_provenance
- `MemoryConflictGroup`: claims with ConflictReview reason
- `MissingMemoryObservation`: repo-sourced (no source_provenance — comes from repo, not memory)
- `MemoryPromptAssemblyInputs`: full assembly with unverifiable count
- Two provenance kinds explicitly defined:
  - **Source provenance** (`ProvenanceSnapshot`): WHERE the claim came from
  - **Inclusion provenance** (`PromptInclusionReason`): WHY it's in the prompt

### Commit 2 — Assembly from RepoConsistencyReport
- `RepoConsistencyPromptAssembler`: stateless struct, no trait, no store state
- `assemble_from_report(report) → MemoryPromptAssemblyInputs`
- Maps each `RepoConsistencyFindingKind` to correct assembly type
- Unverifiable claims → count only, claim text NOT stored

### Commit 3 — Prompt formatting with provenance tags
- `to_prompt_block() → Option<String>`
- Fixed deterministic headings (only if non-empty):
  - `## Verified Memory` — `- {claim} [verified: {keys}]`
  - `## Memory History` — `NOT current truth` + superseded claims
  - `## Memory Conflicts` — `do not treat any as authoritative` + conflicts
  - `## Context Gaps` — `{key} [TODO: {detail}]`
  - `(N claims excluded: outside verification scope)` — unverifiable count
- Empty sections omitted entirely
- `None` if nothing to inject

### Commit 4 — Stateless assembler struct (no trait)
- No `MemoryPromptAssembler` trait
- No blanket impl for `dyn MemoryStore`
- No `MemoryReadStore` contamination
- Pure associated function on a unit struct

### Commit 5 — Session runner integration
- `RunConfig.memory_prompt_inputs: Option<MemoryPromptAssemblyInputs>` (`#[serde(skip)]`)
- Runner uses assembler output when present, falls back to raw `search()`
- Provenance-tagged context gets distinct system prompt instructions
- Legacy `search()` path preserved as fallback

### Commit 6 — Integration tests + SQLite parity
- All 7 edge tests from user's required list
- SQLite/in-memory parity: compares both struct and formatted string

## Architecture

```text
search_ranked(CurrentState)
  → repo_consistency classification
  → RepoConsistencyReport          ← trusted artifact from 02j
  → MemoryPromptAssemblyInputs     ← 02k consumes report, not raw hits
  → provenance-tagged prompt block
```

The session runner does NOT produce the report. The caller (session/app layer) is responsible for producing the `RepoConsistencyReport` via 02j and assembling inputs via 02k.

## Key design rules

1. **Assembly consumes `RepoConsistencyReport`, never raw ranked hits**
2. **Every prompt-includable item carries `PromptInclusionReason`** (inclusion provenance)
3. **Memory-derived items also carry `ProvenanceSnapshot`** (source provenance)
4. **Unverifiable claims are never mentioned by name** — count only
5. **Empty sections are omitted** — no empty `## Memory Conflicts` header
6. **Legacy `search()` preserved** — not removed, just not the authoritative path
7. **No trait, no blanket impl** — stateless struct with associated function

## Prompt invariant

```
No prompt line may be emitted unless it can name:
1. why it was included, and
2. whether it is current truth, historical context, conflict context, or a memory gap.
```

## Test delta

592 → 629 = +37 tests across 7 commits.

## Gaps (honest list)

- Runner doesn't automatically produce reports — caller must wire 02j→02k pipeline
- `MissingInRepo` findings go to `supported_claims` with empty evidence keys — could be a separate category
- `ConflictGroup` merging by group_id not implemented — each conflict finding becomes its own single-item group
- `SupportedMemoryClaim.confidence_bps` is always 0 (not carried in `RepoConsistencyFinding`)
- No caching of reports — each prompt assembly recomputes from findings
- App crate doesn't yet produce reports for real sessions

## Next

02j proved memory is checkable.
02k proved memory is safely consumable.
The 02j→02k pipeline is now the authoritative path from memory to prompt.
