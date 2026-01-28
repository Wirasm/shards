# Feature: Phase 9.7 - Git Diff Stats

## Summary

Add git diff statistics (`+insertions -deletions`) to the kild list view. This enhances the existing git dirty indicator (orange dot) with actual line counts, giving users immediate visibility into the scope of uncommitted changes in each worktree. Implementation uses git2's native `DiffStats` API rather than shelling out to git.

## User Story

As a **Tōryō (power user)**
I want to **see how many lines were added/removed in each kild**
So that I can **quickly gauge the scope of work and prioritize which kilds to review**

## Problem Statement

Currently, the UI shows only a binary dirty/clean indicator (orange dot or nothing). Users can't tell if a kild has 2 lines changed or 2000 lines changed without running `kild diff` or entering the worktree. This makes it hard to prioritize which kilds need attention.

## Solution Statement

Add a `DiffStats` type to kild-core that fetches `insertions`, `deletions`, and `files_changed` via git2. Display these stats in the list row next to the existing git dirty indicator, replacing the simple dot with `+N -N` when dirty.

## Metadata

| Field            | Value                                |
| ---------------- | ------------------------------------ |
| Type             | ENHANCEMENT                          |
| Complexity       | LOW                                  |
| Systems Affected | kild-core (git), kild-ui (state, views) |
| Dependencies     | git2 0.18 (already present)          |
| Estimated Tasks  | 6                                    |

---

## UX Design

### Before State

```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                              BEFORE STATE                                      ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║   KILD List Row:                                                              ║
║   ┌───────────────────────────────────────────────────────────────────────┐   ║
║   │ ● ●  feature-auth   claude   kild   23m   JWT auth...   [Copy][Edit]  │   ║
║   │ ↑ ↑                                                                   │   ║
║   │ │ └─ Orange dot = dirty (no size info)                                │   ║
║   │ └─── Green dot = running                                              │   ║
║   └───────────────────────────────────────────────────────────────────────┘   ║
║                                                                               ║
║   USER_FLOW: See orange dot → "Something changed, but how much?"              ║
║   PAIN_POINT: Can't gauge scope of changes without `kild diff`                ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

### After State

```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                               AFTER STATE                                      ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║   KILD List Row:                                                              ║
║   ┌───────────────────────────────────────────────────────────────────────┐   ║
║   │ ●  feature-auth   claude   kild   23m   +42 -12   JWT...   [Copy]...  │   ║
║   │ ↑                                      ^^^^^^^^                       │   ║
║   │ │                                      │                              │   ║
║   │ │                                      └─ Green +N, red -N            │   ║
║   │ └─── Green dot = running (unchanged)                                  │   ║
║   └───────────────────────────────────────────────────────────────────────┘   ║
║                                                                               ║
║   Clean worktree (no changes):                                                ║
║   ┌───────────────────────────────────────────────────────────────────────┐   ║
║   │ ●  feature-api   claude   kild   1h   Refactor...   [Copy][Edit]      │   ║
║   │                               (no stats shown - clean)                │   ║
║   └───────────────────────────────────────────────────────────────────────┘   ║
║                                                                               ║
║   USER_FLOW: See "+42 -12" → "Substantial changes, should review"             ║
║   VALUE_ADD: Instant scope visibility without additional commands             ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

### Interaction Changes

| Location | Before | After | User Impact |
|----------|--------|-------|-------------|
| List row | Orange `●` when dirty | `+N -N` when dirty | See scope of changes |
| List row | Nothing when clean | Nothing when clean | No change |
| List row | `?` when unknown | `?` when unknown | No change |

---

## Mandatory Reading

**CRITICAL: Implementation agent MUST read these files before starting any task:**

