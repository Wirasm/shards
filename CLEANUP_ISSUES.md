# Shards Cleanup & Pruning Issues

This document outlines all the cleanup, pruning, and state management issues discovered during testing of the Shards CLI application.

## Overview

During comprehensive testing of the Shards CLI, several critical issues were identified related to incomplete cleanup operations, orphaned resources, and state inconsistencies. These issues prevent proper resource management and can lead to conflicts during shard creation.

## Issues Discovered

### 1. Missing Branch Cleanup in Destroy Operation

**Problem**: When destroying a shard with `shards destroy <branch>`, the application removes the worktree but leaves the associated Git branch behind.

**Evidence**: 
- 32 orphaned `worktree-*` branches accumulated during testing
- Branches like `worktree-test-shard`, `worktree-claude-demo`, etc. remained after shard destruction

**Impact**:
- "Branch already exists" errors when trying to recreate shards with the same name
- Git repository pollution with unused branches
- Confusion about which branches are actually active

**Current Behavior**:
```bash
# After destroying a shard, the branch remains:
$ git branch | grep worktree-
  worktree-test-shard
  worktree-claude-demo
  worktree-kiro-test
  # ... 29 more orphaned branches
```

**Expected Behavior**: 
The destroy operation should also delete the associated `worktree-<branch-name>` branch.

### 2. Orphaned Worktree State Management

**Problem**: Git worktrees can become orphaned or corrupted, leading to inconsistent states that prevent normal cleanup operations.

**Evidence**:
```bash
$ git worktree list
/Users/rasmus/.shards/worktrees/shards/final-test  0000000 (detached HEAD) prunable
/Users/rasmus/.shards/worktrees/shards/test-shard  0000000 (detached HEAD) prunable
```

**Symptoms**:
- Worktrees marked as "prunable" with detached HEAD (0000000)
- Git unable to remove worktrees normally: `fatal: validation failed, cannot remove working tree`
- Worktree directories exist but are not properly linked to Git

**Manual Recovery Required**:
```bash
# These operations were needed to clean up:
git worktree prune
rm -rf /Users/rasmus/.shards/worktrees/shards/final-test
git worktree prune  # Again after manual removal
```

### 3. Session File Persistence Without Cleanup

**Problem**: Session files remain in `~/.shards/sessions/` even after worktrees are destroyed, leading to stale session data.

**Evidence**:
- Session JSON files persisted after `shards destroy` operations
- `shards list` could show sessions for non-existent worktrees
- Manual cleanup required: `rm -rf ~/.shards/sessions/`

**Impact**:
- Stale session data confuses users about active shards
- Potential for session files to reference non-existent worktrees
- Storage accumulation over time

### 4. No Automatic Pruning or State Validation

**Problem**: The application doesn't automatically detect or fix inconsistent states between Git worktrees, branches, and session files.

**Missing Capabilities**:
- No automatic `git worktree prune` execution
- No validation that session files correspond to existing worktrees
- No detection of orphaned branches
- No recovery mechanism for corrupted worktree states

### 5. Incomplete Error Recovery During Creation

**Problem**: When shard creation fails partway through, partial resources may be left behind.

**Potential Scenarios**:
- Branch created but worktree creation fails
- Worktree created but terminal launch fails
- Session file created but worktree setup fails

**Current Behavior**: No rollback mechanism exists for partial failures.

## Root Cause Analysis

### Code Analysis

The primary issue is in `src/git/handler.rs` in the `remove_worktree_by_path` function:

```rust
pub fn remove_worktree_by_path(worktree_path: &Path) -> Result<(), GitError> {
    // ... finds and removes worktree ...
    worktree.prune(Some(&mut prune_options))?;
    
    // Missing: Branch deletion
    // Missing: Orphaned worktree detection
    // Missing: State validation
}
```

**What's Missing**:
1. Branch deletion after worktree removal
2. Fallback handling for corrupted worktree states
3. Automatic pruning of orphaned worktrees
4. Validation of worktree/branch/session consistency

### Architecture Gap

