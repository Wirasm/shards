# Investigation: Parallel kild create hits git2 race condition on .git/worktrees directory

**Issue**: #497 (https://github.com/Wirasm/kild/issues/497)
**Type**: BUG
**Investigated**: 2026-02-18T00:00:00Z

### Assessment

| Metric     | Value  | Reasoning                                                                                            |
| ---------- | ------ | ---------------------------------------------------------------------------------------------------- |
| Severity   | MEDIUM | Users must retry manually when two kilds are created simultaneously, but no data loss or corruption occurs and the workaround is a simple retry |
| Complexity | LOW    | Fix is isolated to a single function in `git/handler.rs` (~15 lines of new code), no architectural changes needed |
| Confidence | HIGH   | Root cause is precisely identified at `git/handler.rs:230` with confirmed error code from the issue report and well-understood libgit2 behavior |

---

## Problem Statement

When two `kild create` commands run simultaneously, libgit2's `git_worktree_add()` tries to create the `.git/worktrees/` parent directory with a non-atomic `mkdir`. One process succeeds; the other fails immediately with `git2::ErrorCode::Exists` (-4). The failing process returns an unrecoverable error and the user must retry manually.

---

## Analysis

### Root Cause

WHY: Second `kild create` fails with `"Git operation failed: Git2 library error: failed to make directory '/path/to/.git/worktrees': directory exists"`
↓ BECAUSE: `repo.worktree()` call at `git/handler.rs:230` propagates a `git2::Error` with `code=Exists(-4)` straight to `GitError::Git2Error` without inspection
↓ BECAUSE: libgit2's `git_worktree_add()` calls `p_mkdir` (not `p_mkdir_all`) on `.git/worktrees/` — it fails if the directory already exists, even though existence is fine
↓ ROOT CAUSE: The `.git/worktrees/` parent directory creation inside libgit2 is non-atomic. The first process to call `repo.worktree()` creates it; any concurrent call fails. A single retry on `git2::ErrorCode::Exists` would succeed because the directory now exists and libgit2 proceeds past the `mkdir`.
Evidence: `crates/kild-core/src/git/handler.rs:230` — `repo.worktree(&worktree_name, &worktree_path, Some(&opts)).map_err(git2_error)?;`

### Evidence Chain

WHY: User sees "Git operation failed: Git2 library error: failed to make directory ... directory exists"
↓ BECAUSE: `git2_error()` at `handler.rs:15-17` blindly wraps ALL `git2::Error` into `GitError::Git2Error` without code inspection:
```rust
fn git2_error(e: git2::Error) -> GitError {
    GitError::Git2Error { source: e }
}
```

↓ BECAUSE: `repo.worktree()` at `handler.rs:230` uses `.map_err(git2_error)?` — no check for `e.code() == git2::ErrorCode::Exists` before propagating
```rust
repo.worktree(&worktree_name, &worktree_path, Some(&opts))
    .map_err(git2_error)?;
```

↓ ROOT CAUSE: libgit2 uses `p_mkdir` (non-atomic) for `.git/worktrees/`. On first invocation it creates it; on concurrent invocation it fails with `GIT_EEXISTS`. The error is transient — a retry always succeeds because the directory now exists.

Evidence that retry works: The codebase already uses `e.code() == git2::ErrorCode::NotFound` for conditional handling at `handler.rs:322`, confirming git2 error codes are inspectable. A retry on `Exists` would find `.git/worktrees/` already present and proceed normally.

### Affected Files

| File                                            | Lines   | Action | Description                                      |
| ----------------------------------------------- | ------- | ------ | ------------------------------------------------ |
| `crates/kild-core/src/git/handler.rs`           | 219-231 | UPDATE | Add retry helper for `git2::ErrorCode::Exists`   |
| `crates/kild-core/src/git/handler.rs`           | ~560+   | UPDATE | Add concurrent worktree creation test            |

### Integration Points

- `crates/kild-core/src/sessions/create.rs:179` calls `git::handler::create_worktree()` — no changes needed there
- `crates/kild-core/src/git/errors.rs:35-39` — `GitError::Git2Error` unchanged, still used for non-Exists errors
- CLI at `crates/kild/src/commands/create.rs:92` — unchanged; error handling path never reached for the race

### Git History

