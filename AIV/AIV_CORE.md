# AIV FRAMEWORK CORE — SHARED FOUNDATION

**Extracted from:** AIV Framework v5.4 (master: AIV_FRAMEWORK_v5.4.md)  
**Audience:** All roles (Lead, Assistant, Reviewer)  
**Purpose:** Shared vocabulary, principles, document lifecycle, naming conventions, and project memory.  
**Usage:** This file is read by every session alongside its role-specific file (AIV_LEAD.md, AIV_ASSISTANT.md, AIV_REVIEWER.md).

---

## 1. FRAMEWORK OVERVIEW

The AIV Framework enforces a strict **Plan → Review → Execute → Verify** cycle at two levels:
- **Batch** — the sprint goal (defines scope, Hard Boundaries, and full Task list).
- **Task** — the smallest logical unit of work within a Batch.

Work is never "Done" at either level without a formal sign-off document.

### 1.1 Design Principles

- **Decoupling:** System design (Lead) is separated from execution (Assistant) and review (Reviewer).
- **Two-tiered scope:** Batches define goals. Tasks define discrete, logically coherent units.
- **AI-Executable:** All instructions are written to be followed directly by an AI agent without ambiguity.
- **Lead Sovereignty:** The Lead Programmer has final authority at every decision point.
- **No silent scope changes:** Tasks cannot be added, removed, or modified after the Blueprint is accepted without a formal revision cycle.
- **Source of truth is the artifact:** Completion is determined by the existence of the deliverable file, the git commit, and the signed document — not by session status or infrastructure signals.
- **Project memory:** Persistent documents (`PROJECT.md`, `STATE.md`) ensure every session inherits the project’s intent, architecture, and accumulated wisdom.

### 1.2 Two Cycle Modes

Every Batch runs under exactly one cycle mode, declared in the Blueprint.

| Mode | When to use | Document count |
|:---|:---|:---|
| **Standard Cycle** | >1 Task, or any Task that modifies existing source files, or any Task with Hard Boundaries | `3 + (2 × N Tasks) + 1` |
| **Simplified Cycle** | Exactly 1 Task, no existing source files modified, no Hard Boundaries required, single deliverable | `3` |

### 1.3 Full Standard Cycle at a Glance

```
BATCH BLUEPRINT  (defines goal + all Tasks upfront)
│
├── Phase I-B: AI Reviewer evaluates Blueprint + Task list
│   └── Lead Response: ACCEPT / ACCEPT WITH MODIFICATIONS / REJECT
│       (max two review cycles — then Lead decision is final)
│
├── TASK-01
│   ├── Phase II:   Assistant executes Task-01
│   ├──             Task Implementation Report submitted
│   └── Phase II-B: Lead issues Partial Sign-Off (or returns for correction)
│
├── TASK-02 ...
│
└── Phase III: BATCH CLOSE
    Lead issues Batch Sign-Off Certificate
    (coherence check — confirms all Tasks fit together, no boundary gaps)
```

### 1.4 Naming Conventions

All IDs must follow these formats exactly. IDs are identifiers, not filenames.

| Entity | ID Format | Example |
|:---|:---|:---|
| Batch | `BATCH-[NN]` | `BATCH-07` |
| Task | `BATCH-[NN]/TASK-[NN]` | `BATCH-07/TASK-03` |
| Review Report | `REVIEW-[BATCH-ID]-[YYYY-MM-DD]` | `REVIEW-BATCH-07-2025-06-14` |
| Task Report | `REPORT-[BATCH-ID]-[TASK-NN]-[YYYY-MM-DD]` | `REPORT-BATCH-07-TASK-03-2025-06-14` |
| Partial Sign-Off | `PARTIAL-[BATCH-ID]-[TASK-NN]-[YYYY-MM-DD]` | `PARTIAL-BATCH-07-TASK-03-2025-06-14` |
| Batch Certificate | `CERT-[BATCH-ID]-[YYYY-MM-DD]` | `CERT-BATCH-07-2025-06-14` |

---

## 2. DOCUMENT LIFECYCLE & AUDIT TRAIL

### 2.1 Git Commit Rules

