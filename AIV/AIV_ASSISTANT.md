# AIV FRAMEWORK — ASSISTANT PROCEDURES

**Extracted from:** AIV Framework v5.4 (master: AIV_FRAMEWORK_v5.4.md)  
**Audience:** Assistant AI  
**Read alongside:** AIV_CORE.md, the Batch Blueprint (with Lead Response), the Review Report, and current STATE.md and PROJECT.md.  
**Covers:** Pre-task reading, execution rules, Task Implementation Report, Adaptations vs Deviations, lint and test evidence.

---

## 1. BEFORE EXECUTING ANY TASK

Read and confirm the following:

```
[ ] Full Batch Blueprint (including Lead Response)
[ ] Review Report
[ ] /docs/aiv/STATE.md (if exists)
[ ] /docs/aiv/PROJECT.md (if exists) — internalize value props, quality priorities, friction points
[ ] Cycle mode noted: STANDARD or SIMPLIFIED
[ ] Specific Task block identified
[ ] All dependency Tasks have APPROVED Partial Sign-Off (if Sequential/Mixed)
[ ] All mandatory Blueprint fields present
```

If any mandatory field is missing, or a dependency lacks a Partial Sign-Off, **HALT and notify the Lead**. Do not proceed with assumptions.

---

## 2. EXECUTION RULES

- Execute exactly the Task described in the Blueprint — do not add or remove scope.
- Files changed must match the declared “Files in scope.” Any change outside that list is a Deviation.
- Run the Lint Command specified in the Blueprint. Zero warnings and zero errors are required.
- Every named test in the Blueprint must be executed or explicitly deferred.
- Record all Adaptations (when the codebase reality differs from the Blueprint’s data model) and Deviations (any other departure from the plan).

---

## 3. TASK IMPLEMENTATION REPORT

Submit one report per Task, using this exact template.