- **Last modified**: `212b60a` — "refactor: introduce SessionId, BranchName, ProjectId newtypes" (post-race-condition introduction)
- **Introduced**: Pre-existing in the initial git2 integration; the race window was always present but low-probability
- **Implication**: Long-standing latent bug, surfaces only under explicit parallel invocation

---

## Implementation Plan

### Step 1: Add `add_git_worktree_with_retry` helper and use it

**File**: `crates/kild-core/src/git/handler.rs`
**Lines**: 219-231
**Action**: UPDATE

**Current code** (`handler.rs:219-231`):
```rust
    // Worktree admin name: kild-<sanitized_branch> (filesystem-safe, flat)
    // Decoupled from branch name via WorktreeAddOptions::reference()
    let worktree_name = naming::kild_worktree_admin_name(&validated_branch);
    let branch_ref = repo
        .find_branch(&kild_branch, BranchType::Local)
        .map_err(git2_error)?;
    let reference = branch_ref.into_reference();

    let mut opts = WorktreeAddOptions::new();
    opts.reference(Some(&reference));

    repo.worktree(&worktree_name, &worktree_path, Some(&opts))
        .map_err(git2_error)?;
```

**Required change** — replace the `repo.worktree()` call with a retry helper:

```rust
    // Worktree admin name: kild-<sanitized_branch> (filesystem-safe, flat)
    // Decoupled from branch name via WorktreeAddOptions::reference()
    let worktree_name = naming::kild_worktree_admin_name(&validated_branch);
    let branch_ref = repo
        .find_branch(&kild_branch, BranchType::Local)
        .map_err(git2_error)?;
    let reference = branch_ref.into_reference();

    let mut opts = WorktreeAddOptions::new();
    opts.reference(Some(&reference));

    add_git_worktree_with_retry(&repo, &worktree_name, &worktree_path, &opts)?;
```

And add the helper function near the existing `git2_error` / `io_error` helpers at the top of the file (after line 17):

```rust
/// Calls `repo.worktree()` with retry on `git2::ErrorCode::Exists`.
///
/// libgit2's `git_worktree_add()` creates `.git/worktrees/` with a non-atomic
/// mkdir. When two `kild create` processes run concurrently, the second fails
/// with `Exists(-4)` because the first just created the directory. A retry
/// always succeeds since the directory now exists and libgit2 proceeds normally.
fn add_git_worktree_with_retry(
    repo: &Repository,
    name: &str,
    path: &std::path::Path,
    opts: &WorktreeAddOptions<'_>,
) -> Result<(), GitError> {
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(50);

    let mut attempt = 0;
    loop {
        match repo.worktree(name, path, Some(opts)) {
            Ok(_) => return Ok(()),
            Err(e) if e.code() == git2::ErrorCode::Exists && attempt < MAX_RETRIES => {
                attempt += 1;
                warn!(
                    event = "core.git.worktree.create_retry",
                    attempt = attempt,
                    error = %e,
                    "Retrying worktree creation after concurrent mkdir race"
                );
                std::thread::sleep(RETRY_DELAY);
            }
            Err(e) => return Err(git2_error(e)),
        }
    }
}
```

**Why**: The `Exists` error on `repo.worktree()` when creating two different worktrees simultaneously means `.git/worktrees/` was just created by a concurrent process. The directory existing is perfectly fine for the next attempt — libgit2 only fails because it tried to `mkdir` something that already exists. A retry of up to 3 times with 50ms gaps handles any realistic level of contention.

---

### Step 2: Add concurrent worktree creation regression test

**File**: `crates/kild-core/src/git/handler.rs` (in the `#[cfg(test)]` block)
**Action**: UPDATE — add test after existing worktree tests (around line 560+)

