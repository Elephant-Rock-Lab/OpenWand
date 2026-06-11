#!/usr/bin/env bash
# Binary E2E test: approval approve/reject paths through the real CLI binary.
#
# Proves:
#   - Approval path: approved file-write creates the file with expected contents
#   - Rejection path: rejected file-write creates no file
#   - Summary reports honest stop reason (ToolDenied on rejection, not Natural)
#
# Requires: a running LLM provider (LM Studio / Ollama / etc.)
# Usage: bash tests/e2e_approval.sh [path-to-openwand-binary] [base-url] [model]
#
# Exit code 0 = all assertions passed
# Exit code 1 = failure

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OPENWAND="${1:-$SCRIPT_DIR/../target/release/openwand.exe}"
BASE_URL="${2:-http://100.64.0.1:1234/v1}"
MODEL="${3:-qwen/qwen3-4b-2507}"
API_KEY="lm-studio"

PASS=0
FAIL=0

# Check binary exists
if [ ! -f "$OPENWAND" ]; then
    echo "ERROR: openwand binary not found at $OPENWAND"
    echo "Build with: cargo build --release -p openwand-app"
    exit 1
fi

# Check LLM is reachable
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/models" 2>/dev/null || echo "000")
if [ "$HTTP_CODE" != "200" ]; then
    echo "FAIL: LLM not reachable at $BASE_URL (HTTP $HTTP_CODE)"
    exit 1
fi

run_test() {
    local name="$1"
    local input="$2"
    local expect_approved="$3"
    local db="$SCRIPT_DIR/e2e_approval_${name}.db"
    local workspace="$SCRIPT_DIR/e2e_workspace_${name}"
    local target_file="$workspace/e2e_${name}.txt"
    local target_content="test content from e2e ${name}"

    rm -rf "$db" "$workspace" 2>/dev/null || true
    mkdir -p "$workspace"

    echo ""
    echo "=== Test: $name ==="

    # Run the CLI with stdin piped for approval response
    output=$(echo "$input" | timeout 120 "$OPENWAND" \
        run \
        --base-url "$BASE_URL" \
        --model "$MODEL" \
        --api-key "$API_KEY" \
        --db "$db" \
        "Create a file called e2e_${name}.txt in the workspace with the content '${target_content}'" 2>&1 || true)

    echo "  Output (last 15 lines):"
    echo "$output" | tail -15

    if [ "$expect_approved" = true ]; then
        # ── Approval assertions ──

        # 1. Output says Approved
        if echo "$output" | grep -q "Approved"; then
            echo "  OK Output reports Approved"
            PASS=$((PASS + 1))
        else
            echo "  FAIL: Output does not report Approved"
            FAIL=$((FAIL + 1))
        fi

        # 2. File exists
        if [ -f "$target_file" ]; then
            echo "  OK File exists at $target_file"
            PASS=$((PASS + 1))
        else
            echo "  FAIL: File does NOT exist at $target_file (approval should have created it)"
            FAIL=$((FAIL + 1))
        fi

        # 3. File contents match
        if [ -f "$target_file" ]; then
            actual=$(cat "$target_file")
            if [ "$actual" = "$target_content" ]; then
                echo "  OK File contents match expected payload"
                PASS=$((PASS + 1))
            else
                echo "  FAIL: File contents mismatch"
                echo "    Expected: $target_content"
                echo "    Actual:   $actual"
                FAIL=$((FAIL + 1))
            fi
        fi

        # 4. Stop reason is NOT ToolDenied
        if echo "$output" | grep -q "ToolDenied"; then
            echo "  FAIL: Approved run should NOT report ToolDenied"
            FAIL=$((FAIL + 1))
        else
            echo "  OK Stop reason is not ToolDenied"
            PASS=$((PASS + 1))
        fi

    else
        # ── Rejection assertions ──

        # 1. Output says Rejected
        if echo "$output" | grep -qi "reject"; then
            echo "  OK Output reports rejection"
            PASS=$((PASS + 1))
        else
            echo "  FAIL: Output does not report rejection"
            FAIL=$((FAIL + 1))
        fi

        # 2. File does NOT exist
        if [ -f "$target_file" ]; then
            echo "  FAIL: File EXISTS at $target_file (rejection should NOT have created it)"
            FAIL=$((FAIL + 1))
        else
            echo "  OK File correctly not created after rejection"
            PASS=$((PASS + 1))
        fi

        # 3. Stop reason is ToolDenied (not Natural)
        if echo "$output" | grep -q "ToolDenied"; then
            echo "  OK Stop reason is ToolDenied"
            PASS=$((PASS + 1))
        else
            echo "  FAIL: Rejection should report ToolDenied, not Natural"
            FAIL=$((FAIL + 1))
        fi
    fi

    # Cleanup
    rm -rf "$db" "$workspace" 2>/dev/null || true
}

echo "=== Approval E2E ==="
echo "Binary:  $OPENWAND"
echo "Base URL: $BASE_URL"
echo "Model:   $MODEL"

run_test "reject" "n" false
run_test "approve" "y" true

echo ""
echo "==============================="
echo "Results: $PASS passed, $FAIL failed"
echo "==============================="

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
