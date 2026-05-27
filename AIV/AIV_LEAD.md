# AIV FRAMEWORK — LEAD PROGRAMMER PROCEDURES

**Extracted from:** AIV Framework v5.4 (master: AIV_FRAMEWORK_v5.4.md)  
**Audience:** Lead Programmer  
**Read alongside:** AIV_CORE.md, the current STATE.md and PROJECT.md  
**Covers:** Blueprint creation, review response, Partial Sign-Off, Batch Close, session lifecycle for spawned roles, STATE.md maintenance.

---

## 1. BLUEPRINT CREATION (Phase I)

### 1.1 Standard Cycle Blueprint Template

```
BATCH BLUEPRINT
═══════════════════════════════════════════════════════════

Batch ID:                 [e.g. BATCH-07]
Blueprint Version:        [1.0 on first issue; increment on revision]
Cycle Mode:               STANDARD
Lead Programmer:          [Name / ID]
Date Issued:              [YYYY-MM-DD]
Review SLA:               [Default 30 min]
Execution SLA per Task:   [Default 60 min]
Partial Sign-Off SLA:     [Default 15 min]
Task Sequencing:          [Sequential / Parallel / Mixed]

───────────────────────────────────────────────────────────
BATCH GOAL
───────────────────────────────────────────────────────────
A single clear statement of the deployable outcome this Batch produces:

───────────────────────────────────────────────────────────
SCOPE STATEMENT
───────────────────────────────────────────────────────────
What the code MUST do:
  -

What the code MUST NOT do:
  -

───────────────────────────────────────────────────────────
LINT COMMAND
───────────────────────────────────────────────────────────
The project's zero-warning build/lint command.

  Lint command:  [e.g. cargo build --workspace]

───────────────────────────────────────────────────────────
HARD BOUNDARIES
───────────────────────────────────────────────────────────
Each boundary must be falsifiable.

  HB-01:
  HB-02:

───────────────────────────────────────────────────────────
DATA MODELS / SCHEMA
───────────────────────────────────────────────────────────
[Exact table definitions, API contracts, field types, verified paths]

───────────────────────────────────────────────────────────
AUTHORITY RULES
───────────────────────────────────────────────────────────
[Trust, security, and state-change rules]

───────────────────────────────────────────────────────────
DEPENDENCY MAP
───────────────────────────────────────────────────────────
[Prior Batches or modules this Batch depends on]

───────────────────────────────────────────────────────────
STATE.md STATUS
───────────────────────────────────────────────────────────
  State file exists:       [ ] YES  [ ] NO — first Batch, will create
  Last Updated:            [date from STATE.md]
  Batches since update:    [N — must be <5, or reconciliation audit required]
  Reconciliation audit:    [ ] N/A (< 5 batches since update)
                           [ ] PERFORMED — see audit notes: [reference]

───────────────────────────────────────────────────────────
TEST BASELINE
───────────────────────────────────────────────────────────
  Baseline at Blueprint issuance:  [N] existing tests
  Expected delta (all Tasks):      +[M] new tests
  Expected total at Batch close:   [N+M]

───────────────────────────────────────────────────────────
TASK LIST
───────────────────────────────────────────────────────────
All Tasks must be defined here before review.

TASK-01: [BATCH-07/TASK-01]
  Priority:          [Critical / High / Medium / Low]
  Description:       [What this Task does — one logical concern]
  Files in scope:    [Files expected to be created or modified]
  Depends on:        [None / TASK-NN]
  Required Tests:
    | Test ID          | Type                              | Behavior Verified                  | Failure Mode                          | Falsified By                                     | Pass Criteria                             |
    |:-----------------|:----------------------------------|:-----------------------------------|:--------------------------------------|:-------------------------------------------------|:------------------------------------------|
    | TEST-07-01-01    | unit / integration / e2e / manual | [What specific behavior this tests]| [What would go wrong if this breaks]  | [What code change would make this test fail]     | [Specific assertion]                     |
  Acceptance Criteria:
    AC-01-01:
    AC-01-02:
  Traceability:
    AC-01-01 → TEST-07-01-01

TASK-02: [BATCH-07/TASK-02]
  ... same structure ...

───────────────────────────────────────────────────────────
BATCH-LEVEL ACCEPTANCE CRITERIA
───────────────────────────────────────────────────────────
  BAC-01:
  BAC-02:
  BAC-03: CHANGELOG.md updated with [BATCH-ID] entry.
  BAC-04: All documents archived under /docs/aiv/[BATCH-ID]/.

───────────────────────────────────────────────────────────
LEAD RESPONSE TO REVIEW REPORT
───────────────────────────────────────────────────────────
[Completed after Phase I-B. Leave blank until Review Report received.]

Reviewer Report ID:       [REVIEW-BATCH-NN-YYYY-MM-DD]
Review Cycle:             [1 or 2]
Lead Decision:            [ ] ACCEPT   [ ] ACCEPT WITH MODIFICATIONS   [ ] REJECT

If ACCEPT WITH MODIFICATIONS — list each Reviewer flag acted on:
  FLAG-01 → Action taken:

If REJECT — reason and next action:

Blueprint Version after response:
Lead Sign:                [Name + YYYY-MM-DD HH:MM]

═══════════════════════════════════════════════════════════
```

