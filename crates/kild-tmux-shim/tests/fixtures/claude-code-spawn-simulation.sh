#!/bin/bash
# Simulate the exact tmux command sequence Claude Code performs when spawning a 3-teammate team.
# Extracted from @anthropic-ai/claude-code v2.1.38 TmuxBackend class.
#
# Requires: TMUX, TMUX_PANE, KILD_SHIM_SESSION env vars set + shim on PATH.
# Exit code: 0 if all commands succeed, 1 on first failure.

set -euo pipefail

ERRORS=0

assert_ok() {
    if [ $? -ne 0 ]; then
        echo "FAIL: $1"
        ERRORS=$((ERRORS + 1))
    else
        echo "OK:   $1"
    fi
}

assert_output() {
    local expected="$1"
    local actual="$2"
    local label="$3"
    if [ -z "$actual" ]; then
        echo "FAIL: $label — empty output"
        ERRORS=$((ERRORS + 1))
    elif [ "$actual" != "$expected" ] && [ "$expected" != "*" ]; then
        echo "FAIL: $label — expected '$expected', got '$actual'"
        ERRORS=$((ERRORS + 1))
    else
        echo "OK:   $label — output: $actual"
    fi
}

assert_starts_with() {
    local prefix="$1"
    local actual="$2"
    local label="$3"
    if [[ "$actual" != ${prefix}* ]]; then
        echo "FAIL: $label — expected to start with '$prefix', got '$actual'"
        ERRORS=$((ERRORS + 1))
    else
        echo "OK:   $label — output: $actual"
    fi
}

echo "=== Claude Code Agent Team Spawn Simulation ==="
echo "TMUX=$TMUX"
echo "TMUX_PANE=$TMUX_PANE"
echo "KILD_SHIM_SESSION=$KILD_SHIM_SESSION"
echo ""

# Step 1: Check tmux availability
echo "--- Step 1: Availability check ---"
tmux -V
assert_ok "tmux -V exits 0"

# Step 2: Get leader's pane ID
echo ""
echo "--- Step 2: Get leader pane ID ---"
LEADER_PANE=$(tmux display-message -p "#{pane_id}")
assert_starts_with "%" "$LEADER_PANE" "display-message returns pane ID"

# Step 3: Get window target
echo ""
echo "--- Step 3: Get window target ---"
WINDOW=$(tmux display-message -p "#{session_name}:#{window_index}")
assert_output "*" "$WINDOW" "display-message returns window target"

# Step 4: List panes (should see leader)
echo ""
echo "--- Step 4: List panes ---"
PANE_COUNT=$(tmux list-panes -t "$WINDOW" -F "#{pane_id}" | wc -l | tr -d ' ')
echo "Pane count: $PANE_COUNT"

# Step 5: Create first teammate (horizontal split, 70%)
echo ""
echo "--- Step 5: Create teammate 1 (horizontal split) ---"
TEAMMATE1=$(tmux split-window -t "$LEADER_PANE" -h -l 70% -P -F "#{pane_id}")
assert_starts_with "%" "$TEAMMATE1" "split-window returns pane ID"

# Step 6: Style the pane
echo ""
echo "--- Step 6: Style teammate 1 ---"
tmux select-pane -t "$TEAMMATE1" -P "bg=default,fg=blue"
assert_ok "select-pane style"
tmux set-option -p -t "$TEAMMATE1" pane-border-style "fg=blue"
assert_ok "set-option pane-border-style"
tmux set-option -p -t "$TEAMMATE1" pane-active-border-style "fg=blue"
assert_ok "set-option pane-active-border-style"
tmux select-pane -t "$TEAMMATE1" -T "researcher"
assert_ok "select-pane title"
tmux set-option -p -t "$TEAMMATE1" pane-border-format "#[fg=blue,bold] #{pane_title} #[default]"
assert_ok "set-option pane-border-format"

# Step 7: Enable border status
echo ""
echo "--- Step 7: Enable border status ---"
tmux set-option -w -t "$WINDOW" pane-border-status top
assert_ok "set-option window pane-border-status"

# Step 8: Rebalance layout
echo ""
echo "--- Step 8: Rebalance layout ---"
tmux select-layout -t "$WINDOW" main-vertical
assert_ok "select-layout main-vertical"
tmux resize-pane -t "$LEADER_PANE" -x 30%
assert_ok "resize-pane leader width"

