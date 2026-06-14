# OpenWand v0.7.0 Roadmap

**Theme:** External assurance and operational completeness.

**Status:** Complete (Wave 107A)

---

## Context

v0.6.0 hardened internal evidence: the verifier recomputes BLAKE3 hashes under
`Blake3HashPolicy`, and trace-backed correspondence exists for workflow initiation
and evidence export in new traces.

However, all verification remains self-referential: the system checks its own
hashes against its own store. An attacker who can rewrite the store AND recompute
all hashes produces a self-consistent trace. The documented limitation states:

> Full immutability requires an external trust anchor (signature, checkpoint, or
> append-only storage guarantee), which is out of scope [for v0.6.0].

v0.7.0 moves beyond self-consistency toward externally trustable evidence.

---

## Architecture Arc

```
v0.2  governed execution substrate
v0.3  live observation
v0.4  desktop operation requests
v0.5  read-only verification
v0.6  evidence-backed assurance hardening
v0.7  external assurance and operational completeness
```

```
Control → Observe → Operate → Verify → Harden → Externally Anchor
```

---

## Blockers (VG series)

| Blocker | Description | Priority |
|---------|-------------|----------|
| VG-1 | External anchor / checkpoint design | P1 (core) |
| VG-2 | Security review execution | P1 (core) |
| VG-3 | Linux GUI runtime validation | P2 (environment-gated) |
| VG-4 | Provider validation expansion | P2 (strategic) |
| VG-5 | Evidence UX hardening | P2 |

---

### VG-1: External Anchor / Checkpoint Design

**Problem:** Even with hash recomputation (v0.6.0), an attacker who controls the
trace store can rewrite entries AND recompute all hashes to produce a
self-consistent trace. The verifier cannot detect this.

**Goal:** Design and implement an integrity checkpoint mechanism that anchors
trace state to something the attacker cannot forge.

**Candidate approaches:**

| Approach | Description | Tradeoff |
|----------|-------------|----------|
| A: Periodic checkpoint hash | Compute a root hash over all entries at intervals; persist to external location (file outside store, signed log) | Simple; protects against silent rewrite if anchor is outside attacker's control |
| B: Ed25519 signature anchoring | Sign checkpoint hashes with an Ed25519 key; verifier checks signature | Stronger; key management complexity |
| C: Append-only storage mode | Configure SQLite in WAL/append-only mode with filesystem-level protections | Infrastructure-dependent; not portable |

**Recommended scope for v0.7.0:** Approach A (periodic checkpoint hash persisted
to a file outside the trace store root). This is the simplest mechanism that
breaks self-consistency: the verifier compares the checkpoint hash to the
externally stored anchor, and a mismatch proves the store was modified after the
checkpoint was taken.

**Authority boundary:** Checkpoint creation is a service-boundary operation.
The verifier reads checkpoints and anchors. It does not create, sign, or mutate
them.

**What this does NOT claim:**
- Real-time tamper prevention (detects after the fact)
- Protection against an attacker who controls both the store and the anchor location
- Cryptographic non-repudiation (requires signatures, Approach B)

---

### VG-2: Security Review Execution

**Problem:** Wave 94A created `docs/SECURITY_REVIEW_PREP.md` — a comprehensive
preparation document with threat model, authority-boundary checklist, 11 caveats,
and review-ready assets. But no actual security review has been performed.

**Goal:** Convert the preparation into an executed review or an external review
packet ready for submission.

**Scope options:**

| Option | Description | Effort |
|--------|-------------|--------|
| A: Internal structured review | Walk through each checklist item, produce findings, update caveats | Medium |
| B: External review packet | Package all assets into a reviewable artifact (zip, GitHub Security Advisory) | Low packaging, external dependency for review |
| C: Automated security scanning | Run `cargo audit`, `cargo deny`, dependency analysis; document results | Low |