### 1.2 Simplified Cycle Blueprint Template

```
BATCH BLUEPRINT
═══════════════════════════════════════════════════════════

Batch ID:                 [e.g. BATCH-07]
Blueprint Version:        [1.0]
Cycle Mode:               SIMPLIFIED
Lead Programmer:          [Name]
Date Issued:              [YYYY-MM-DD]
Review SLA:               [Default 30 min]
Execution SLA:            [Default 60 min]

SIMPLIFIED CYCLE ELIGIBILITY — confirm all:
  [ ] Exactly 1 Task
  [ ] No existing source files modified
  [ ] No Hard Boundaries required
  [ ] Single deliverable

───────────────────────────────────────────────────────────
BATCH GOAL
───────────────────────────────────────────────────────────


───────────────────────────────────────────────────────────
SCOPE STATEMENT
───────────────────────────────────────────────────────────
What this deliverable MUST contain or do:
  -

What it MUST NOT do:
  -

───────────────────────────────────────────────────────────
TASK DEFINITION
───────────────────────────────────────────────────────────
  Description:      [What is being produced]
  Files in scope:   [File(s) to be created — no existing source files]
  Priority:         [Critical / High / Medium / Low]
  Required Tests:
    | Test ID         | Type | Behavior Verified | Failure Mode | Falsified By | Pass Criteria |
    |:----------------|:-----|:------------------|:-------------|:-------------|:--------------|
    | TEST-NN-01-01   |      |                   |              |              |               |
  Acceptance Criteria:
    AC-01:
    AC-02:
  Traceability:
    AC-01 → TEST-NN-01-01

───────────────────────────────────────────────────────────
BATCH-LEVEL ACCEPTANCE CRITERIA
───────────────────────────────────────────────────────────
  BAC-01:
  BAC-02:
  BAC-03: All documents archived under /docs/aiv/[BATCH-ID]/.

───────────────────────────────────────────────────────────
LEAD RESPONSE TO REVIEW REPORT
───────────────────────────────────────────────────────────
[Same as Standard Cycle]

═══════════════════════════════════════════════════════════
```

### 1.3 Hard Boundary Format Rules

Every Hard Boundary must be falsifiable.  
**Valid:** `HB-01: The system MUST NOT allow any public-source package to auto-install without an explicit admin approval record.`  
**Invalid:** `"Be careful with security."`

---

## 2. REVIEW RESPONSE (Phase I-B)

After receiving the Review Report, complete the **Lead Response** section in the Blueprint.

| Decision | Meaning |
|:---|:---|
| **ACCEPT** | Blueprint approved as-is. Pass Blueprint + Review Report to Assistant. |
| **ACCEPT WITH MODIFICATIONS** | Blueprint approved after changes. Log each flag acted on. Increment Blueprint Version. Pass revised Blueprint + Report to Assistant. |
| **REJECT** | Revise Blueprint and trigger a second (and final) Review Cycle. |

Maximum two Review Cycles. After the second, the Lead's decision is final.

If the Reviewer states `INVESTIGATIVE LAYER: SKIPPED`, you must perform the Manual Investigative Review yourself (template in the master framework) and attach it to the Lead Response.

---

## 3. PARTIAL SIGN-OFF (Phase II-B, Standard Cycle only)

After an Assistant submits a Task Implementation Report, issue a Partial Sign-Off using this template:

```
PARTIAL SIGN-OFF
═══════════════════════════════════════════════════════════

Partial Sign-Off ID:      PARTIAL-[BATCH-ID]-[TASK-NN]-[YYYY-MM-DD]
Batch ID:                 [BATCH-NN]
Task ID:                  [BATCH-NN/TASK-NN]
Report Reviewed:          [Report ID]
Review Timestamp:         [ISO 8601]
SLA Compliance:           [ ] YES   [ ] NO
Self-Review Acknowledged: [ ] N/A   [ ] YES — Lead acted as both Lead and Assistant

───────────────────────────────────────────────────────────
VERDICT
───────────────────────────────────────────────────────────
  [ ] APPROVED — Task complete. Dependent Tasks may now begin.
  [ ] RETURNED — See corrections below.

───────────────────────────────────────────────────────────
DEFERRED TESTS NOTED
───────────────────────────────────────────────────────────
List any deferred tests from the Task Report. Lead confirms or rejects each
PENDING_LEAD_CONFIRMATION entry.
  DEFER-01: [Test ID] — STATE.md entry: DEFER-BATCH-NN-TASK-NN-TEST-NN
    Lead action: [ ] CONFIRMED → OPEN  [ ] REJECTED

───────────────────────────────────────────────────────────
CORRECTIONS REQUIRED (only if RETURNED)
───────────────────────────────────────────────────────────
  CORRECTION-01: [Blueprint field ref] — [Discrepancy]

───────────────────────────────────────────────────────────
NOTES FOR SUBSEQUENT TASKS
───────────────────────────────────────────────────────────
[Optional observations for later Tasks or Batch Close.]

───────────────────────────────────────────────────────────
LEAD SIGN
───────────────────────────────────────────────────────────
  Lead Name:   [Name]
  Timestamp:   [ISO 8601]

═══════════════════════════════════════════════════════════
```

- A RETURNED Task is resubmitted as a revised Implementation Report. SLA resets.
- A Task depending on a RETURNED Task must not begin until the dependency is APPROVED.

---

## 4. BATCH CLOSE (Phase III)

After ALL Tasks are APPROVED (Standard) or execution is complete (Simplified), issue the Batch Sign-Off Certificate.

```
BATCH SIGN-OFF CERTIFICATE
═══════════════════════════════════════════════════════════

Certificate ID:          CERT-[BATCH-ID]-[YYYY-MM-DD]
Batch ID:                [BATCH-NN]
Cycle Mode:              [STANDARD / SIMPLIFIED]
Blueprint Version:       [Final accepted version]
Review Timestamp:        [ISO 8601]

Partial Sign-Offs confirmed (Standard only):
  [ ] PARTIAL-[BATCH-ID]-[TASK-01]-[DATE]
  ...

DELIVERABLE CONFIRMATION (Simplified only; N/A for Standard)
  Deliverable path:  [file path]
  Exists on disk:    [ ] YES
  Git commit ref:    [hash]

───────────────────────────────────────────────────────────
PROJECT MEMORY CHECK
───────────────────────────────────────────────────────────
  [ ] PROJECT.md reviewed — still accurate for current product stage
  [ ] STATE.md updated and committed

───────────────────────────────────────────────────────────
BATCH-LEVEL ACCEPTANCE CRITERIA
───────────────────────────────────────────────────────────
  BAC-01: [ ✓ Met / ✗ Failed ]
  ...

───────────────────────────────────────────────────────────
COHERENCE CHECK (Standard only; N/A for Simplified)
───────────────────────────────────────────────────────────
  [ ] All Tasks together fully deliver the Batch Goal
  [ ] No Hard Boundary gaps exist between Tasks
  [ ] No unresolved Deviations affect the Batch Goal
  [ ] Documentation set complete

───────────────────────────────────────────────────────────
STATE.md UPDATE
───────────────────────────────────────────────────────────
  [ ] Verified Module Map updated
  [ ] Architectural Decisions updated
  [ ] Known Gotchas updated
  [ ] Adaptation Log prepended
  [ ] Test Baseline updated
  [ ] Carry-Forward Obligations updated
  [ ] STATE.md committed

───────────────────────────────────────────────────────────
TEST INTEGRITY VERIFICATION
───────────────────────────────────────────────────────────
  Standard Cycle:
  [ ] All tests satisfy T1 (falsifiable)
  [ ] Every Task has happy-path + error-path coverage (T2)
  [ ] Traceability maps every AC to a test and vice versa (T5)
  [ ] Critical/High Tasks have falsification results (T6)
  [ ] No defective tests unresolved

  Simplified Cycle:
  [ ] All tests satisfy T1
  [ ] Happy-path + error-path coverage present or absence justified (T2)
  [ ] Every AC maps to test and vice versa (T5)
  [ ] T6 falsification if Critical/High

  T1 violations:     [0]
  T2 violations:     [0]
  T5 coverage gaps:  [0]
  T6 unresolved:     [0]

───────────────────────────────────────────────────────────
DEFERRED TESTS SUMMARY
───────────────────────────────────────────────────────────
Carry forward all deferred tests from all Partial Sign-Offs.

  DEFER-01: [Test ID] — STATE.md entry: DEFER-BATCH-NN-TASK-NN-TEST-NN (status: OPEN)
Reconciled against STATE.md: [ ] YES   [ ] NO (BLOCKED if NO)

───────────────────────────────────────────────────────────
NOTES
───────────────────────────────────────────────────────────
Include: Reviewer fallback used (Y/N), Lead Override used (Y/N + count),
any Adaptations that require Blueprint corrections.

───────────────────────────────────────────────────────────
VERDICT
───────────────────────────────────────────────────────────
  [ ] APPROVED — Batch closed. Work merged into release target.
  [ ] RETURNED — See corrections below.

───────────────────────────────────────────────────────────
CORRECTIONS REQUIRED (only if RETURNED)
───────────────────────────────────────────────────────────

───────────────────────────────────────────────────────────
RELEASE TARGET
───────────────────────────────────────────────────────────
Version / tag this Batch is merged into:

───────────────────────────────────────────────────────────
LEAD PROGRAMMER SIGN
───────────────────────────────────────────────────────────
  Lead Name:   [Name]
  Timestamp:   [ISO 8601]

═══════════════════════════════════════════════════════════
```