1. Every commit message must reference the Batch ID and, where applicable, the Task ID:

   ```
   feat(batch-07/task-02): add remote agent transport
   docs(batch-07/task-01): update SDK reference
   chore(batch-07): Batch Sign-Off Certificate
   ```

2. One commit per role action:
   - Lead: Blueprint commit
   - Assistant: Implementation commit (code + tests)
   - Lead: Partial Sign-Off commit
   - Lead: Batch Certificate commit

3. The Assistant's implementation commit must include in the commit body:
   - Test evidence summary (N passed, M failed, K deferred)
   - LOC delta
   - Files changed count

4. The Lead must not combine code changes with certificate commits.

### 2.2 Session Status Caveat

**Session status is unreliable.** The source of truth for completion is:
1. The deliverable file exists on disk at the declared path
2. The git commit exists with the correct reference
3. The signed document (Report, Partial Sign-Off, or Certificate) is written and complete

Do not gate progress on session status. Gate progress on the existence of the signed document.

---

## 3. OPERATIONAL PRINCIPLES

These principles apply to all roles.

**P1 — Specification accuracy is the Lead's primary quality lever.** Adaptations logged by the Assistant are signals, not failures.  
**P2 — Session infrastructure is unreliable; documents and probes are not.** Trust the signed document, not session status.  
**P3 — The Reviewer's value is in catching gaps before they become errors.** Recurring flags should be addressed in the template.  
**P4 — The Simplified Cycle is a privilege, not a shortcut.** It still goes through Phase I-B review.  
**P5 — Deferred tests are debts, not dismissals.** Every deferred test carries a tracking reference.  
**P6 — The Lead Override is an escape valve, not a workflow.** Three consecutive overrides trigger a mandatory infrastructure halt.  
**P7 — Hard Boundaries are a contract.** A boundary affirmed as CONFIRMED and later found violated is grounds for RETURNING the Task.  
**P8 — Commit discipline is audit discipline.** One commit per role action.  
**P9 — Zero warnings is a gate, not an aspiration.** The Lint Command is mandatory.  
**P10 — LLM agents have no sense of time.** Always compute elapsed time from timestamps.  
**P11 — STATE.md is the codebase's long-term memory.** A Batch that closes without updating it forgets.  
**P12 — A review that doesn’t read the code is a rubber stamp.** The Investigative Layer must be performed or explicitly flagged as skipped.  
**P13 — A test that has never failed is a test that has never been tested.** The Test Integrity Protocol ensures every test is challenged.  
**P14 — Project intent must be as persistent as codebase state.** PROJECT.md is the compass; STATE.md is the map.

---

## 4. PROJECT MEMORY DOCUMENTS

### 4.1 STATE.md (Codebase State File)

**Location:** `/docs/aiv/STATE.md`  
**Owner:** Lead Programmer (updated at every Batch Close)  
**Read by:** Assistant, Reviewer, Lead  
**Contents:** Verified module paths, architectural decisions, known gotchas, adaptation log, test baseline, carry-forward obligations (deferred tests).

### 4.2 PROJECT.md (Project Intent Memory)

**Location:** `/docs/aiv/PROJECT.md`  
**Owner:** Lead Programmer  
**Read by:** Assistant, Reviewer, Lead  
**Contents:** Project vision, core value propositions, user personas & critical workflows, quality attributes, engineering philosophy, known friction points, roadmap.

### 4.3 PRECHECK.md (Pre-Blueprint Health Checklist)

**Location:** `/docs/aiv/PRECHECK.md`  
**Audience:** Lead only (advisory self-assessment)  
**Purpose:** A checklist to surface architectural risks before writing a Blueprint. Not reviewed or executed by other roles.

---

## 5. GLOSSARY

- **Adaptation:** A technically correct adjustment made when the Blueprint’s data model or paths do not match the actual codebase. Not a violation.
- **Deviation:** A departure from the Blueprint’s instructions not caused by a codebase mismatch. Requires justification; Lead may RETURN.
- **Hard Boundary:** A falsifiable constraint that applies to all Tasks in a Batch.
- **Partial Sign-Off:** The Lead’s per-Task approval (Standard Cycle only).
- **Batch Sign-Off Certificate:** The Lead’s final approval closing the entire Batch.

---

**This file is not authoritative alone. The complete, binding framework is AIV_FRAMEWORK_v5.4.md.**
