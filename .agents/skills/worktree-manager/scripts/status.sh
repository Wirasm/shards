#!/bin/bash
# status.sh - Show status of all managed worktrees globally
#
# Usage: ./status.sh [--project <name>]
#
# Options:
#   --project <name>   Filter to show only worktrees for specific project
#
# Examples:
#   ./status.sh                              # All worktrees
#   ./status.sh --project obsidian-ai-agent  # Only this project

set -e

PROJECT_FILTER=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --project)
            PROJECT_FILTER="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1" >&2
            echo "Usage: $0 [--project <name>]" >&2
            exit 1
            ;;
    esac
done

REGISTRY="${HOME}/.claude/worktree-registry.json"

# Check if registry exists
if [ ! -f "$REGISTRY" ]; then
    echo "No worktree registry found at: $REGISTRY"
    echo ""
    echo "No worktrees are being tracked yet."
    exit 0
fi

# Check if jq is available
if ! command -v jq &> /dev/null; then
    echo "Error: jq is required but not installed"
    echo "Install with: brew install jq"
    exit 1
fi

echo "═══════════════════════════════════════════════════════════════════════════════"
echo "                        GLOBAL WORKTREE STATUS"
if [ -n "$PROJECT_FILTER" ]; then
    echo "                        (filtered: $PROJECT_FILTER)"
fi
echo "═══════════════════════════════════════════════════════════════════════════════"
echo ""

# Get worktrees, optionally filtered
if [ -n "$PROJECT_FILTER" ]; then
    WORKTREES=$(jq -c ".worktrees[] | select(.project == \"$PROJECT_FILTER\")" "$REGISTRY" 2>/dev/null || echo "")
else
    WORKTREES=$(jq -c '.worktrees[]' "$REGISTRY" 2>/dev/null || echo "")
fi

if [ -z "$WORKTREES" ]; then
    if [ -n "$PROJECT_FILTER" ]; then
        echo "No worktrees found for project: $PROJECT_FILTER"
    else
        echo "No worktrees registered."
    fi
    echo ""

    # Show port pool status
    echo "Port Pool:"
    ALLOCATED=$(jq -r '.portPool.allocated | length' "$REGISTRY" 2>/dev/null || echo "0")
    START=$(jq -r '.portPool.start // 8100' "$REGISTRY")
    END=$(jq -r '.portPool.end // 8199' "$REGISTRY")
    TOTAL=$((END - START + 1))
    echo "  Allocated: $ALLOCATED / $TOTAL"
    exit 0
fi

# Print header
printf "%-25s %-20s %-12s %-8s %-10s %s\n" "PROJECT" "BRANCH" "PORTS" "PR" "STATUS" "TASK"
printf "%-25s %-20s %-12s %-8s %-10s %s\n" "─────────────────────────" "────────────────────" "────────────" "────────" "──────────" "──────────────────────"

# Process each worktree
echo "$WORKTREES" | while read -r wt; do
    PROJECT=$(echo "$wt" | jq -r '.project')
    BRANCH=$(echo "$wt" | jq -r '.branch')
    PORTS=$(echo "$wt" | jq -r '.ports | join(",")')
    TASK=$(echo "$wt" | jq -r 'if .task == null then "-" else .task end')
    PR=$(echo "$wt" | jq -r 'if .prNumber == null then "-" else .prNumber end')
    STATUS=$(echo "$wt" | jq -r '.status // "active"')
    WORKTREE_PATH=$(echo "$wt" | jq -r '.worktreePath')
    REPO_PATH=$(echo "$wt" | jq -r '.repoPath')

    # Truncate task if too long
    if [ ${#TASK} -gt 25 ]; then
        TASK="${TASK:0:22}..."
    fi

    # Truncate branch if too long
    if [ ${#BRANCH} -gt 18 ]; then
        BRANCH="${BRANCH:0:15}..."
    fi

    # Check if worktree directory exists
    if [ ! -d "$WORKTREE_PATH" ]; then
        STATUS="missing"
    fi

    # Check if original repo exists
    if [ ! -d "$REPO_PATH" ]; then
        STATUS="orphaned"
    fi

    # Check PR status if not set
    if [ "$PR" = "-" ] && command -v gh &> /dev/null; then
        PR_INFO=$(cd "$REPO_PATH" 2>/dev/null && gh pr list --head "$BRANCH" --json number,state --jq 'if length > 0 then .[0] | "\(.number):\(.state)" else "" end' 2>/dev/null || echo "")
        if [ -n "$PR_INFO" ]; then
            PR_NUM=$(echo "$PR_INFO" | cut -d':' -f1)
            PR_STATE=$(echo "$PR_INFO" | cut -d':' -f2)
            case "$PR_STATE" in
                "OPEN") PR="#$PR_NUM" ;;
                "MERGED") PR="#$PR_NUM ✓" ; STATUS="merged" ;;
                "CLOSED") PR="#$PR_NUM ✗" ;;
                *) PR="#$PR_NUM" ;;
            esac
        fi
    else
        PR="#$PR"
    fi

    # Check if any port is in use (indicates server running)
    FIRST_PORT=$(echo "$PORTS" | cut -d',' -f1)
    if lsof -i :"$FIRST_PORT" &>/dev/null; then
        PORTS="${PORTS}*"
    fi

    printf "%-25s %-20s %-12s %-8s %-10s %s\n" "$PROJECT" "$BRANCH" "$PORTS" "$PR" "$STATUS" "$TASK"
done

echo ""
echo "Legend: * = port in use, ✓ = PR merged, ✗ = PR closed"
echo ""

# Show port pool status
echo "───────────────────────────────────────────────────────────────────────────────"
echo "Port Pool:"
ALLOCATED=$(jq -r '.portPool.allocated | length' "$REGISTRY")
ALLOCATED_LIST=$(jq -r '.portPool.allocated | join(", ")' "$REGISTRY")
START=$(jq -r '.portPool.start // 8100' "$REGISTRY")
END=$(jq -r '.portPool.end // 8199' "$REGISTRY")
TOTAL=$((END - START + 1))
echo "  Range: $START-$END ($TOTAL total)"
echo "  Allocated: $ALLOCATED ports"
if [ "$ALLOCATED" -gt 0 ]; then
    echo "  In use: $ALLOCATED_LIST"
fi
echo ""

# Show worktree paths
echo "───────────────────────────────────────────────────────────────────────────────"
echo "Worktree Paths:"
if [ -n "$PROJECT_FILTER" ]; then
    jq -r ".worktrees[] | select(.project == \"$PROJECT_FILTER\") | \"  \\(.project)/\\(.branchSlug): \\(.worktreePath)\"" "$REGISTRY" 2>/dev/null
else
    jq -r '.worktrees[] | "  \(.project)/\(.branchSlug): \(.worktreePath)"' "$REGISTRY" 2>/dev/null
fi
