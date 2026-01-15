#!/bin/bash
# cleanup.sh - Full cleanup of a worktree (ports, directory, registry, optionally branch)
#
# Usage: ./cleanup.sh <project> <branch> [--delete-branch]
#        ./cleanup.sh --merged [--delete-branch]
#
# Examples:
#   ./cleanup.sh obsidian-ai-agent feature/auth                    # Cleanup worktree only
#   ./cleanup.sh obsidian-ai-agent feature/auth --delete-branch    # Also delete git branch
#   ./cleanup.sh --merged                                          # Cleanup all merged worktrees
#   ./cleanup.sh --merged --delete-branch                          # Also delete branches
#
# This script:
# 1. Kills processes on allocated ports
# 2. Removes the worktree directory
# 3. Prunes git worktree references
# 4. Removes entry from global registry
# 5. Releases ports back to pool
# 6. Optionally deletes local and remote git branches

set -e

REGISTRY="${HOME}/.claude/worktree-registry.json"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Handle --merged mode
if [ "$1" = "--merged" ]; then
    DELETE_BRANCH="${2:-}"

    echo "Finding worktrees with merged PRs..."
    echo "─────────────────────────────────────────────────────────"

    CLEANED=0
    SKIPPED=0

    while IFS= read -r entry; do
        [ -z "$entry" ] && continue

        PROJECT=$(echo "$entry" | jq -r '.project')
        BRANCH=$(echo "$entry" | jq -r '.branch')

        # Check PR status
        PR_INFO=$(gh pr list --head "$BRANCH" --state all --json number,state --limit 1 2>/dev/null || echo "[]")
        PR_STATE=$(echo "$PR_INFO" | jq -r '.[0].state // "NONE"')
        PR_NUM=$(echo "$PR_INFO" | jq -r '.[0].number // "?"')

        if [ "$PR_STATE" = "MERGED" ]; then
            echo ""
            echo "Found merged: $PROJECT / $BRANCH (PR #$PR_NUM)"
            "$SCRIPT_DIR/cleanup.sh" "$PROJECT" "$BRANCH" $DELETE_BRANCH
            CLEANED=$((CLEANED + 1))
        else
            SKIPPED=$((SKIPPED + 1))
        fi

    done < <(jq -c '.worktrees[]' "$REGISTRY" 2>/dev/null)

    echo ""
    echo "─────────────────────────────────────────────────────────"
    echo "Cleaned: $CLEANED worktrees"
    echo "Skipped: $SKIPPED worktrees (not merged)"
    exit 0
fi

PROJECT="$1"
BRANCH="$2"
DELETE_BRANCH="${3:-}"

if [ -z "$PROJECT" ] || [ -z "$BRANCH" ]; then
    echo "Usage: $0 <project> <branch> [--delete-branch]"
    echo "       $0 --merged [--delete-branch]"
    exit 1
fi

# Check jq is available
if ! command -v jq &> /dev/null; then
    echo "Error: jq is required. Install with: brew install jq"
    exit 1
fi

# Check if registry exists
if [ ! -f "$REGISTRY" ]; then
    echo "Error: No registry found at: $REGISTRY"
    exit 1
fi

echo "Cleaning up: $PROJECT / $BRANCH"
echo "─────────────────────────────────────────────────────────"

# Find worktree entry in registry
ENTRY=$(jq -c ".worktrees[] | select(.project == \"$PROJECT\" and .branch == \"$BRANCH\")" "$REGISTRY" 2>/dev/null || echo "")

if [ -z "$ENTRY" ]; then
    echo "Warning: Worktree not found in registry"
    echo "Attempting cleanup based on project/branch names..."
fi

# Get details from entry (or defaults)
if [ -n "$ENTRY" ]; then
    WORKTREE_PATH=$(echo "$ENTRY" | jq -r '.worktreePath')
    REPO_PATH=$(echo "$ENTRY" | jq -r '.repoPath')
    PORTS=$(echo "$ENTRY" | jq -r '.ports[]' 2>/dev/null || echo "")
else
    # Construct default path
    BRANCH_SLUG=$(echo "$BRANCH" | tr '/' '-')
    WORKTREE_PATH="$HOME/tmp/worktrees/$PROJECT/$BRANCH_SLUG"
    REPO_PATH=""
    PORTS=""
fi

# Step 1: Kill processes on ports
if [ -n "$PORTS" ]; then
    echo "Killing processes on allocated ports..."
    for PORT in $PORTS; do
        if lsof -ti:"$PORT" &>/dev/null; then
            lsof -ti:"$PORT" | xargs kill -9 2>/dev/null && echo "  Killed process on port $PORT" || echo "  Failed to kill on port $PORT"
        else
            echo "  Port $PORT: no process"
        fi
    done
else
    echo "No ports registered, skipping port cleanup"
fi

# Step 2: Remove worktree directory
if [ -d "$WORKTREE_PATH" ]; then
    echo "Removing worktree directory: $WORKTREE_PATH"

    # Try to find the main repo to run git worktree remove
    if [ -n "$REPO_PATH" ] && [ -d "$REPO_PATH" ]; then
        cd "$REPO_PATH"
        git worktree remove "$WORKTREE_PATH" --force 2>/dev/null && echo "  Removed via git worktree" || {
            echo "  Git worktree remove failed, removing directory directly..."
            rm -rf "$WORKTREE_PATH" && echo "  Removed directory"
        }
    else
        # Just remove the directory
        rm -rf "$WORKTREE_PATH" && echo "  Removed directory"
    fi
else
    echo "Worktree directory not found: $WORKTREE_PATH"
fi

# Step 3: Prune stale worktree references
if [ -n "$REPO_PATH" ] && [ -d "$REPO_PATH" ]; then
    echo "Pruning stale git worktree references..."
    cd "$REPO_PATH"
    git worktree prune 2>/dev/null && echo "  Pruned" || echo "  Prune skipped"
fi

# Step 4: Remove from registry and release ports
echo "Updating global registry..."
TMP=$(mktemp)

# Remove worktree entry
jq "del(.worktrees[] | select(.project == \"$PROJECT\" and .branch == \"$BRANCH\"))" "$REGISTRY" > "$TMP"

# Release ports from allocated pool
if [ -n "$PORTS" ]; then
    for PORT in $PORTS; do
        jq ".portPool.allocated = (.portPool.allocated | map(select(. != $PORT)))" "$TMP" > "${TMP}.2" && mv "${TMP}.2" "$TMP"
    done
    echo "  Released ports: $PORTS"
fi

mv "$TMP" "$REGISTRY"
echo "  Registry updated"

# Step 5: Delete git branches (if requested)
if [ "$DELETE_BRANCH" = "--delete-branch" ]; then
    if [ -n "$REPO_PATH" ] && [ -d "$REPO_PATH" ]; then
        cd "$REPO_PATH"

        echo "Deleting local branch: $BRANCH"
        git branch -D "$BRANCH" 2>/dev/null && echo "  Deleted local branch" || echo "  Local branch not found or is current branch"

        echo "Deleting remote branch: $BRANCH"
        git push origin --delete "$BRANCH" 2>/dev/null && echo "  Deleted remote branch" || echo "  Remote branch not found or already deleted"
    else
        echo "Cannot delete branches: original repo path not found"
    fi
fi

echo "─────────────────────────────────────────────────────────"
echo "✅ Cleanup complete: $PROJECT / $BRANCH"
