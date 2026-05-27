#!/bin/bash
# Binary E2E test: proves memory loop works through the real CLI binary.
# 
# Usage: bash tests/e2e_binary.sh [path-to-openwand-binary]
#
# Exit code 0 = all assertions passed
# Exit code 1 = failure

set -euo pipefail

BINARY="${1:-./target/release/openwand.exe}"
DB="$(mktemp -u /tmp/openwand_e2e_XXXXXX.db)"
BASE_URL="http://100.64.0.1:1234/v1"
MODEL="qwen/qwen3-4b-2507"
API_KEY="lm-studio"

# Check binary exists
if [ ! -f "$BINARY" ]; then
    echo "FAIL: Binary not found at $BINARY"
    echo "Build with: cargo build --release -p openwand-app"
    exit 1
fi

# Check LLM is reachable
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/models" 2>/dev/null || echo "000")
if [ "$HTTP_CODE" != "200" ]; then
    echo "FAIL: LLM not reachable at $BASE_URL (HTTP $HTTP_CODE)"
    exit 1
fi

echo "=== Binary E2E: Memory Loop ==="
echo "Binary: $BINARY"
echo "DB:     $DB"
echo ""

# ── Turn 1: Store memory ──
echo "Turn 1: Storing memory..."
OUTPUT1=$("$BINARY" \
    --base-url "$BASE_URL" \
    --model "$MODEL" \
    --api-key "$API_KEY" \
    --db "$DB" \
    "Remember that I always use Rust for new projects" 2>&1) || {
    echo "FAIL: Turn 1 exited with error"
    echo "$OUTPUT1"
    exit 1
}

# Assert: memory projection accepted at least 1 record
if ! echo "$OUTPUT1" | grep -q "Records accepted:    [1-9]"; then
    echo "FAIL: Turn 1 did not accept any memory records"
    echo "$OUTPUT1"
    exit 1
fi
echo "  ✓ Memory stored"

# ── Turn 2: Retrieve memory via semantically related query ──
echo "Turn 2: Retrieving memory..."
OUTPUT2=$("$BINARY" \
    --base-url "$BASE_URL" \
    --model "$MODEL" \
    --api-key "$API_KEY" \
    --db "$DB" \
    "What programming language should I use for my new project?" 2>&1) || {
    echo "FAIL: Turn 2 exited with error"
    echo "$OUTPUT2"
    exit 1
}

# Assert: LLM response references Rust
if ! echo "$OUTPUT2" | grep -qi "rust"; then
    echo "FAIL: Turn 2 response does not mention Rust"
    echo "  LLM did not use the memory"
    echo "$OUTPUT2"
    exit 1
fi
echo "  ✓ LLM response references Rust — memory was retrieved and injected"

# ── Cleanup ──
rm -f "$DB"

echo ""
echo "=== ALL ASSERTIONS PASSED ==="
echo "Turn 1: Memory stored ✓"
echo "Turn 2: Memory retrieved and used by LLM ✓"
