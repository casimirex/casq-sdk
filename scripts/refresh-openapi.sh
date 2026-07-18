#!/usr/bin/env bash
# Refresh the vendored OpenAPI contract from the casimirQ platform.
#
# Run this whenever the platform's API changes so the contract test
# (tests/contract.rs) validates against the current spec.
set -euo pipefail

SRC="${1:-../casimirQ/openapi.json}"
DEST="$(dirname "$0")/../tests/openapi.json"

if [ ! -f "$SRC" ]; then
  echo "error: $SRC not found. Pass the path to casimirQ's openapi.json:" >&2
  echo "  scripts/refresh-openapi.sh /path/to/casimirQ/openapi.json" >&2
  exit 1
fi

cp "$SRC" "$DEST"
echo "Refreshed $DEST from $SRC"
