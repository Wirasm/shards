# Shards CLI E2E Testing Guide

Run this after every merge to main to verify the CLI works correctly.

## Pre-flight

1. Ensure you're in the SHARDS repository root
2. Ensure you're on the main branch with latest changes
3. Build the release binary:
   ```bash
   cargo build --release --bin shards
   ```

## Test Sequence

Execute these tests in order. After each command, verify the expected output. If something fails, investigate and fix before continuing.

### Phase 1: Clean State Verification
```bash
./target/release/shards list
```
**Expected**: Either "No active shards found." or a table of existing shards. Note any existing shards - they should not be affected by our tests.

### Phase 2: Create a Test Shard
```bash
./target/release/shards create e2e-test-shard --agent claude
```
**Expected**:
- Success message
- Branch: `e2e-test-shard`
- Worktree path shown
- Port range allocated (e.g., 3000-3009)
- A new Ghostty terminal window opens with Claude

**If it fails**: Check if branch already exists (`git branch -a | grep e2e-test`), check disk space, check if in a git repo.

### Phase 3: Verify Shard Appears in List
```bash
./target/release/shards list
```
**Expected**: Table shows `e2e-test-shard` with:
- Agent: claude
- Status: active
- Process: Running with a PID

### Phase 4: Check Detailed Status
```bash
./target/release/shards status e2e-test-shard
```
**Expected**: Detailed box showing:
- Branch, Agent, Status
- Worktree path exists
- Process is Running with PID
- Process Name: claude

### Phase 5: Health Check (All Shards)
```bash
./target/release/shards health
```
**Expected**: Health dashboard table with:
- Status icon (green for Working)
- CPU and Memory metrics
- Summary line showing totals

### Phase 6: Health Check (Single Shard)
```bash
./target/release/shards health e2e-test-shard
```
**Expected**: Detailed health for just this shard

### Phase 7: Test Cleanup --orphans Flag
```bash
./target/release/shards cleanup --orphans
```
**Expected**: "No orphaned resources found" (since our shard has a valid session)

### Phase 8: Restart the Shard
```bash
./target/release/shards restart e2e-test-shard
```
**Expected**:
- Success message
- Agent process restarted
- Terminal window may flash/reload

### Phase 9: Restart with Different Agent (Optional)
```bash
./target/release/shards restart e2e-test-shard --agent kiro
```
**Expected**: Shard now running with kiro agent instead of claude
**Note**: Skip if kiro is not installed

### Phase 10: Destroy the Test Shard
```bash
./target/release/shards destroy e2e-test-shard
```
**Expected**:
- Success message
- Terminal window closes
- Worktree removed

### Phase 11: Verify Clean State
```bash
./target/release/shards list
```
**Expected**: `e2e-test-shard` no longer appears. Only shards that existed before the test remain.

## Edge Case Tests

Run these after the main sequence to test error handling:

### Edge Case 1: Create Duplicate Shard
```bash
./target/release/shards create edge-test --agent claude
./target/release/shards create edge-test --agent claude
```
**Expected**: Second create should fail with "already exists" error
**Cleanup**: `./target/release/shards destroy edge-test`

### Edge Case 2: Destroy Non-existent Shard
```bash
./target/release/shards destroy this-does-not-exist
```
**Expected**: Error message indicating shard not found

### Edge Case 3: Status of Non-existent Shard
```bash
./target/release/shards status this-does-not-exist
```
**Expected**: Error message indicating shard not found

### Edge Case 4: Invalid Agent
```bash
./target/release/shards create invalid-agent-test --agent not-a-real-agent
```
**Expected**: Error about invalid agent type

### Edge Case 5: Cleanup When Nothing to Clean
```bash
./target/release/shards cleanup --stopped
```
**Expected**: "No orphaned resources found" message

### Edge Case 6: Health with JSON Output
```bash
./target/release/shards health --json
```
**Expected**: Valid JSON output that can be parsed

## Test Report

After running all tests, summarize:

| Test | Status | Notes |
|------|--------|-------|
| Build | | |
| List (empty) | | |
| Create | | |
| List (with shard) | | |
| Status | | |
| Health (all) | | |
| Health (single) | | |
| Cleanup --orphans | | |
| Restart | | |
| Destroy | | |
| List (clean) | | |
| Edge cases | | |

**All tests must pass before considering a merge successful.**

## Troubleshooting

**Terminal doesn't open**: Check if Ghostty is installed, try `--terminal iterm` or `--terminal terminal`

**Process not tracked**: PID file may not have been written. Check `~/.shards/pids/`

**Worktree already exists**: Run `git worktree list` and `git worktree prune`

**Port conflict**: Another shard may be using the ports. Run `shards list` to check

**Structured logging noise**: The JSON log lines are expected. Look for the human-readable output (success messages, tables)
