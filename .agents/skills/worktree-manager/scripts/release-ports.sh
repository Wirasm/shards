#!/bin/bash
# release-ports.sh - Release ports back to the global pool
#
# Usage: ./release-ports.sh <port1> [port2] [port3] ...
#
# Example: ./release-ports.sh 8100 8101

set -e

if [ $# -eq 0 ]; then
    echo "Usage: $0 <port1> [port2] [port3] ..."
    exit 1
fi

REGISTRY="${HOME}/.claude/worktree-registry.json"

# Check jq is available
if ! command -v jq &> /dev/null; then
    echo "Error: jq is required. Install with: brew install jq"
    exit 1
fi

# Check if registry exists
if [ ! -f "$REGISTRY" ]; then
    echo "No registry found, nothing to release"
    exit 0
fi

TMP=$(mktemp)
cp "$REGISTRY" "$TMP"

RELEASED=()
for PORT in "$@"; do
    if [[ "$PORT" =~ ^[0-9]+$ ]]; then
        jq ".portPool.allocated = (.portPool.allocated | map(select(. != $PORT)))" "$TMP" > "${TMP}.2" && mv "${TMP}.2" "$TMP"
        RELEASED+=("$PORT")
    else
        echo "Warning: Invalid port number: $PORT" >&2
    fi
done

mv "$TMP" "$REGISTRY"

if [ ${#RELEASED[@]} -gt 0 ]; then
    echo "Released ports: ${RELEASED[*]}"
else
    echo "No ports released"
fi
