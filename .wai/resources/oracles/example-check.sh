#!/usr/bin/env bash
# example-check.sh — sample oracle that verifies artifact has content
# Exit 0 = pass, non-zero = fail. Stderr is shown on failure.
set -euo pipefail

FILE="$1"
if [ ! -s "$FILE" ]; then
    echo "Artifact is empty: $FILE" >&2
    exit 1
fi
