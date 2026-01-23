# Shards CLI E2E Testing Guide

Run this to verify the CLI works correctly. Can be run from any location: main branch, feature branch, or inside a worktree.

## Pre-flight: Locate Yourself

Before running tests, determine your current context:

```bash
# Where am I?
pwd

# What branch am I on?
git branch --show-current

# Am I in a worktree?
git rev-parse --show-toplevel
```

**Context determines the test target:**

| Location | What you're testing |
|----------|---------------------|
| Main branch in main repo | The merged/released code |
| Feature branch in main repo | Your changes before merging |
| Inside a worktree (`~/.shards/worktrees/...`) | Changes in that isolated workspace |

## Build from Current Location

Build the release binary from wherever you are:

```bash
cargo build --release --bin shards
```

This builds from your current branch/worktree code. The binary will be at `./target/release/shards`.

**Verify the build is from your code:**
```bash
git log -1 --oneline  # Note the commit
./target/release/shards --version  # Should match
```

## Test Sequence

Execute these tests in order. Use `./target/release/shards` for all commands.

### Phase 1: Clean State Check
```bash
./target/release/shards list
```
**Expected**: Shows existing shards or "No active shards found." Note any existing shards - they should not be affected.

### Phase 2: Create Test Shard
```bash
./target/release/shards create e2e-test-shard --agent claude
```
**Expected**:
- Success message
- Branch: `e2e-test-shard`
- Worktree path shown
- Port range allocated
- Terminal window opens with Claude

**If it fails**:
- Branch exists? `git branch -a | grep e2e-test`
- In a git repo? `git status`
- Disk space? `df -h`

### Phase 3: List Shows the Shard
```bash
./target/release/shards list
```
**Expected**: Table shows `e2e-test-shard` with status active, process running.

### Phase 4: Status Details
```bash
./target/release/shards status e2e-test-shard
```
**Expected**: Detailed info box with process running, PID shown.

### Phase 5: Health (All)
```bash
./target/release/shards health
```
**Expected**: Dashboard table with Working status, CPU/memory metrics.

### Phase 6: Health (Single)
```bash
./target/release/shards health e2e-test-shard
```
**Expected**: Detailed health for just this shard.

### Phase 7: Cleanup --orphans
```bash
./target/release/shards cleanup --orphans
```
**Expected**: "No orphaned resources found" (shard has valid session).

### Phase 8: Restart
```bash
./target/release/shards restart e2e-test-shard
```
**Expected**: Success, agent restarted.

### Phase 9: Destroy
```bash
./target/release/shards destroy e2e-test-shard
```
**Expected**: Success, terminal closes, worktree removed.

### Phase 10: Verify Clean
```bash
./target/release/shards list
```
**Expected**: `e2e-test-shard` gone, only pre-existing shards remain.

## Edge Cases

Test error handling after the main sequence:

### Destroy Non-existent
```bash
./target/release/shards destroy fake-shard-xyz
```
**Expected**: Error "not found"

### Status Non-existent
```bash
./target/release/shards status fake-shard-xyz
```
**Expected**: Error "not found"

### Cleanup Empty
```bash
./target/release/shards cleanup --stopped
```
**Expected**: "No orphaned resources found"

### Health JSON
```bash
./target/release/shards health --json
```
**Expected**: Valid JSON output

## Test Report

Summarize results:

| Test | Status | Notes |
|------|--------|-------|
| Location | | (branch/worktree name) |
| Build | | |
| Create | | |
| List | | |
| Status | | |
| Health (all) | | |
| Health (single) | | |
| Cleanup --orphans | | |
| Restart | | |
| Destroy | | |
| Clean state | | |
| Edge cases | | |

**All tests must pass.**

## Special Considerations

### Testing from a Worktree

If you're inside a shards worktree (e.g., `~/.shards/worktrees/shards/feature-x/`):
- You're testing the code from that worktree's branch
- The test shard will be created as a nested worktree (this is fine)
- Make sure to destroy test shards before destroying the parent worktree

### Testing from a Feature Branch

If you're on a feature branch in the main repo:
- You're testing your uncommitted/committed changes
- Good for verifying changes before creating a PR
- The binary reflects your branch's code, not main

### Comparing Against Main

To compare behavior between your changes and main:
```bash
# Build your branch
cargo build --release --bin shards
cp ./target/release/shards /tmp/shards-feature

# Switch to main and build
git checkout main
cargo build --release --bin shards
cp ./target/release/shards /tmp/shards-main

# Now you can compare
/tmp/shards-main list
/tmp/shards-feature list
```

## Troubleshooting

**Terminal doesn't open**: Try `--terminal iterm` or `--terminal terminal`

**Process not tracked**: Check `~/.shards/pids/`

**Worktree exists**: Run `git worktree list` and `git worktree prune`

**Port conflict**: Run `shards list` to check existing shards

**JSON log noise**: Normal - look for human-readable success messages and tables