| Priority | File | Lines | Why Read This |
|----------|------|-------|---------------|
| P0 | `crates/kild-core/src/git/types.rs` | all | Pattern for type definitions, add DiffStats here |
| P0 | `crates/kild-core/src/git/errors.rs` | all | Error enum pattern, no changes needed (use existing Git2Error) |
| P0 | `crates/kild-core/src/git/handler.rs` | 130-220 | Repository::open pattern, add get_diff_stats here |
| P1 | `crates/kild-ui/src/state.rs` | 40-120 | KildDisplay struct, check_git_status pattern |
| P1 | `crates/kild-ui/src/views/kild_list.rs` | 185-200 | Current git dirty indicator rendering |
| P2 | `crates/kild-ui/src/theme.rs` | 74-108 | aurora() for green, ember() for red |

**External Documentation:**

| Source | Section | Why Needed |
|--------|---------|------------|
| [git2 DiffStats](https://docs.rs/git2/latest/git2/struct.DiffStats.html) | Method signatures | insertions(), deletions(), files_changed() return usize |
| [git2 Diff](https://docs.rs/git2/latest/git2/struct.Diff.html) | diff_index_to_workdir | Create diff between index and workdir |

---

## Patterns to Mirror

**TYPE_DEFINITION:**
```rust
// SOURCE: crates/kild-core/src/git/types.rs:10-16
// COPY THIS PATTERN:
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectInfo {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub remote_url: Option<String>,
}
```

**GIT2_REPOSITORY_OPEN:**
```rust
// SOURCE: crates/kild-core/src/git/handler.rs:134
// COPY THIS PATTERN:
let repo = Repository::open(&project.path).map_err(|e| GitError::Git2Error { source: e })?;
```

**LOGGING_PATTERN:**
```rust
// SOURCE: crates/kild-ui/src/state.rs:67-73
// COPY THIS PATTERN:
tracing::warn!(
    event = "ui.kild_list.git_status_failed",
    path = %worktree_path.display(),
    exit_code = ?output.status.code(),
    "Git status command failed"
);
```

**UI_STATE_COMPUTATION:**
```rust
// SOURCE: crates/kild-ui/src/state.rs:107-111
// COPY THIS PATTERN:
let git_status = if session.worktree_path.exists() {
    check_git_status(&session.worktree_path)
} else {
    GitStatus::Unknown
};
```

**CONDITIONAL_RENDERING:**
```rust
// SOURCE: crates/kild-ui/src/views/kild_list.rs:186-190
// COPY THIS PATTERN:
.when(git_status == GitStatus::Dirty, |row| {
    row.child(
        div().text_color(theme::copper()).child("●"),
    )
})
```

---

## Files to Change

| File | Action | Justification |
| ---- | ------ | ------------- |
| `crates/kild-core/src/git/types.rs` | UPDATE | Add DiffStats struct |
| `crates/kild-core/src/git/operations.rs` | UPDATE | Add get_diff_stats() function |
| `crates/kild-core/src/git/mod.rs` | UPDATE | Re-export DiffStats from types |
| `crates/kild-ui/src/state.rs` | UPDATE | Add diff_stats to KildDisplay, compute in from_session |
| `crates/kild-ui/src/views/kild_list.rs` | UPDATE | Replace orange dot with +N -N display |

---

## NOT Building (Scope Limits)

Explicit exclusions to prevent scope creep:

- **Staged vs unstaged split** - Just total stats, not staged/unstaged breakdown
- **Per-file breakdown** - Just totals, not file-by-file stats
- **files_changed display** - Only show in detail panel (Phase 9.8), not in list
- **Clickable stats** - No click-to-show-diff functionality
- **Caching** - Recompute on each refresh (5-second interval is fine)
- **Background fetching** - Synchronous in from_session() is acceptable

---

## Step-by-Step Tasks

Execute in order. Each task is atomic and independently verifiable.

### Task 1: UPDATE `crates/kild-core/src/git/types.rs` - Add DiffStats struct

**ACTION**: Add DiffStats struct with Default impl

**IMPLEMENT**:
```rust
/// Git diff statistics for a worktree.
///
/// Represents the number of lines added, removed, and files changed
/// compared to the index (uncommitted changes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DiffStats {
    /// Number of lines added
    pub insertions: usize,
    /// Number of lines removed
    pub deletions: usize,
    /// Number of files changed
    pub files_changed: usize,
}
```

**MIRROR**: `crates/kild-core/src/git/types.rs:10-16` - follow ProjectInfo pattern

**GOTCHA**: Use `Copy` since it's small (3 usizes). Use `Default` for zero-stats case.

**VALIDATE**: `cargo build -p kild-core && cargo clippy -p kild-core -- -D warnings`

---

### Task 2: UPDATE `crates/kild-core/src/git/operations.rs` - Add get_diff_stats function

**ACTION**: Add function to compute diff stats using git2

**IMPLEMENT**:
```rust
use git2::Repository;
use std::path::Path;
use crate::git::errors::GitError;
use crate::git::types::DiffStats;

/// Get diff statistics for uncommitted changes in a worktree.
///
/// Returns the number of insertions, deletions, and files changed
/// between the index and the working directory.
///
/// # Errors
///
/// Returns `GitError::Git2Error` if the repository cannot be opened
/// or the diff cannot be computed.
pub fn get_diff_stats(worktree_path: &Path) -> Result<DiffStats, GitError> {
    let repo = Repository::open(worktree_path).map_err(|e| GitError::Git2Error { source: e })?;

    let diff = repo
        .diff_index_to_workdir(None, None)
        .map_err(|e| GitError::Git2Error { source: e })?;

    let stats = diff.stats().map_err(|e| GitError::Git2Error { source: e })?;

    Ok(DiffStats {
        insertions: stats.insertions(),
        deletions: stats.deletions(),
        files_changed: stats.files_changed(),
    })
}
```

**MIRROR**: `crates/kild-core/src/git/handler.rs:134` - Repository::open pattern

**IMPORTS**: Add `use git2::Repository;` at top of file

**GOTCHA**: `diff_index_to_workdir(None, None)` uses default index and options - this matches `git diff` behavior

**VALIDATE**: `cargo build -p kild-core && cargo clippy -p kild-core -- -D warnings`

---

### Task 3: UPDATE `crates/kild-core/src/git/mod.rs` - Re-export DiffStats

**ACTION**: Add DiffStats to public exports (if not already auto-exported via `types`)

**CURRENT STATE**: Module just declares submodules, types are accessed via `git::types::DiffStats`

**IMPLEMENT**: No change needed - `pub mod types;` already exports it. Users import via:
```rust
use kild_core::git::types::DiffStats;
// or
use kild_core::git::operations::get_diff_stats;
```

**VALIDATE**: `cargo build -p kild-core`

---

### Task 4: UPDATE `crates/kild-ui/src/state.rs` - Add diff_stats to KildDisplay

**ACTION**: Add diff_stats field and compute it in from_session()

**IMPLEMENT**:

1. Add import at top:
```rust
use kild_core::git::{operations::get_diff_stats, types::DiffStats};
```

2. Add field to KildDisplay struct (line ~45):
```rust
#[derive(Clone)]
pub struct KildDisplay {
    pub session: Session,
    pub status: ProcessStatus,
    pub git_status: GitStatus,
    pub diff_stats: Option<DiffStats>,  // NEW
}
```

3. Compute in from_session() after git_status check (line ~107):
```rust
let git_status = if session.worktree_path.exists() {
    check_git_status(&session.worktree_path)
} else {
    GitStatus::Unknown
};

// Compute diff stats if worktree exists and is dirty
let diff_stats = if git_status == GitStatus::Dirty {
    match get_diff_stats(&session.worktree_path) {
        Ok(stats) => Some(stats),
        Err(e) => {
            tracing::debug!(
                event = "ui.kild_list.diff_stats_failed",
                path = %session.worktree_path.display(),
                error = %e,
                "Failed to compute diff stats"
            );
            None
        }
    }
} else {
    None
};

Self {
    session,
    status,
    git_status,
    diff_stats,  // NEW
}
```

**MIRROR**: `crates/kild-ui/src/state.rs:107-111` - git_status computation pattern

**GOTCHA**: Only compute stats if dirty - no point computing for clean repos. Use `debug!` not `warn!` for failures since it's non-critical info.

**VALIDATE**: `cargo build -p kild-ui && cargo clippy -p kild-ui -- -D warnings`

---

### Task 5: UPDATE `crates/kild-ui/src/views/kild_list.rs` - Display diff stats

**ACTION**: Replace orange dot with `+N -N` display when stats available

**IMPLEMENT**:

1. Add import for DiffStats (if needed - may come through KildDisplay):
```rust
// At top, ensure state imports include what we need
use crate::state::{AppState, GitStatus, ProcessStatus};
```

2. In the row rendering (around line 185-197), replace the git dirty indicator:

**BEFORE** (lines 185-197):
```rust
// Git dirty indicator (orange dot when dirty, gray ? when unknown)
.when(git_status == GitStatus::Dirty, |row| {
    row.child(
        div().text_color(theme::copper()).child("●"),
    )
})
.when(git_status == GitStatus::Unknown, |row| {
    row.child(
        div()
            .text_color(theme::text_muted())
            .child("?"),
    )
})
```

**AFTER**:
```rust
// Git diff stats (when dirty) or unknown indicator
.when_some(display.diff_stats, |row, stats| {
    row.child(
        div()
            .flex()
            .gap(px(theme::SPACE_1))
            .text_size(px(theme::TEXT_SM))
            .child(
                div()
                    .text_color(theme::aurora())
                    .child(format!("+{}", stats.insertions)),
            )
            .child(
                div()
                    .text_color(theme::ember())
                    .child(format!("-{}", stats.deletions)),
            ),
    )
})
// Fallback: dirty but no stats (shouldn't happen often)
.when(git_status == GitStatus::Dirty && display.diff_stats.is_none(), |row| {
    row.child(div().text_color(theme::copper()).child("●"))
})
// Unknown git status
.when(git_status == GitStatus::Unknown, |row| {
    row.child(div().text_color(theme::text_muted()).child("?"))
})
```

**MIRROR**: `crates/kild-ui/src/views/kild_list.rs:186-197` - conditional rendering pattern

**GOTCHA**: Need to clone `diff_stats` for use in closure since `display` is borrowed. Actually, `DiffStats` is `Copy` so no clone needed.

**VALIDATE**: `cargo build -p kild-ui && cargo clippy -p kild-ui -- -D warnings`

---

### Task 6: ADD tests for get_diff_stats

**ACTION**: Add unit tests for the new function

**IMPLEMENT** in `crates/kild-core/src/git/operations.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn init_git_repo(dir: &Path) {
        Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .output()
            .expect("Failed to init git repo");
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir)
            .output()
            .expect("Failed to set git email");
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir)
            .output()
            .expect("Failed to set git name");
    }

    #[test]
    fn test_get_diff_stats_clean_repo() {
        let dir = TempDir::new().unwrap();
        init_git_repo(dir.path());

        // Create and commit a file
        fs::write(dir.path().join("test.txt"), "hello").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let stats = get_diff_stats(dir.path()).unwrap();
        assert_eq!(stats.insertions, 0);
        assert_eq!(stats.deletions, 0);
        assert_eq!(stats.files_changed, 0);
    }

    #[test]
    fn test_get_diff_stats_with_changes() {
        let dir = TempDir::new().unwrap();
        init_git_repo(dir.path());

        // Create and commit a file
        fs::write(dir.path().join("test.txt"), "line1\nline2\n").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "initial"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // Make changes
        fs::write(dir.path().join("test.txt"), "line1\nmodified\nnew line\n").unwrap();

        let stats = get_diff_stats(dir.path()).unwrap();
        assert!(stats.insertions > 0 || stats.deletions > 0);
        assert_eq!(stats.files_changed, 1);
    }

    #[test]
    fn test_get_diff_stats_not_a_repo() {
        let dir = TempDir::new().unwrap();
        // Don't init git

        let result = get_diff_stats(dir.path());
        assert!(result.is_err());
    }
}
```

**MIRROR**: `crates/kild-core/src/git/types.rs:46-91` - test structure pattern

**GOTCHA**: Tests need `tempfile` crate - check if already in dev-dependencies

**VALIDATE**: `cargo test -p kild-core -- git::operations::tests`

---

## Testing Strategy

### Unit Tests to Write

| Test File | Test Cases | Validates |
|-----------|------------|-----------|
| `crates/kild-core/src/git/operations.rs` | clean repo, dirty repo, not a repo | get_diff_stats function |

### Edge Cases Checklist

- [ ] Clean repository (no uncommitted changes) → stats all zeros
- [ ] Dirty repository with additions only → insertions > 0, deletions = 0
- [ ] Dirty repository with deletions only → insertions = 0, deletions > 0
- [ ] Dirty repository with both → both > 0
- [ ] Path is not a git repo → returns error
- [ ] Worktree doesn't exist → from_session returns None for diff_stats
- [ ] Binary files changed → still counts as files_changed

---

## Validation Commands

### Level 1: STATIC_ANALYSIS

```bash
cargo fmt --check && cargo clippy --all -- -D warnings
```

**EXPECT**: Exit 0, no errors or warnings

### Level 2: UNIT_TESTS

```bash
cargo test -p kild-core -- git::operations
cargo test -p kild-ui
```

**EXPECT**: All tests pass

### Level 3: FULL_SUITE

```bash
cargo test --all && cargo build --all && cargo clippy --all -- -D warnings
```

**EXPECT**: All tests pass, build succeeds

### Level 4: MANUAL_VALIDATION

```bash
cargo run -p kild-ui
```

1. Create a kild with `+ Create`
2. Make changes in the worktree (add/delete lines in a file)
3. Click Refresh or wait for auto-refresh
4. Verify row shows `+N -N` in green/red instead of orange dot
5. Verify clean kilds show no stats

---

## Acceptance Criteria

- [ ] `DiffStats` struct exists in kild-core with insertions, deletions, files_changed
- [ ] `get_diff_stats()` function uses git2 API (not shell out)
- [ ] `KildDisplay` has `diff_stats: Option<DiffStats>` field
- [ ] List rows show `+N -N` for dirty kilds (green/red colored)
- [ ] List rows show nothing for clean kilds
- [ ] List rows show `?` for unknown git status
- [ ] Level 1-3 validation commands pass with exit 0
- [ ] Unit tests cover clean, dirty, and error cases

---

## Completion Checklist

- [ ] Task 1: DiffStats struct added to types.rs
- [ ] Task 2: get_diff_stats() added to operations.rs
- [ ] Task 3: Exports verified (no change needed)
- [ ] Task 4: KildDisplay updated with diff_stats field
- [ ] Task 5: kild_list.rs renders +N -N display
- [ ] Task 6: Unit tests added and passing
- [ ] All validation commands pass

---

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Performance: git2 diff slow on large repos | LOW | MED | Only compute for dirty repos, debug log timing if needed |
| git2 API differences from CLI | LOW | LOW | Test against real repos, both methods should give same results |
| Rendering cluttered with stats | LOW | LOW | Use small text, subtle colors, can adjust in follow-up |

---

## Notes

**Color Choices:**
- `theme::aurora()` (green) for insertions - matches git convention
- `theme::ember()` (red) for deletions - matches git convention
- Removed `theme::copper()` (orange) dot since we now have precise stats

**Why git2 instead of shell:**
- Already using git2 for other operations
- Type-safe API with proper error handling
- No subprocess overhead
- Consistent with CLAUDE.md preference for libraries over shell

**Future Enhancement (Phase 9.8):**
- Detail panel will show `files_changed` count
- Could add click-to-expand showing per-file stats

**Sources:**
- [git2 DiffStats documentation](https://docs.rs/git2/latest/git2/struct.DiffStats.html)
- [git2 Diff documentation](https://docs.rs/git2/latest/git2/struct.Diff.html)
