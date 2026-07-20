#!/usr/bin/env bash
# Fail if the vendored OpenAPI contract has drifted from the platform's.
#
# The contract test (tests/contract.rs) validates the SDK against the *vendored*
# copy at tests/openapi.json. That copy can silently go stale when casimirQ's
# API changes, so this guard diffs it against the canonical spec and fails on
# any difference. CI runs it against a freshly checked-out casimirQ; locally it
# defaults to the sibling checkout.
#
# Usage:
#   scripts/check-openapi-sync.sh [path-to-casimirQ-openapi.json]
set -euo pipefail

SRC="${1:-../casimirQ/openapi.json}"
DEST="$(dirname "$0")/../tests/openapi.json"

if [ ! -f "$SRC" ]; then
  echo "error: canonical spec not found at '$SRC'." >&2
  echo "  pass the path to casimirQ's openapi.json:" >&2
  echo "    scripts/check-openapi-sync.sh /path/to/casimirQ/openapi.json" >&2
  exit 2
fi

if diff -u "$DEST" "$SRC" > /tmp/openapi-sync.diff 2>&1; then
  echo "OpenAPI contract in sync: tests/openapi.json matches $SRC"
  exit 0
fi

echo "::error::vendored tests/openapi.json is STALE — it differs from casimirQ's openapi.json." >&2
echo "Refresh it and commit the result:" >&2
echo "  scripts/refresh-openapi.sh $SRC && git add tests/openapi.json" >&2
echo >&2
echo "--- drift (vendored vs canonical) ---" >&2
cat /tmp/openapi-sync.diff >&2
exit 1
