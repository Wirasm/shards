# Shards CLI Manual Testing Guide

## Prerequisites
- Built Shards CLI: `cargo build`
- Git repository (run tests from within a Git repo)
- Terminal emulators: Terminal.app and/or Ghostty
- AI agents: kiro-cli, claude, etc. (optional for basic testing)

## Test Suite

### 1. Basic Functionality Tests

#### Test 1.1: Create and List Shards
```bash
# Create a basic shard
./target/debug/shards start test-basic echo "Hello World"

# Verify it appears in list
./target/debug/shards list
# Expected: Shows test-basic as Active

# Check shard info
./target/debug/shards info test-basic
# Expected: Shows name, status, command, path, creation time, worktree exists ✅
```

#### Test 1.2: Stop and Cleanup
```bash
# Stop the shard
./target/debug/shards stop test-basic

# Verify it's gone
./target/debug/shards list
# Expected: "No active shards"

# Verify worktree is cleaned up
ls .shards/
# Expected: Directory should be empty or not exist
```

### 2. Terminal Integration Tests

#### Test 2.1: Terminal.app (Default)
```bash
# Test with Terminal.app (should auto-execute command)
./target/debug/shards start terminal-test --terminal terminal echo "Terminal.app test"

# Expected: Terminal.app window opens, runs command, shows output
# Verify: Check that terminal opened in correct directory (.shards/terminal-test)
```

#### Test 2.2: Ghostty (if available)
```bash
# Test with Ghostty (should use AppleScript automation)
./target/debug/shards start ghostty-test --terminal ghostty echo "Ghostty test"

# Expected: Ghostty window opens, command is typed automatically
# Verify: Check that Ghostty opened in correct directory (.shards/ghostty-test)
```

#### Test 2.3: Auto-Detection
```bash
# Test auto-detection (should pick Ghostty if available, else Terminal.app)
./target/debug/shards start auto-test echo "Auto-detection test"

# Expected: Opens in detected terminal
# Verify: Command executes automatically
```

### 3. Agent Profile Tests

#### Test 3.1: Default Agent Configuration
```bash
# Create config file
mkdir -p ~/.shards
cat > ~/.shards/config.toml << EOF
terminal = "terminal"
default_agent = "echo 'Default agent'"

[agents]
test = "echo 'Test agent'"
kiro = "kiro-cli chat"
claude = "claude"
EOF

# Test default agent (no command specified)
./target/debug/shards start default-agent-test

# Expected: Prompts to choose agent or uses default_agent
```

#### Test 3.2: Agent Profile Selection
```bash
# Test specific agent profile
./target/debug/shards start profile-test --agent test

# Expected: Uses "echo 'Test agent'" command
```

#### Test 3.3: Command Override
```bash
# Test that explicit command overrides profiles
./target/debug/shards start override-test --agent test echo "Override command"

# Expected: Uses "echo 'Override command'" instead of profile command
```

### 4. Git Integration Tests

#### Test 4.1: Worktree Creation
```bash
# Create shard and verify Git setup
./target/debug/shards start git-test echo "Git test"

# Verify worktree exists
ls .shards/git-test/
# Expected: Should contain project files

# Verify unique branch created
cd .shards/git-test && git branch --show-current
# Expected: Shows branch like "shard_a1b2c3d4..."

# Return to main directory
cd ../..
```

#### Test 4.2: Multiple Shards (Isolation)
```bash
# Create multiple shards
./target/debug/shards start shard-1 echo "Shard 1"
./target/debug/shards start shard-2 echo "Shard 2"
./target/debug/shards start shard-3 echo "Shard 3"

# Verify all are listed
./target/debug/shards list
# Expected: Shows all 3 shards as Active

# Verify each has unique branch
cd .shards/shard-1 && git branch --show-current && cd ../..
cd .shards/shard-2 && git branch --show-current && cd ../..
cd .shards/shard-3 && git branch --show-current && cd ../..
# Expected: Each shows different shard_* branch
```

### 5. Error Handling Tests

#### Test 5.1: Duplicate Shard Names
```bash
# Try to create shard with existing name
./target/debug/shards start shard-1 echo "Duplicate test"
# Expected: Error "Shard 'shard-1' already exists"
```

#### Test 5.2: Non-existent Shard Operations
```bash
# Try to stop non-existent shard
./target/debug/shards stop non-existent
# Expected: Error "Shard 'non-existent' not found"

# Try to get info for non-existent shard
./target/debug/shards info non-existent
# Expected: Error "Shard 'non-existent' not found"
```

#### Test 5.3: Invalid Agent Profile
```bash
# Try to use non-existent agent profile
./target/debug/shards start invalid-agent-test --agent nonexistent
# Expected: Error "Agent profile 'nonexistent' not found"
```

### 6. Cleanup and Recovery Tests

#### Test 6.1: Cleanup Command
```bash
# Stop all test shards
./target/debug/shards stop git-test
./target/debug/shards stop shard-1
./target/debug/shards stop shard-2
./target/debug/shards stop shard-3
./target/debug/shards stop terminal-test
./target/debug/shards stop ghostty-test
./target/debug/shards stop auto-test
./target/debug/shards stop default-agent-test
./target/debug/shards stop profile-test
./target/debug/shards stop override-test

# Verify cleanup
./target/debug/shards list
# Expected: "No active shards"

# Test cleanup command
./target/debug/shards cleanup
# Expected: "No orphaned shards found" or cleans up any orphaned entries
```

#### Test 6.2: Manual Worktree Removal (Orphan Test)
```bash
# Create a shard
./target/debug/shards start orphan-test echo "Orphan test"

# Manually remove worktree directory (simulate crash/corruption)
rm -rf .shards/orphan-test

# Run cleanup
./target/debug/shards cleanup
# Expected: "Cleaning up orphaned session: orphan-test"

# Verify it's removed from registry
./target/debug/shards list
# Expected: "No active shards"
```

### 7. Real Agent Tests (Optional)

#### Test 7.1: Kiro CLI (if available)
```bash
./target/debug/shards start kiro-test kiro-cli chat
# Expected: Kiro CLI starts in terminal, ready for interaction
# Manual: Type a simple question to verify it works
```

#### Test 7.2: Claude Code (if available)
```bash
./target/debug/shards start claude-test claude
# or with alias:
./target/debug/shards start cc-test cc
# Expected: Claude Code starts in terminal
```

## Expected Results Summary

✅ **All commands should execute without errors**
✅ **Terminal windows should open in correct directories**
✅ **Git worktrees should be created with unique branches**
✅ **Session registry should track all operations correctly**
✅ **Cleanup should remove all traces of test shards**
✅ **Error messages should be clear and helpful**

## Troubleshooting

- **Terminal doesn't open**: Check terminal availability and permissions
- **Commands don't execute**: Verify terminal integration and AppleScript permissions
- **Git errors**: Ensure you're running from within a Git repository
- **Permission errors**: Check file system permissions for ~/.shards/ directory

## Clean Up After Testing

```bash
# Remove test config
rm ~/.shards/config.toml

# Final cleanup
./target/debug/shards cleanup
```
