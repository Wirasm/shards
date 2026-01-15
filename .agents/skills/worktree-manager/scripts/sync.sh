#!/bin/bash
# sync.sh - Reconcile registry with actual worktrees and PR status
#
# Usage: ./sync.sh [--quiet] [--fix]
#
# Options:
#   --quiet  Only show issues, not OK entries
#   --fix    Automatically fix issues (remove missing, update PR numbers)
#
# This script:
# 1. Compares registry entries with actual git worktrees
# 2. Checks PR status for each worktree branch
# 3. Reports discrepancies
# 4. Optionally fixes issues (with --fix)

set -e

REGISTRY="${HOME}/.claude/worktree-registry.json"
QUIET=false
FIX=false

# Parse arguments
for arg in "$@"; do
    case $arg in
        --quiet) QUIET=true ;;
        --fix) FIX=true ;;
    esac
done

# Check dependencies
if ! command -v jq &> /dev/null; then
    echo "Error: jq is required. Install with: brew install jq"
    exit 1
fi

if ! command -v gh &> /dev/null; then
    echo "Warning: gh (GitHub CLI) not found. PR status checks will be skipped."
    HAS_GH=false
else
    HAS_GH=true
fi

# Check if registry exists
if [ ! -f "$REGISTRY" ]; then
    echo "Error: No registry found at: $REGISTRY"
    exit 1
fi

echo "Syncing worktree registry..."
echo "─────────────────────────────────────────────────────────"

# Counters
TOTAL=0
MISSING=0
MERGED=0
ORPHANED=0
UPDATED=0

# Process each registry entry
while IFS= read -r entry; do
    [ -z "$entry" ] && continue

    TOTAL=$((TOTAL + 1))
    PROJECT=$(echo "$entry" | jq -r '.project')
    BRANCH=$(echo "$entry" | jq -r '.branch')
    WPATH=$(echo "$entry" | jq -r '.worktreePath')
    CURRENT_PR=$(echo "$entry" | jq -r '.prNumber // "null"')
    CURRENT_STATUS=$(echo "$entry" | jq -r '.status')

    # Check if directory exists
    if [ ! -d "$WPATH" ]; then
        echo "❌ MISSING: $PROJECT / $BRANCH"
        echo "   Path: $WPATH"
        MISSING=$((MISSING + 1))

        if [ "$FIX" = true ]; then
            echo "   → Removing from registry..."
            TMP=$(mktemp)
            jq "del(.worktrees[] | select(.project == \"$PROJECT\" and .branch == \"$BRANCH\"))" "$REGISTRY" > "$TMP"
            mv "$TMP" "$REGISTRY"
            UPDATED=$((UPDATED + 1))
        fi
        continue
    fi

    # Check if it's a valid git worktree
    if [ ! -f "$WPATH/.git" ] && [ ! -d "$WPATH/.git" ]; then
        echo "⚠️  ORPHANED: $PROJECT / $BRANCH"
        echo "   Path exists but not a git worktree"
        ORPHANED=$((ORPHANED + 1))
        continue
    fi

    # Check PR status
    PR_NUM="null"
    PR_STATE="NONE"

    if [ "$HAS_GH" = true ]; then
        PR_INFO=$(gh pr list --head "$BRANCH" --state all --json number,state --limit 1 2>/dev/null || echo "[]")
        if [ "$PR_INFO" != "[]" ] && [ -n "$PR_INFO" ]; then
            PR_NUM=$(echo "$PR_INFO" | jq -r '.[0].number // "null"')
            PR_STATE=$(echo "$PR_INFO" | jq -r '.[0].state // "NONE"')
        fi
    fi

    # Determine new status
    NEW_STATUS="$CURRENT_STATUS"
    if [ "$PR_STATE" = "MERGED" ]; then
        NEW_STATUS="merged"
        echo "✅ MERGED: $PROJECT / $BRANCH (PR #$PR_NUM)"
        MERGED=$((MERGED + 1))
    elif [ "$PR_STATE" = "OPEN" ]; then
        NEW_STATUS="active"
        if [ "$QUIET" = false ]; then
            echo "🟢 ACTIVE: $PROJECT / $BRANCH (PR #$PR_NUM open)"
        fi
    elif [ "$PR_STATE" = "CLOSED" ]; then
        NEW_STATUS="closed"
        echo "🟡 CLOSED: $PROJECT / $BRANCH (PR #$PR_NUM closed without merge)"
    else
        if [ "$QUIET" = false ]; then
            echo "🔵 NO PR: $PROJECT / $BRANCH"
        fi
    fi

    # Update registry if needed
    if [ "$FIX" = true ]; then
        NEEDS_UPDATE=false

        # Update prNumber if changed
        if [ "$PR_NUM" != "null" ] && [ "$PR_NUM" != "$CURRENT_PR" ]; then
            NEEDS_UPDATE=true
        fi

        # Update status if changed
        if [ "$NEW_STATUS" != "$CURRENT_STATUS" ]; then
            NEEDS_UPDATE=true
        fi

        if [ "$NEEDS_UPDATE" = true ]; then
            TMP=$(mktemp)
            if [ "$PR_NUM" != "null" ]; then
                jq "(.worktrees[] | select(.project == \"$PROJECT\" and .branch == \"$BRANCH\")) |= . + {prNumber: $PR_NUM, status: \"$NEW_STATUS\"}" "$REGISTRY" > "$TMP"
            else
                jq "(.worktrees[] | select(.project == \"$PROJECT\" and .branch == \"$BRANCH\")).status = \"$NEW_STATUS\"" "$REGISTRY" > "$TMP"
            fi
            mv "$TMP" "$REGISTRY"
            UPDATED=$((UPDATED + 1))
            echo "   → Updated registry (PR: $PR_NUM, status: $NEW_STATUS)"
        fi
    fi

done < <(jq -c '.worktrees[]' "$REGISTRY" 2>/dev/null)

echo "─────────────────────────────────────────────────────────"
echo "Summary:"
echo "  Total entries: $TOTAL"
echo "  Missing: $MISSING"
echo "  Merged PRs: $MERGED"
echo "  Orphaned: $ORPHANED"
if [ "$FIX" = true ]; then
    echo "  Updated: $UPDATED"
fi

if [ $MERGED -gt 0 ]; then
    echo ""
    echo "💡 Tip: Run 'cleanup.sh --merged' to remove merged worktrees"
fi
