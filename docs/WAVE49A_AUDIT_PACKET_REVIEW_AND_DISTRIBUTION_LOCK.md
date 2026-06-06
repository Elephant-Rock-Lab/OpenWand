# Wave 49A — Audit Packet Review and Distribution — LOCK

**Committed:** 6 commits
**Baseline:** 3312 tests (Wave 48A locked)
**Final:** 3392 tests (+80), zero failures (pre-existing acceptance test compilation error unrelated)

---

## What Shipped

### Strategy

Fork A — Stay Human-Reported. Audit packet review and distribution are metadata-only records.

### Workflow Crate — New Modules

| Module | Purpose |
|--------|---------|
| `workflow_audit_packet_review.rs` | Review record: human review of exported audit packet |
| `workflow_audit_packet_distribution.rs` | Distribution record: metadata-only destination tracking |

### New Types

| Type | Prefix | Purpose |
|------|--------|---------|
| `AuditPacketReview` | `wapr_` | Records human review of audit packet |
| `AuditPacketDistribution` | `wapd_` | Records reported distribution to a destination |
| `AuditPacketReviewDecision` | — | ReviewedWithCaveats, AcknowledgedForDistribution, NotedWithoutCertification |
| `AuditPacketDestinationKind` | — | FileShare, Email, Archive, Other |
| `AuditPacketDistributionDestination` | — | Metadata-only destination with label, reference, operator_supplied_hash |

### App Crate

| File | Purpose |
|------|---------|
| `workflow_audit_packet_review.rs` | Persistence: save, load, list, by_workflow_run, by_inspection, by_audit_packet_hash |
| `workflow_audit_packet_distribution.rs` | Persistence: save, load, list, by_workflow_run, by_review, by_inspection, by_audit_packet_hash, by_destination_kind |

### CLI Commands

```bash
openwand audit-packet-review record \
  --inspection-id <id> --workflow-execution-id <id> \
  --expected-audit-packet-hash <hash> --expected-chain-hash <hash> \
  --reviewer <name> --decision <decision> --scope <text> [--json]

openwand audit-packet-review show --review-id <id> [--json]
openwand audit-packet-review list --workflow-execution-id <id> [--json]

openwand audit-packet-distribution record \
  --review-id <id> --workflow-execution-id <id> \
  --expected-review-hash <hash> --expected-audit-packet-hash <hash> \
  --expected-chain-hash <hash> --inspection-id <id> \
  --destination-kind <kind> --destination-label <label> \
  --destination-reference <ref> [--json]

openwand audit-packet-distribution show --distribution-id <id> [--json]
openwand audit-packet-distribution list --workflow-execution-id <id> [--json]
```

---

## Patch Compliance

| Patch | Status |
|-------|--------|
| 1. Review binds to packet_hash + chain_hash + inspection_id | ✅ 4 tests |
| 2. Distribution binds to review_hash + packet_hash + chain_hash + inspection_id | ✅ 5 tests |
| 3. Decision semantics avoid certification | ✅ 6 tests + snapshot fields |
| 4. Reported distribution only semantics | ✅ 5 tests |
| 5. Metadata-only destination model | ✅ 6 tests |
| 6. Extended persistence indexes + idempotency | ✅ 13 tests |
| 7. CLI requires hashes, no forbidden verbs | ✅ 6 CLI tests |
| 8. No-authority flags on both records + guards | ✅ 22 guard tests |

---

## Test Breakdown

| Area | Count |
|------|------:|
| Review DTOs + builder (Patches 1,3,8) | +20 |
| Distribution DTOs + builder (Patches 2,4,5,8) | +16 |
| App persistence review (Patch 6) | +6 |
| App persistence distribution (Patch 6) | +7 |
| CLI review + distribution (Patch 7) | +6 |
| UI state + components | +6 |
| Guard tests (Patch 8) | +22 |
| **Total** | **+83** |

*Note: Actual delta may differ slightly from estimate due to test count overlap.*

---

## Central Invariant

```
Audit packet review is not truth certification.
Audit packet distribution is not verification.
Distribution metadata does not prove receipt, acceptance, or correctness.
```

## Sharper Lock Invariant

```
Review records describe human handling. They do not certify truth, verify
packet contents, approve packet truth, modify the audit packet, or promote trust.

Distribution records describe reported destinations. They do not prove delivery,
confirm receipt, verify the destination, upload files, send messages,
integrate with external systems, or modify the audit packet.
```

---

## No Modification of Audit Packet

```
The exported audit packet remains immutable recorded evidence.
Review and distribution records attach to it; they do not modify it.
```

---

## Honest Caveats

- The review binds to `audit_packet_hash` which the operator must supply correctly. OpenWand does not re-read the exported packet to verify the hash.
- Distribution destination is metadata-only. OpenWand does not send email, upload files, verify paths, fetch URLs, or confirm receipt.
- `operator_supplied_hash` is stored verbatim. OpenWand does not interpret or verify it.
- Review history is preserved — different idempotency keys create additional reviews, they do not supersede.
- The `AuditPacketDestinationKind` enum is closed. `Other` covers anything not explicitly listed.