```
TASK IMPLEMENTATION REPORT
═══════════════════════════════════════════════════════════

Report ID:             REPORT-[BATCH-ID]-[TASK-NN]-[YYYY-MM-DD]
Batch ID:              [BATCH-NN]
Task ID:               [BATCH-NN/TASK-NN]
Blueprint Version:     [Must match version passed]
Submitted By:          [Assistant name / system ID]
Submission Timestamp:  [ISO 8601]

───────────────────────────────────────────────────────────
SCOPE CONFIRMATION
───────────────────────────────────────────────────────────
  Task Description confirmed: [ ] YES / [ ] NO — reason
  Final test count:  [N] total ([baseline] existing + [delta] new)

───────────────────────────────────────────────────────────
LINT EVIDENCE
───────────────────────────────────────────────────────────
  Lint command executed: [command]
  Warnings:  [N — must be 0]
  Errors:    [N — must be 0]
  Output excerpt (last 5 lines):
  [paste]

───────────────────────────────────────────────────────────
HARD BOUNDARY AFFIRMATION
───────────────────────────────────────────────────────────
State compliance for each Batch-level Hard Boundary.

  HB-01: CONFIRMED — [restate boundary]. This boundary was NOT violated.
  HB-02: VIOLATED  — [restate boundary]. Violation: [explain]. Remediation: [action].

───────────────────────────────────────────────────────────
FILES CHANGED
───────────────────────────────────────────────────────────
| File Path | Action | In Scope? | Reason |
|:----------|:-------|:----------|:-------|
| src/...   | Created/Modified/Deleted | YES/NO | [Why] |

Files marked NO must also appear in Deviations.

───────────────────────────────────────────────────────────
TEST EVIDENCE
───────────────────────────────────────────────────────────
Every test named in the Blueprint must have a row.

| Test ID        | Type | Behavior Verified              | Result  | Failure Confirmed?                                  | Log Reference |
|:---------------|:-----|:-------------------------------|:--------|:----------------------------------------------------|:--------------|
| TEST-NN-NN-01  | unit | [from Blueprint Behavior col]  | ✓ PASS  | N/A (previously failed in BATCH-NN)                 | ...           |
| TEST-NN-NN-02  | unit | [from Blueprint Behavior col]  | ✓ PASS  | YES — falsified: introduced bug, test failed, reverted | ...        |
| TEST-NN-NN-03  | e2e  | [from Blueprint Behavior col]  | ⏸ DEFERRED | N/A                                              | [reason]      |

Deferred tests must also appear in Deferred Tests section. Deferred tests must not exceed 20% of total named tests for this Task.

───────────────────────────────────────────────────────────
FAILURE VERIFICATION (§13 — Test Integrity Protocol, T6)
───────────────────────────────────────────────────────────
Task Priority: [Critical / High / Medium / Low]

For Critical/High Tasks: falsification is mandatory for every test.
For Low/Medium Tasks: falsification required only for tests that have never failed.

  TEST-NN-NN-01: Previously failed in BATCH-NN — verified.
  TEST-NN-NN-02: Never failed. Falsification performed:
    Diff applied:    [exact unified diff]
    Test output:     [failure output lines]
    Revert:          [commit hash / "reverted in working tree before commit"]
  TEST-NN-NN-03: DEFERRED — not applicable.

If any falsification produced a PASS (test did not fail when bug was introduced):
  DEFECTIVE TEST: TEST-NN-NN-XX — falsification did not produce failure.
  Root cause: [explain]
  Resolution: [test was fixed / replaced / requires Lead decision]

───────────────────────────────────────────────────────────
TRACEABILITY CONFIRMATION (§13 — T5)
───────────────────────────────────────────────────────────
  AC-NN-01 → TEST-NN-NN-01 (✓ PASS), TEST-NN-NN-02 (✓ PASS) — covered
  AC-NN-02 → TEST-NN-NN-03 (⏸ DEFERRED) — tracked in: [ref]
  Unmapped tests: None
  Uncovered ACs: None

───────────────────────────────────────────────────────────
DEFERRED TESTS
───────────────────────────────────────────────────────────
Write "None" if all tests executed.

  DEFER-01: [Test ID] — [reason]
            STATE.md entry: DEFER-BATCH-NN-TASK-NN-TEST-NN (status: PENDING_LEAD_CONFIRMATION)

The Assistant appends to STATE.md as PENDING_LEAD_CONFIRMATION. Do not edit or resolve existing STATE.md entries.

───────────────────────────────────────────────────────────
ACCEPTANCE CRITERIA
───────────────────────────────────────────────────────────
  AC-NN-01: [ ✓ Met / ✗ Failed ] — [comment]
  AC-NN-02: [ ✓ Met / ✗ Failed ] — [comment]

───────────────────────────────────────────────────────────
ADAPTATIONS
───────────────────────────────────────────────────────────
Record any mismatch between Blueprint specification and actual codebase.
Write "None" if Blueprint matched codebase exactly.

  ADAPT-01: Blueprint stated [X]. Actual codebase has [Y]. Resolution: [Z].

───────────────────────────────────────────────────────────
DEVIATIONS
───────────────────────────────────────────────────────────
Any departure from the Blueprint not caused by a codebase mismatch.
Write "None" if no deviations occurred.

  DEVIATION-01: [What deviated] — [justification]

───────────────────────────────────────────────────────────
DOCUMENTATION DELIVERED
───────────────────────────────────────────────────────────
  [ ] Inline code comments on all complex logic blocks
  [ ] Task section added to BATCH_[ID].md under /docs/aiv/[BATCH-ID]/

───────────────────────────────────────────────────────────
ASSISTANT SIGN
───────────────────────────────────────────────────────────
  Assistant ID:   [Name / system ID]
  Timestamp:      [ISO 8601]

═══════════════════════════════════════════════════════════
```

---

## 4. ADAPTATIONS VS DEVIATIONS

| Category | Definition | Example |
|:---|:---|:---|
| **Adaptation** | Blueprint’s specification did not match the actual codebase; technically correct adjustment made. | Blueprint referenced `kore_cap::keys::KeyPair`; actual export is `ed25519_dalek::KeyPair`. Used type alias. |
| **Deviation** | Departure from Blueprint instructions not caused by a codebase mismatch. | Assistant modified a file outside the declared scope. |

Adaptations inform future Blueprint accuracy. Deviations require justification and may cause the Lead to RETURN the Task.

---

## 5. SESSION COMMUNICATION

- After completing the Task Report, send a message to the Lead session with a summary: files changed, test count, commit hash, any deviations or adaptations.
- The message is the completion signal. Do not rely on session status.

---

**This file is not authoritative alone. The complete, binding framework is AIV_FRAMEWORK_v5.4.md.**
