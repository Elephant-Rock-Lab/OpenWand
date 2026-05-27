#!/usr/bin/env bash
# Wave 03c — Automated binary E2E for approval approve/reject paths.
#
# Prerequisites:
#   - openwand.exe built at ../target/release/openwand.exe
#   - LM Studio running at http://100.64.0.1:1234 with a loaded model
#   - Model: qwen/qwen3-4b-2507
#
# Usage: bash e2e_approval.sh [approve|reject]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OPENWAND="$SCRIPT_DIR/../target/release/openwand.exe"

if [ ! -f "$OPENWAND" ]; then
    echo "ERROR: openwand.exe not found at $OPENWAND"
    echo "Build with: cargo build --release -p openwand-app"
    exit 1
fi

MODE="${1:-both}"
BASE_URL="http://100.64.0.1:1234/v1"
MODEL="qwen/qwen3-4b-2507"
API_KEY="lm-studio"

PASS=0
FAIL=0

run_test() {
    local name="$1"
    local input="$2"
    local db="$SCRIPT_DIR/e2e_approval_${name}.db"
    local expected_pattern="$3"

    rm -f "$db" 2>/dev/null || true

    echo ""
    echo "=== Test: $name ==="

    output=$(echo "$input" | timeout 120 "$OPENWAND" \
        --base-url "$BASE_URL" \
        --model "$MODEL" \
        --api-key "$API_KEY" \
        --db "$db" \
        "Create a file called e2e_${name}.txt with the content 'test'" 2>&1 || true)

    if echo "$output" | grep -q "$expected_pattern"; then
        echo "✅ PASS: $name"
        PASS=$((PASS + 1))
    else
        echo "❌ FAIL: $name"
        echo "Expected pattern: $expected_pattern"
        echo "Output:"
        echo "$output" | tail -20
        FAIL=$((FAIL + 1))
    fi

    # Check file was or wasn't created
    if [ "$name" = "reject" ]; then
        if [ -f "$SCRIPT_DIR/e2e_reject.txt" ]; then
            echo "❌ FAIL: File should NOT exist after rejection"
            FAIL=$((FAIL + 1))
        else
            echo "✅ File correctly not created after rejection"
        fi
    fi

    # Check trace in database
    if [ -f "$db" ]; then
        echo ""
        echo "Trace events:"
        python3 -c "
import sqlite3, sys
try:
    conn = sqlite3.connect('$db')
    for row in conn.execute('SELECT global_sequence, event_kind FROM trace_entry ORDER BY global_sequence'):
        print(f'  seq={row[0]:3d}  {row[1]}')
except Exception as e:
    print(f'  Error reading DB: {e}')
" 2>/dev/null || echo "  (could not read database)"
    fi

    rm -f "$db" "$SCRIPT_DIR/e2e_${name}.txt" 2>/dev/null || true
}

if [ "$MODE" = "both" ] || [ "$MODE" = "reject" ]; then
    run_test "reject" "n" "Rejected"
fi

if [ "$MODE" = "both" ] || [ "$MODE" = "approve" ]; then
    run_test "approve" "y" "Approved"
fi

echo ""
echo "==============================="
echo "Results: $PASS passed, $FAIL failed"
echo "==============================="

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