```rust
#[test]
fn test_concurrent_worktree_creation_different_branches() {
    use std::sync::{Arc, Barrier};
    use std::thread;

    let temp_dir = create_temp_test_dir("kild_test_concurrent");
    init_test_repo(&temp_dir);
    let base_dir = create_temp_test_dir("kild_test_concurrent_base");

    let temp_dir = Arc::new(temp_dir);
    let base_dir = Arc::new(base_dir);
    let barrier = Arc::new(Barrier::new(2));

    let handles: Vec<_> = ["branch-a", "branch-b"]
        .iter()
        .map(|branch| {
            let temp_dir = Arc::clone(&temp_dir);
            let base_dir = Arc::clone(&base_dir);
            let barrier = Arc::clone(&barrier);
            let branch = branch.to_string();

            thread::spawn(move || {
                let project = ProjectInfo::new(
                    "test-id".to_string(),
                    "test-project".to_string(),
                    (*temp_dir).clone(),
                    None,
                );
                let git_config = GitConfig {
                    fetch_before_create: Some(false),
                    ..GitConfig::default()
                };

                // Both threads synchronize here to maximize the race window
                barrier.wait();

                create_worktree(&base_dir, &project, &branch, None, &git_config)
            })
        })
        .collect();

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    assert!(
        results.iter().all(|r| r.is_ok()),
        "Both concurrent worktree creations should succeed, got: {:?}",
        results.iter().filter(|r| r.is_err()).collect::<Vec<_>>()
    );

    let _ = std::fs::remove_dir_all(&*Arc::try_unwrap(temp_dir.clone()).unwrap_or_else(|t| (*t).clone()));
    let _ = std::fs::remove_dir_all(&*Arc::try_unwrap(base_dir.clone()).unwrap_or_else(|t| (*t).clone()));
}
```

**Why**: Directly reproduces the race condition reported in the issue using a `Barrier` to synchronize two threads at the exact race window. The test proves both concurrent creates succeed without requiring manual retry.

---

## Patterns to Follow

**Existing conditional `e.code()` check** — mirror this pattern exactly:

```rust
// SOURCE: crates/kild-core/src/git/handler.rs:322
Err(e) if e.code() == git2::ErrorCode::NotFound => {
    // Remote ref not found - fall back to HEAD
```

**Existing warn! logging** — mirror this pattern:

```rust
// SOURCE: crates/kild-core/src/git/handler.rs (multiple sites)
warn!(
    event = "core.git.worktree.file_copy_failed",
    error = %e,
    message = "File copying failed, but worktree creation succeeded"
);
```

**Concurrent test pattern** — mirror from kild-tmux-shim:

```rust
// SOURCE: crates/kild-tmux-shim/src/state.rs:607-677
// Uses std::thread, Arc, Barrier for concurrent lock tests
let barrier = Arc::new(Barrier::new(2));
// All threads call barrier.wait() before the racy operation
```

---

## Edge Cases & Risks

| Risk/Edge Case                                 | Mitigation                                                                  |
| ---------------------------------------------- | --------------------------------------------------------------------------- |
| Same branch created concurrently               | Already handled at `handler.rs:141-152` (`worktree_path.exists()` check); the retry won't run because `WorktreeAlreadyExists` is returned before reaching `repo.worktree()` |
| Retry on a genuine `Exists` error (not race)   | Not possible: both worktree path and admin name are branch-derived and unique; if path existed, we'd fail at line 141 first |
| 3 retries insufficient                         | Extremely unlikely — the race window is microseconds; 3×50ms gives 150ms of retry budget |
| Test flakiness                                 | The Barrier ensures both threads start simultaneously; test is deterministic enough for CI |
| `std::thread::sleep` in production path        | Only invoked on concurrent contention (exceedingly rare in practice); 50ms is imperceptible to users |

---

## Validation

```bash
# Build
cargo build -p kild-core

# Run all git handler tests (includes new concurrent test)
cargo test -p kild-core test_concurrent_worktree_creation
cargo test -p kild-core -- git::

# Run full test suite
cargo test --all

# Lint
cargo clippy --all -- -D warnings

# Format check
cargo fmt --check
```

### Manual Verification

```bash
# Build the binary
cargo build -p kild

# Reproduce the original race
kild daemon stop
kild create test-race-a --no-agent & kild create test-race-b --no-agent & wait

# Both should succeed now; verify
kild list
```

---

## Scope Boundaries

**IN SCOPE:**
- Adding `add_git_worktree_with_retry()` helper in `git/handler.rs`
- Replacing the single `repo.worktree().map_err(git2_error)?` call with the helper
- Adding a concurrent regression test

**OUT OF SCOPE (do not touch):**
- File locking at the session create level (heavier than needed for this specific race)
- Port allocation race at `sessions/ports.rs` (separate issue, no report)
- Any other `git2_error` call sites (they don't create worktrees)
- Changes to `SessionError`, `GitError`, or error display strings

---

## Metadata

- **Investigated by**: Claude
- **Timestamp**: 2026-02-18T00:00:00Z
- **Artifact**: `.claude/PRPs/issues/issue-497.md`
