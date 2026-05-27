# OpenWand Trust Architecture

**Date:** 2026-05-26  
**Status:** Adopted  
**Source:** Deterministic/non-deterministic analysis + external critique integration

---

## Core Principle

> **AI proposes. Deterministic systems constrain. Evidence promotes. Users govern consequential change.**

---

## The Split

| Kind | Examples | Character |
|---|---|---|
| **Deterministic** | Workflow engine FSM, gate evaluation, mode routing, Loro CRDT operations, policy enforcement, content rendering, reconciliation, git/cargo operations, mode ledger, artifact accuracy | Reproducible. Same input → same output. Inspectable. Testable. Not automatically correct — a bad rule is reliably bad. |
| **Non-Deterministic** | LLM code generation, memory extraction, request classification, reviewer evaluation, assistant execution, proposal generation, memory tiering (L0/L1/L2), conception/discovery frameworks | Probabilistic. Same prompt ≠ same output. Variable in quality, tone, correctness, and completeness. |

**Deterministic does not mean trusted. It means reproducible.** A deterministic gate is only as good as the property it checks.

---

## The Rule

**Every non-deterministic output is a candidate. It becomes trusted state only after deterministic verification.**

LLM output is never promoted directly into:
- Persistent memory
- File writes
- Git commits
- Workflow state transitions
- Policy mutations
- User-facing claims of completion

---

## Gate Taxonomy

### Structural Gates

| Gate type | What it checks |
|---|---|
| Syntax | Is the output well-formed? (compiles, parses, valid JSON) |
| Schema | Does it match the required structure? |
| Type | Does it satisfy compile-time constraints? |
| Test | Do known behaviors still pass? |
| Policy | Is the action allowed? |
| Artifact | Does the required artifact exist? |

### Semantic Gates

| Gate type | What it checks |
|---|---|
| Semantic | Does it satisfy the user's actual intent? |
| Regression | Did it break existing behavior not covered by unit tests? |
| Security | Did it introduce unsafe capability, leakage, injection, or escalation? |
| Rollback | Can this action be reversed? |
| Auditability | Is this action recorded for future audit? |

### Rust Types

```rust
pub enum GateCondition {
    // Structural
    LintPasses { command: String },
    TestsPass { coverage_threshold: Option<f64>, include_deferred: bool },
    ArtifactsPresent { artifacts: Vec<ArtifactKind> },
    BoundariesRespected { check_unscoped_files: bool },
    ReviewApproved { checklist_items: usize, min_pass_ratio: f64 },
    WithinSla { max_duration: Duration },

    // Semantic
    SemanticMatch { original_request_hash: String, obligation_check: String },
    RegressionCheck { baseline_test_suite: String },
    SecurityCheck { scan_patterns: Vec<String> },
    RollbackAvailable { requires_backup: bool },
    AuditabilityCheck { ledger_entry_required: bool },

    // Combinators
    All { conditions: Vec<GateCondition> },
    Any { conditions: Vec<GateCondition> },
    Custom { evaluator: String, params: serde_json::Value },
}
```

---

## Decision Ledger

Alongside the mode ledger (which tracks mode history), a decision ledger tracks every LLM output that was accepted or rejected:

```rust
pub struct DecisionRecord {
    pub id: DecisionId,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub source: DecisionSource,        // LLM, User, System
    pub model: Option<String>,
    pub prompt_hash: Option<String>,
    pub candidate_action: String,
    pub gates_run: Vec<GateResult>,
    pub outcome: DecisionOutcome,      // Accepted, Rejected, Deferred
    pub risk_level: RiskLevel,
    pub rollback_path: Option<String>,
}

pub enum DecisionSource { Llm, User, System }
pub enum DecisionOutcome { Accepted, Rejected, Deferred }
pub enum RiskLevel { Low, Medium, High, Critical }
```

---

## Risk-Aware Confirmation

User confirmation is not a magic safety boundary. The confirmation level is determined by what's being changed, not by the LLM's opinion:

| Risk level | Trigger | Required acceptance |
|---|---|---|
| Low | Single file, no existing code modified | Auto-accept after deterministic gates |
| Medium | 3+ files or modifies existing code | Show diff + explanation + tests |
| High | Touches auth, persistence, policy, or 5+ files | Explicit approval + rollback plan |
| Critical | Structural change (new crate, API change, dependency add) | Approval + backup + possible second review |

```rust
pub enum ConfirmationLevel {
    Auto,      // Gates pass → accepted
    Inform,    // Show diff, explain, accept on ack
    Approve,   // Require explicit approval
    Escalate,  // Approval + rollback plan + optional second review
}
```

---

## Memory Controls

Extracted memories need controls beyond schema validation. These are CozoDB schema fields:

| Control | Purpose |
|---|---|
| Provenance | Where did this memory come from? (session, batch, user input, inference) |
| Scope | Does it apply globally, to one project, or one session? |
| Confidence | Is it explicit (user stated), inferred (LLM extracted), or speculative? |
| Expiry | Should it decay or be revalidated? (Ebbinghaus curve) |
| Conflict | What happens when new memory contradicts old? (invalidate, not delete) |
| Visibility | Can the user inspect and correct it? (always yes) |

---

## LLM Risk Spectrum

Non-determinism is only one risk. The full spectrum:

| Risk | Meaning |
|---|---|
| Non-determinism | Same prompt → different outputs |
| Hallucination | Output unsupported or false |
| Misalignment | Output satisfies prompt but violates product intent |
| Prompt injection | External content manipulates the assistant |
| Context poisoning | Bad memory or retrieved documents corrupt future behavior |
| Overconfidence | System presents uncertain output as verified |
| Tool misuse | Model calls wrong tool or acts at wrong time |
| Silent omission | Model misses constraint without obvious failure |
| Goal drift | Assistant optimizes for task completion over correctness |
| Evaluation blind spots | Gates check only known failure modes |

**The LLM is not merely variable. It is an untrusted reasoning component that must be constrained, observed, and verified.**

---

## Design Principle

> High-agency reasoning, low-authority execution.

The LLM does heavy cognitive lifting — planning, reasoning, critiquing, synthesizing. That's not thin. What's constrained is its **authority**: it doesn't write to disk, doesn't commit, doesn't transition workflow state, doesn't mutate memory directly. It proposes. The deterministic spine decides.