**Recommended scope for v0.7.0:** Option C (automated scanning + structured
internal review of the authority boundary). This is the most actionable without
external dependencies and directly strengthens the evidence behind security claims.

**What this does NOT claim:**
- External security audit completion
- Zero vulnerabilities (only documents what was checked and what was found)

---

### VG-3: Linux GUI Runtime Validation

**Problem:** OpenWand compiles on Linux (WSL2 Ubuntu, Rust 1.96.0, webkit2gtk-4.1)
but the GUI has never been runtime-validated. No display server is available in
the development environment.

**Goal:** Validate the desktop GUI launches and renders on Linux.

**Blocker:** Requires a Linux environment with a display server (X11 or Wayland).
This is environment-gated and may not be resolvable in the current development
setup.

**If unresolvable:** Continue to defer. Compile-validation remains the gate.

---

### VG-4: Provider Validation Expansion

**Problem:** Provider validation is limited to 5 models across 2 provider
families (LM Studio and Z.AI). Direct OpenAI, Anthropic, and Ollama coverage
remains deferred.

**Goal:** Add at least one direct provider validation if strategically needed.

**Scope:** Depends on API key availability and strategic priority. This is
marked P2 and may be deferred if no strategic value.

---

### VG-5: Evidence UX Hardening

**Problem:** Verification results (trace-verify, operation-replay) produce text
output. External reviewers need human-readable, exportable evidence.

**Goal:** Make verification results understandable and exportable.

**Candidate scope:**
- Verification report export (JSON or HTML)
- Human-readable summary with pass/fail/inconclusive per check
- Caveat inclusion in exported reports

**Authority boundary:** Export is observation. The verifier produces the report;
it does not modify traces or operations.

---

## Proposed Wave Sequence

| Wave | Title | Blocker | Description |
|------|-------|---------|-------------|
| 103A | Post-v0.6 Roadmap Reset | — | This document; VG blockers proposed |
| 104A | External Anchor Design | VG-1 | Checkpoint hash design + implementation |
| 104B | External Anchor CLI | VG-1 | `openwand checkpoint-create` / `checkpoint-verify` CLI |
| 105A | Automated Security Scanning | VG-2 | `cargo audit`, dependency analysis, documented results |
| 105B | Structured Authority Review | VG-2 | Walk through checklist, produce findings |
| 106A | Evidence Report Export | VG-5 | Verification report → exportable format |
| 107A | v0.7.0 Release Preparation | — | Reconcile blockers, release notes |
| 107B | v0.7.0 Declaration | — | Tag v0.7.0 |

**Note:** Wave numbering may shift based on findings. VG-3 (Linux GUI) and VG-4
(provider expansion) are environment/strategy-gated and may be deferred.

---

## Deferred Items (from prior arcs)

| Item | Origin | Status |
|------|--------|--------|
| Linux GUI runtime | VE-3 / VF-4 / VG-3 | Environment-gated; compile-validated only |
| Direct OpenAI/Anthropic/Ollama | Provider matrix caveat | Deferred; LM Studio + Z.AI validated |
| macOS runtime | Platform caveat | No environment |
| External trust anchor (full) | DEFERRED-004 | VG-1 begins this work; full signature anchoring may require v0.8+ |
| Stable API guarantee | v0.5.0 caveat | Post-production-readiness |
| Production readiness | Global caveat | Not claimed for any version |

---

## Success Criteria for v0.7.0

1. **At least one external anchor mechanism exists** and is verifiable through CLI
2. **Security scanning results documented** with actionable findings
3. **Verification reports are exportable** in at least one structured format
4. **No new write authority for the verifier** — it reads and reports, nothing more
5. **Legacy traces remain backward compatible** — no retroactive failures

---

## What v0.7.0 is NOT

- Not production-ready
- Not a formal external security audit
- Not full cryptographic non-repudiation (requires signature infrastructure)
- Not real-time tamper prevention
- Not cross-platform runtime validation (unless environment allows)
- Not a stable API guarantee
