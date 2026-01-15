#!/bin/bash
# register.sh - Register a worktree in the global registry
#
# Usage: ./register.sh <project> <branch> <branch-slug> <worktree-path> <repo-path> <ports> [task]
#
# Arguments:
#   project       - Project name (e.g., "obsidian-ai-agent")
#   branch        - Full branch name (e.g., "feature/auth")
#   branch-slug   - Slugified branch (e.g., "feature-auth")
#   worktree-path - Full path to worktree
#   repo-path     - Full path to original repo
#   ports         - Comma-separated ports (e.g., "8100,8101")
#   task          - Optional task description
#
# Example:
#   ./register.sh obsidian-ai-agent feature/auth feature-auth \
#     ~/tmp/worktrees/obsidian-ai-agent/feature-auth \
#     ~/Projects/obsidian-ai-agent \
#     8100,8101 "Implement OAuth"

set -e

PROJECT="$1"
BRANCH="$2"
BRANCH_SLUG="$3"
WORKTREE_PATH="$4"
REPO_PATH="$5"
PORTS="$6"
TASK="${7:-}"

# Validate inputs
if [ -z "$PROJECT" ] || [ -z "$BRANCH" ] || [ -z "$BRANCH_SLUG" ] || [ -z "$WORKTREE_PATH" ] || [ -z "$REPO_PATH" ] || [ -z "$PORTS" ]; then
    echo "Usage: $0 <project> <branch> <branch-slug> <worktree-path> <repo-path> <ports> [task]"
    exit 1
fi

REGISTRY="${HOME}/.claude/worktree-registry.json"

# Check jq is available
if ! command -v jq &> /dev/null; then
    echo "Error: jq is required. Install with: brew install jq"
    exit 1
fi

# Expand ~ in paths
WORKTREE_PATH="${WORKTREE_PATH/#\~/$HOME}"
REPO_PATH="${REPO_PATH/#\~/$HOME}"

# Initialize registry if it doesn't exist
if [ ! -f "$REGISTRY" ]; then
    mkdir -p "$(dirname "$REGISTRY")"
    cat > "$REGISTRY" << 'EOF'
{
  "worktrees": [],
  "portPool": {
    "start": 8100,
    "end": 8199,
    "allocated": []
  }
}
EOF
fi

# Check if already registered - release old ports before updating
if jq -e ".worktrees[] | select(.project == \"$PROJECT\" and .branch == \"$BRANCH\")" "$REGISTRY" > /dev/null 2>&1; then
    echo "Warning: Worktree already registered, updating..."

    # Get old ports to release them
    OLD_PORTS=$(jq -r ".worktrees[] | select(.project == \"$PROJECT\" and .branch == \"$BRANCH\") | .ports[]" "$REGISTRY" 2>/dev/null || echo "")

    TMP=$(mktemp)
    # Remove old entry
    jq "del(.worktrees[] | select(.project == \"$PROJECT\" and .branch == \"$BRANCH\"))" "$REGISTRY" > "$TMP"

    # Release old ports from pool
    if [ -n "$OLD_PORTS" ]; then
        for OLD_PORT in $OLD_PORTS; do
            jq ".portPool.allocated = (.portPool.allocated | map(select(. != $OLD_PORT)))" "$TMP" > "${TMP}.2" && mv "${TMP}.2" "$TMP"
        done
        echo "   Released old ports: $OLD_PORTS"
    fi

    mv "$TMP" "$REGISTRY"
fi

# Parse ports into JSON array
PORTS_JSON=$(echo "$PORTS" | tr ',' '\n' | jq -R 'tonumber' | jq -s .)

# Format task
if [ -n "$TASK" ]; then
    TASK_JSON="\"$TASK\""
else
    TASK_JSON="null"
fi

# Generate UUID
UUID=$(uuidgen 2>/dev/null || cat /proc/sys/kernel/random/uuid 2>/dev/null || echo "wt-$(date +%s)-$$")

# Add entry
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
TMP=$(mktemp)
jq ".worktrees += [{
    \"id\": \"$UUID\",
    \"project\": \"$PROJECT\",
    \"repoPath\": \"$REPO_PATH\",
    \"branch\": \"$BRANCH\",
    \"branchSlug\": \"$BRANCH_SLUG\",
    \"worktreePath\": \"$WORKTREE_PATH\",
    \"ports\": $PORTS_JSON,
    \"createdAt\": \"$TIMESTAMP\",
    \"validatedAt\": null,
    \"agentLaunchedAt\": null,
    \"task\": $TASK_JSON,
    \"prNumber\": null,
    \"status\": \"active\"
}]" "$REGISTRY" > "$TMP" && mv "$TMP" "$REGISTRY"

echo "✅ Registered worktree:"
echo "   Project: $PROJECT"
echo "   Branch: $BRANCH"
echo "   Path: $WORKTREE_PATH"
echo "   Ports: $PORTS"
if [ -n "$TASK" ]; then
    echo "   Task: $TASK"
fi
