# PROJECT INTENT

Last Updated:       [YYYY-MM-DD — update when project priorities shift]
Maintained by:      Lead Programmer
Audience:           All AIV sessions (Reviewer, Assistant), new contributors
Status:             [ACTIVE / STALE — review if more than 10 Batches since last update]

───────────────────────────────────────────────────────────
PROJECT VISION
───────────────────────────────────────────────────────────
What is this product, who uses it, and what problem does it solve?
[2–3 sentences that a new session can internalise in 30 seconds. Think elevator pitch.]


───────────────────────────────────────────────────────────
CORE VALUE PROPOSITIONS (VPs)
───────────────────────────────────────────────────────────
The 1–3 things that make users choose this over alternatives.
These are non-negotiable quality axes — anything that degrades these is a regression,
even if all automated tests pass.

  VP-1: [Description — e.g. "Reliable offline sync: users trust that no data is ever lost, even across crashes."]
  VP-2: [Description]
  VP-3: [Description — leave empty if only two]

───────────────────────────────────────────────────────────
USER PERSONAS & CRITICAL WORKFLOWS
───────────────────────────────────────────────────────────
Who are the primary users? What are the sequences of actions that matter most to them?
For each persona, define the workflow that must NEVER break — and what breaking it means.

  ── PERSONA 1 ───────────────────────────────────────────
  Name:              [e.g. Field Agent]
  Priority:          [Critical / High / Medium]
  Context:           [When/where/how they use the product]
  Critical Workflow: [Step-by-step, from their perspective. Include emotional beats if relevant.]
  Failure Impact:    [What happens if this breaks — in user terms, not technical terms.]

  ── PERSONA 2 ───────────────────────────────────────────
  Name:
  Priority:
  Context:
  Critical Workflow:
  Failure Impact:

  [Duplicate block for each persona. Keep to the top 2–3.]

───────────────────────────────────────────────────────────
QUALITY ATTRIBUTES
───────────────────────────────────────────────────────────
Architectural drivers that should inform every design tradeoff.
1 = lowest priority, 5 = non-negotiable. Rank honestly — this shapes scope decisions.

  Attribute        | Priority (1–5) | Note
  -----------------|----------------|-------------------------------------------------
  Performance      |                | [e.g. "Sub-100ms p95 for all local reads" or N/A]
  Security         |                | [e.g. "Auth integrity; no PII exposure"]
  Reliability      |                | [e.g. "Zero data loss — see VP-1"]
  Usability        |                | [e.g. "All workflows have a GUI path"]
  Maintainability  |                | [e.g. "Code must be testable by new hires"]
  Scalability      |                | [e.g. "Single-user only" or "Horizontal scale expected"]
  Observability    |                | [e.g. "All API calls must emit structured logs"]

───────────────────────────────────────────────────────────
ENGINEERING PHILOSOPHY
───────────────────────────────────────────────────────────
Rules of thumb and conventions that go beyond what linters catch.
These are tribal knowledge made explicit.

  - [e.g. Prefer explicit error types over .unwrap() in library code.]
  - [e.g. Async is allowed only at the outer layer; core logic stays synchronous.]
  - [e.g. No dependency on nightly Rust; must build on stable.]
  - [e.g. Refactoring for testability is always allowed — but ask before changing public API signatures.]

───────────────────────────────────────────────────────────
CURRENT PRODUCT STAGE & IMMEDIATE PRIORITIES
───────────────────────────────────────────────────────────
What phase is the product in, and what’s the most important thing RIGHT NOW?
This changes over time — update it when the focus shifts.

  Stage:              [Concept / Alpha / Beta / GA / Maintenance]
  #1 Priority:        [e.g. "Stabilise the external API before the beta launch."]
  Accepted Tradeoff:  [e.g. "Velocity over code elegance until the next milestone."]
  Key Deadline:       [If any — "v1.0 must ship by Q3 2026." Otherwise: None.]

───────────────────────────────────────────────────────────
KNOWN FRICTION POINTS
───────────────────────────────────────────────────────────
Parts of the codebase or user experience that are fragile, duplicated, confusing, or overdue for redesign.
Not bugs, not gotchas — these are judgment signals. If you touch these areas, tread carefully.

  FP-01: [Description — e.g. "The onboarding wizard validation is brittle; if you change config parsing, manually test the entire wizard flow."]
  FP-02: [Description — e.g. "Payment reconciliation logic is duplicated across two modules; don't add a third copy without consolidating first."]
  FP-03: [Description]

───────────────────────────────────────────────────────────
FUTURE ROADMAP (NEAR TERM — NEXT 3–5 BATCHES)
───────────────────────────────────────────────────────────
What’s likely to be built soon? Helps sessions avoid painting into a corner.
Update when the roadmap shifts. Keep it brief.

  BATCH-expected-01: [e.g. "Email notifications — requires a new outbound transport crate; keep communication logic decoupled."]
  BATCH-expected-02: [e.g. "Multi-tenancy — do NOT assume single-user in any new database queries from now on."]
  BATCH-expected-03:
  BATCH-expected-04:
  BATCH-expected-05:

───────────────────────────────────────────────────────────
EXTERNAL CONSTRAINTS & COMPLIANCE
───────────────────────────────────────────────────────────
Regulatory, contractual, or third-party constraints that the code must respect.
If none, write "None."

  - [e.g. GDPR: all user data must be exportable and deletable.]
  - [e.g. SOC2: all admin actions must be logged to an append-only audit table.]
  - [e.g. License: cannot use GPL libraries in this proprietary project.]

═══════════════════════════════════════════════════════════