---

## 5. SESSION LIFECYCLE FOR SPAWNED ROLES

### 5.1 Spawning Reviewer

- Use `permissionMode: "allow-all"`.
- Include in prompt: "After completing your review, send a message to the Lead session with summary: file written, commit hash, total flags, severity, recommendation."
- Wait for message (completion signal). If no message within Review SLA, send probe "Status?". If still no reply after 10 minutes, invoke Lead Override.
- After decision, dismiss: message "Review complete. Set status to 'done'."

### 5.2 Spawning Assistant

- Use `permissionMode: "allow-all"`.
- Include in prompt: "After completing your task, send a message to the Lead session with summary: files changed, test count, commit hash, any deviations or adaptations."
- Wait for message. Probe if no message within Execution SLA + 15 min.
- Lead verifies, signs, then dismisses.

### 5.3 Lead Override

If Assistant session is stalled (>60 min with no message and no deliverable), Lead may implement directly. The Task Report must record this as a Deviation, and the Partial Sign-Off must acknowledge self-review. Three consecutive overrides triggers a mandatory infrastructure halt.

---

## 6. SPRINT CHECKLISTS

### 6.1 Before Issuing Blueprint

```
[ ] Cycle mode determined and verified
[ ] All required fields populated
[ ] (Optional) PRECHECK.md run — findings translated into Hard Boundaries, Tasks, or STATE.md entries
  If STANDARD:
[ ] Hard Boundaries falsifiable
[ ] All Tasks defined with test IDs, acceptance criteria, traceability
[ ] Task dependencies non-circular
[ ] Test Baseline populated
[ ] STATE.md STATUS section completed
  If SIMPLIFIED:
[ ] All four eligibility conditions confirmed
[ ] Task Definition block completed
[ ] Lead Response section left blank
```

### 6.2 After Review Report

```
[ ] Review Report read
[ ] Lead Response section completed
[ ] Decision recorded
[ ] If ACCEPT WITH MODIFICATIONS: each flag acted on logged
[ ] If REJECT: Blueprint revised and re-sent (max 2 cycles)
[ ] If INVESTIGATIVE LAYER SKIPPED: Manual Investigative Review completed and attached
[ ] Accepted Blueprint + Review Report sent to Assistant
```

### 6.3 Before Partial Sign-Off

```
[ ] Task Implementation Report received
[ ] Lint Evidence: zero warnings, zero errors
[ ] Partial Sign-Off ID correct format
[ ] SLA compliance recorded
[ ] Deferred tests from Report carried forward
[ ] Verdict stated: APPROVED or RETURNED
[ ] If RETURNED: corrections itemized with Blueprint references
[ ] Document committed and filed
```

### 6.4 Before Batch Sign-Off Certificate

```
[ ] All Tasks have APPROVED Partial Sign-Offs (Standard)
[ ] Coherence check completed (Standard) or execution confirmed (Simplified)
[ ] Batch-level Acceptance Criteria evaluated
[ ] Deferred Tests Summary compiled
[ ] PROJECT.md reviewed and updated if needed
[ ] STATE.md updated and committed
[ ] All documents archived
[ ] Test Integrity Verification section completed (all counts 0)
[ ] Certificate filed and committed
```

---

## 7. STATE.md MAINTENANCE

At Batch Close, update STATE.md:
- Verified Module Map with any new/changed paths
- Architectural Decisions if new constraints introduced
- Known Gotchas if new surprises discovered
- Adaptation Log prepended with this Batch’s entries
- Test Baseline to final count
- Carry-Forward Obligations (add deferred tests, resolve completed ones)

Ensure STATE.md is committed. If STATE.md is missing or placeholder, the bootstrap Batch must create it with at least one real entry.

---

**This file is not authoritative alone. The complete, binding framework is AIV_FRAMEWORK_v5.4.md.**
