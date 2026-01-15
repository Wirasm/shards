#!/bin/bash
# allocate-ports.sh - Allocate N unused ports from global pool
#
# Usage: ./allocate-ports.sh <count>
#
# Returns: Space-separated list of port numbers
# Example: ./allocate-ports.sh 2 → "8100 8101"
#
# Also updates the registry to mark ports as allocated

set -e

COUNT="${1:-2}"

# Validate input
if ! [[ "$COUNT" =~ ^[0-9]+$ ]] || [ "$COUNT" -lt 1 ]; then
    echo "Error: Count must be a positive integer" >&2
    exit 1
fi

# Find config and registry
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CONFIG_FILE="$SCRIPT_DIR/../config.json"
REGISTRY="${HOME}/.claude/worktree-registry.json"

# Load port pool from config
if [ -f "$CONFIG_FILE" ] && command -v jq &> /dev/null; then
    PORT_START=$(jq -r '.portPool.start // 8100' "$CONFIG_FILE")
    PORT_END=$(jq -r '.portPool.end // 8199' "$CONFIG_FILE")
else
    PORT_START=8100
    PORT_END=8199
fi

# Check jq is available
if ! command -v jq &> /dev/null; then
    echo "Error: jq is required. Install with: brew install jq" >&2
    exit 1
fi

# Initialize registry if it doesn't exist
if [ ! -f "$REGISTRY" ]; then
    mkdir -p "$(dirname "$REGISTRY")"
    cat > "$REGISTRY" << EOF
{
  "worktrees": [],
  "portPool": {
    "start": $PORT_START,
    "end": $PORT_END,
    "allocated": []
  }
}
EOF
fi

# Get currently allocated ports from registry
ALLOCATED_PORTS=$(jq -r '.portPool.allocated // [] | .[]' "$REGISTRY" 2>/dev/null || echo "")

# Function to check if port is in use (by system or in registry)
is_port_available() {
    local port=$1

    # Check if in registry's allocated list
    if echo "$ALLOCATED_PORTS" | grep -q "^${port}$"; then
        return 1
    fi

    # Check if port is actually in use by system
    if lsof -i :"$port" &>/dev/null; then
        return 1
    fi

    return 0
}

# Find available ports
FOUND_PORTS=()
for ((port=PORT_START; port<=PORT_END; port++)); do
    if is_port_available "$port"; then
        FOUND_PORTS+=("$port")
        if [ ${#FOUND_PORTS[@]} -eq "$COUNT" ]; then
            break
        fi
    fi
done

# Check if we found enough ports
if [ ${#FOUND_PORTS[@]} -lt "$COUNT" ]; then
    echo "Error: Could not find $COUNT available ports in range $PORT_START-$PORT_END" >&2
    echo "Found only: ${FOUND_PORTS[*]}" >&2
    exit 1
fi

# Update registry with newly allocated ports
TMP=$(mktemp)
PORTS_JSON=$(printf '%s\n' "${FOUND_PORTS[@]}" | jq -R 'tonumber' | jq -s .)
jq ".portPool.allocated = ((.portPool.allocated // []) + $PORTS_JSON | unique | sort_by(.))" "$REGISTRY" > "$TMP" && mv "$TMP" "$REGISTRY"

# Output the ports (space-separated)
echo "${FOUND_PORTS[*]}"
