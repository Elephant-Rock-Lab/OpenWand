#!/usr/bin/env bash
# verify_test_count.sh — Assert STATE.md test count matches cargo test output.
#
# Usage: bash scripts/verify_test_count.sh
# Exit 0 if counts match, exit 1 if they diverge.
#
# Canonical command (must match STATE.md recorded command):
#   cargo test --workspace --features "openwand-session/testing,openwand-session/sqlite-testing,openwand-memory/testing,openwand-memory/sqlite-testing"

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

# Read expected count from STATE.md (first number before "tests, zero failures" or "tests, zero")
EXPECTED=$(grep -oE '[0-9]+ tests' STATE.md | head -1 | grep -oE '^[0-9]+')

if [ -z "$EXPECTED" ]; then
    echo "ERROR: Could not find test count in STATE.md"
    echo "Expected format: 'NNNN tests'"
    exit 1
fi

echo "STATE.md expects: $EXPECTED tests"
echo "Running canonical test command..."

# Canonical command — same as lock checklist and STATE.md
ACTUAL=$(cargo test --workspace \
    --features "openwand-session/testing,openwand-session/sqlite-testing,openwand-memory/testing,openwand-memory/sqlite-testing" \
    2>&1 \
    | grep "test result:" \
    | grep -oE "[0-9]+ passed" \
    | awk '{sum += $1} END {print sum}')

if [ -z "$ACTUAL" ]; then
    echo "ERROR: Could not parse test output"
    exit 1
fi

echo "cargo test reports: $ACTUAL passed"

if [ "$ACTUAL" -eq "$EXPECTED" ]; then
    echo "MATCH: $ACTUAL == $EXPECTED"
    exit 0
else
    echo "MISMATCH: $ACTUAL != $EXPECTED"
    echo "Update STATE.md or investigate the divergence."
    exit 1
fi