The current architecture treats worktrees, branches, and sessions as separate concerns, but they form a cohesive unit that should be managed atomically.

## Impact on User Experience

### During Testing
- **Creation Conflicts**: "Branch already exists" errors prevented shard creation
- **Manual Intervention**: Required manual Git commands to clean up state
- **Confusion**: `git branch` output cluttered with orphaned branches
- **Inconsistent State**: Sessions showing for non-existent worktrees

### Production Implications
- **Resource Accumulation**: Branches and directories accumulate over time
- **Disk Usage**: Orphaned worktree directories consume disk space
- **Performance**: Large numbers of orphaned branches slow Git operations
- **User Confusion**: Inconsistent state between `shards list` and actual resources

## Recommended Solutions

### 1. Enhanced Destroy Operation
```rust
// Pseudo-code for improved destroy
pub fn destroy_shard_completely(branch_name: &str) -> Result<(), Error> {
    // 1. Remove worktree
    remove_worktree_by_path(&worktree_path)?;
    
    // 2. Delete associated branch
    delete_branch(&format!("worktree-{}", branch_name))?;
    
    // 3. Remove session file
    remove_session_file(&session_id)?;
    
    // 4. Validate cleanup completed
    validate_cleanup_complete(branch_name)?;
}
```

### 2. Add Cleanup Command
```bash
shards cleanup  # New command to fix orphaned resources
```

**Functionality**:
- Prune orphaned worktrees (`git worktree prune`)
- Delete orphaned `worktree-*` branches
- Remove stale session files
- Validate and fix state inconsistencies
- Report cleanup actions taken

### 3. State Validation System
- Periodic validation of worktree/branch/session consistency
- Automatic detection of orphaned resources
- Warning messages for inconsistent states
- Recovery suggestions for users

### 4. Atomic Operations
- Implement rollback mechanisms for failed operations
- Ensure all-or-nothing semantics for shard creation/destruction
- Better error recovery during partial failures

### 5. Improved Error Handling
- Detect corrupted worktree states
- Fallback to manual directory removal when Git operations fail
- Graceful handling of edge cases

## Testing Evidence

### Before Cleanup
```bash
$ git branch | grep -E "(worktree-|test-)" | wc -l
32

$ git worktree list
/Users/rasmus/Projects/mine/SHARDS                 d456707 [ralph/file-based-persistence]
/Users/rasmus/.shards/worktrees/shards/final-test  0000000 (detached HEAD)
/Users/rasmus/.shards/worktrees/shards/test-shard  0000000 (detached HEAD) prunable
```

### Manual Cleanup Required
```bash
# Commands needed to clean up:
git worktree prune
rm -rf /Users/rasmus/.shards/worktrees/shards/final-test
git worktree prune
git branch | grep -E "(worktree-|test-)" | xargs -r git branch -D
rm -rf ~/.shards/sessions/
rm -rf ~/.shards/worktrees/
```

### After Cleanup
```bash
$ git branch | grep -E "(worktree-|test-)" || echo "No test branches found"
No test branches found

$ git worktree list
/Users/rasmus/Projects/mine/SHARDS  d456707 [ralph/file-based-persistence]

$ shards list
No active shards found.
```

## Priority Assessment

### Critical (Must Fix)
1. **Missing branch cleanup in destroy** - Causes creation conflicts
2. **Orphaned worktree handling** - Prevents normal Git operations

### High Priority (Should Fix)
3. **Session file cleanup** - Causes state confusion
4. **Add cleanup command** - Provides user recovery mechanism

### Medium Priority (Nice to Have)
5. **Automatic state validation** - Prevents issues proactively
6. **Atomic operations with rollback** - Improves reliability

## Next Steps

1. **Implement branch deletion** in the destroy operation
2. **Add worktree pruning** to cleanup orphaned worktrees
3. **Create cleanup command** for user-initiated state fixes
4. **Add state validation** to detect inconsistencies
5. **Improve error recovery** for partial operation failures

This cleanup track should address these issues systematically to ensure Shards provides reliable resource management and a clean user experience.