# Step 9: Send spawn command to teammate 1
echo ""
echo "--- Step 9: Send spawn command ---"
tmux send-keys -t "$TEAMMATE1" "cd /tmp && CLAUDECODE=1 echo 'researcher started'" Enter
assert_ok "send-keys to teammate 1"

sleep 0.2  # 200ms delay (matches Claude Code behavior)

# Step 10: Create teammate 2 (vertical split from teammate 1)
echo ""
echo "--- Step 10: Create teammate 2 (vertical split) ---"
TEAMMATE2=$(tmux split-window -t "$TEAMMATE1" -v -P -F "#{pane_id}")
assert_starts_with "%" "$TEAMMATE2" "split-window vertical returns pane ID"

# Style teammate 2
tmux select-pane -t "$TEAMMATE2" -P "bg=default,fg=green"
assert_ok "select-pane style teammate 2"
tmux select-pane -t "$TEAMMATE2" -T "implementer"
assert_ok "select-pane title teammate 2"
tmux select-layout -t "$WINDOW" main-vertical
assert_ok "select-layout after teammate 2"
tmux resize-pane -t "$LEADER_PANE" -x 30%
assert_ok "resize-pane after teammate 2"

tmux send-keys -t "$TEAMMATE2" "cd /tmp && CLAUDECODE=1 echo 'implementer started'" Enter
assert_ok "send-keys to teammate 2"

sleep 0.2

# Step 11: Create teammate 3 (horizontal split from teammate 1)
echo ""
echo "--- Step 11: Create teammate 3 (horizontal split) ---"
TEAMMATE3=$(tmux split-window -t "$TEAMMATE1" -h -P -F "#{pane_id}")
assert_starts_with "%" "$TEAMMATE3" "split-window horizontal returns pane ID"

# Style teammate 3
tmux select-pane -t "$TEAMMATE3" -P "bg=default,fg=yellow"
tmux select-pane -t "$TEAMMATE3" -T "tester"
tmux select-layout -t "$WINDOW" main-vertical
tmux resize-pane -t "$LEADER_PANE" -x 30%

tmux send-keys -t "$TEAMMATE3" "cd /tmp && CLAUDECODE=1 echo 'tester started'" Enter
assert_ok "send-keys to teammate 3"

# Step 12: Verify pane IDs are unique and sequential
echo ""
echo "--- Step 12: Verify pane IDs ---"
echo "Leader: $LEADER_PANE"
echo "Teammate 1: $TEAMMATE1"
echo "Teammate 2: $TEAMMATE2"
echo "Teammate 3: $TEAMMATE3"

if [ "$TEAMMATE1" = "$TEAMMATE2" ] || [ "$TEAMMATE1" = "$TEAMMATE3" ] || [ "$TEAMMATE2" = "$TEAMMATE3" ]; then
    echo "FAIL: Pane IDs are not unique!"
    ERRORS=$((ERRORS + 1))
else
    echo "OK:   All pane IDs are unique"
fi

# Step 13: List panes — should see all 4 (leader + 3 teammates)
echo ""
echo "--- Step 13: List all panes ---"
FINAL_PANES=$(tmux list-panes -t "$WINDOW" -F "#{pane_id}")
FINAL_COUNT=$(echo "$FINAL_PANES" | wc -l | tr -d ' ')
echo "Final pane list:"
echo "$FINAL_PANES"
echo "Count: $FINAL_COUNT"

# Step 14: Cleanup — kill all teammate panes
echo ""
echo "--- Step 14: Cleanup ---"
tmux kill-pane -t "$TEAMMATE1"
assert_ok "kill-pane teammate 1"
tmux kill-pane -t "$TEAMMATE2"
assert_ok "kill-pane teammate 2"
tmux kill-pane -t "$TEAMMATE3"
assert_ok "kill-pane teammate 3"

# Final summary
echo ""
echo "=== Simulation Complete ==="
if [ $ERRORS -gt 0 ]; then
    echo "FAILED: $ERRORS errors"
    exit 1
else
    echo "PASSED: All commands succeeded"
    exit 0
fi
