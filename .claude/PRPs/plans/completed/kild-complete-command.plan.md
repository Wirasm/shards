# Feature: kild complete Command

## Summary

Add a new `kild complete <branch>` command that provides a clean workflow for finishing work in a kild. The command detects whether a PR has been merged and adapts its behavior: if the PR was already merged (user ran `gh pr merge` first), it also deletes the orphaned remote branch; if the PR hasn't been merged yet (user will merge after), it just destroys the kild so `gh pr merge --delete-branch` can work.

## User Story

As a developer finishing work in a kild
I want a single command to clean up after merging (or before merging) a PR
So that I don't have orphaned remote branches regardless of the order I run commands

## Problem Statement

When merging a PR created from a kild, `gh pr merge --delete-branch` fails to delete the local branch because it's checked out in the worktree. This leaves the remote branch orphaned after `kild destroy`. Users must manually clean up with `git push origin --delete <branch>`.

## Solution Statement

Add `kild complete <branch>` that:
1. Checks if there's a merged PR for the kild's branch (via `gh pr view`)
2. If merged: delete the remote branch (since `gh pr merge --delete-branch` would have failed)
3. Destroy the kild (reuse existing `destroy_session` logic)

This handles both orderings:
- **Complete first, then merge**: Just destroys kild; merge will delete remote
- **Merge first, then complete**: Destroys kild AND deletes orphaned remote

## Metadata

| Field            | Value                                          |
| ---------------- | ---------------------------------------------- |
| Type             | NEW_CAPABILITY                                 |
| Complexity       | MEDIUM                                         |
| Systems Affected | kild CLI, kild-core sessions                   |
| Dependencies     | gh CLI (for PR status check)                   |
| Estimated Tasks  | 7                                              |

---

## UX Design

### Before State

```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                              BEFORE STATE                                      ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║   Scenario A: Merge first, destroy second                                     ║
║   ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐          ║
║   │ gh pr merge     │───►│ kild destroy    │───►│ Remote branch   │          ║
║   │ --delete-branch │    │                 │    │ ORPHANED!       │          ║
║   │ (branch delete  │    │ (cleans local)  │    │                 │          ║
║   │  FAILS)         │    │                 │    │ Manual cleanup  │          ║
║   └─────────────────┘    └─────────────────┘    └─────────────────┘          ║
║                                                                               ║
║   USER_FLOW: merge → destroy → manually delete remote                         ║
║   PAIN_POINT: Must remember to run `git push origin --delete <branch>`        ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

### After State

```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                               AFTER STATE                                      ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║   Scenario A: Merge first, complete second                                    ║
║   ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐          ║
║   │ gh pr merge     │───►│ kild complete   │───►│ All clean!      │          ║
║   │ --squash        │    │ (detects merged │    │ - Local gone    │          ║
║   │                 │    │  deletes remote │    │ - Remote gone   │          ║
║   │                 │    │  destroys kild) │    │ - Worktree gone │          ║
║   └─────────────────┘    └─────────────────┘    └─────────────────┘          ║
║                                                                               ║
║   Scenario B: Complete first, merge second                                    ║
║   ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐          ║
║   │ kild complete   │───►│ gh pr merge     │───►│ All clean!      │          ║
║   │ (destroys kild, │    │ --delete-branch │    │ - Local gone    │          ║
║   │  no remote del) │    │ (NOW WORKS!)    │    │ - Remote gone   │          ║
║   └─────────────────┘    └─────────────────┘    └─────────────────┘          ║
║                                                                               ║
║   USER_FLOW: Either order works, no orphaned branches                         ║
║   VALUE_ADD: Single command, adapts to user's workflow                        ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

### Interaction Changes

| Location | Before | After | User Impact |
|----------|--------|-------|-------------|
| `kild destroy` | Only destroys local | Unchanged | Still available for non-PR kilds |
| `kild complete` | N/A | New command | Clean PR completion workflow |
| Remote branches | Often orphaned | Auto-cleaned if needed | No manual cleanup |

---

## Mandatory Reading

**CRITICAL: Implementation agent MUST read these files before starting any task:**

| Priority | File | Lines | Why Read This |
|----------|------|-------|---------------|
| P0 | `crates/kild/src/app.rs` | 80-102 | destroy subcommand pattern to MIRROR |
| P0 | `crates/kild/src/commands.rs` | 59-82 | run_command routing to ADD to |
| P0 | `crates/kild/src/commands.rs` | 229-268 | handle_destroy_command to MIRROR |
| P0 | `crates/kild-core/src/sessions/handler.rs` | 233-377 | destroy_session to REUSE |
| P1 | `crates/kild-core/src/sessions/errors.rs` | 1-61 | SessionError variants |
| P1 | `crates/kild/src/commands.rs` | 868-890 | Command::new("git") pattern |

---

## Patterns to Mirror

**CLI_SUBCOMMAND_DEFINITION:**
```rust
// SOURCE: crates/kild/src/app.rs:80-102
// COPY THIS PATTERN:
.subcommand(
    Command::new("destroy")
        .about("Remove kild completely")
        .arg(
            Arg::new("branch")
                .help("Branch name of the kild to destroy")
                .required_unless_present("all")
                .index(1)
        )
        .arg(
            Arg::new("force")
                .long("force")
                .short('f')
                .help("Force destroy, bypassing git uncommitted changes check")
                .action(ArgAction::SetTrue)
        )
)
```

**COMMAND_HANDLER:**
```rust
// SOURCE: crates/kild/src/commands.rs:229-268
// COPY THIS PATTERN:
fn handle_destroy_command(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let force = matches.get_flag("force");

    let branch = matches
        .get_one::<String>("branch")
        .ok_or("Branch argument is required")?;

    info!(
        event = "cli.destroy_started",
        branch = branch,
        force = force
    );

    match session_handler::destroy_session(branch, force) {
        Ok(()) => {
            println!("✅ KILD '{}' destroyed successfully!", branch);
            info!(event = "cli.destroy_completed", branch = branch);
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Failed to destroy kild '{}': {}", branch, e);
            error!(event = "cli.destroy_failed", branch = branch, error = %e);
            events::log_app_error(&e);
            Err(e.into())
        }
    }
}
```

**EXTERNAL_COMMAND_PATTERN:**
```rust
// SOURCE: crates/kild/src/commands.rs:868-873
// COPY THIS PATTERN for gh CLI calls:
let output = std::process::Command::new("git")
    .current_dir(&session.worktree_path)
    .args(["log", "--oneline", "-n", &count.to_string()])
    .output()
    .map_err(|e| {
        eprintln!("Failed to execute git: {}", e);
        e
    })?;
```

**SESSION_HANDLER_LOGGING:**
```rust
// SOURCE: crates/kild-core/src/sessions/handler.rs:248-252
// COPY THIS PATTERN:
info!(
    event = "core.session.destroy_started",
    name = name,
    force = force
);
```

---

## Files to Change

| File | Action | Justification |
|------|--------|---------------|
| `crates/kild/src/app.rs` | UPDATE | Add `complete` subcommand definition |
| `crates/kild/src/commands.rs` | UPDATE | Add routing + handler for complete |
| `crates/kild-core/src/sessions/handler.rs` | UPDATE | Add `complete_session` function |
| `crates/kild-core/src/sessions/errors.rs` | UPDATE | Add error variants for PR/remote ops |
| `crates/kild/src/app.rs` | UPDATE | Add CLI tests for complete command |

---

## NOT Building (Scope Limits)

Explicit exclusions to prevent scope creep:

- **No `--all` flag** - Unlike destroy, complete is for individual PRs. Bulk completion doesn't make sense.
- **No auto-merge** - The command doesn't merge the PR, just handles cleanup
- **No GitHub API integration** - Use `gh` CLI which handles auth
- **No config option for auto-complete** - Keep it explicit

---

## Step-by-Step Tasks

Execute in order. Each task is atomic and independently verifiable.

### Task 1: ADD error variants to `crates/kild-core/src/sessions/errors.rs`

- **ACTION**: Add new error variants for PR check and remote branch operations
- **FILE**: `crates/kild-core/src/sessions/errors.rs`
- **IMPLEMENT**:

```rust
// Add after line 59 (before closing brace of enum):

#[error("Failed to check PR status: {message}")]
PrCheckFailed { message: String },

#[error("Failed to delete remote branch '{branch}': {message}")]
RemoteBranchDeleteFailed { branch: String, message: String },
```

- **ALSO UPDATE** `error_code()` match (around line 64):
```rust
SessionError::PrCheckFailed { .. } => "PR_CHECK_FAILED",
SessionError::RemoteBranchDeleteFailed { .. } => "REMOTE_BRANCH_DELETE_FAILED",
```

- **ALSO UPDATE** `is_user_error()` match (around line 85):
```rust
// Add to the matches! list:
| SessionError::PrCheckFailed { .. }
| SessionError::RemoteBranchDeleteFailed { .. }
```

- **VALIDATE**: `cargo build -p kild-core`

### Task 2: ADD `complete_session` to `crates/kild-core/src/sessions/handler.rs`

- **ACTION**: Add new public function for completing a kild
- **FILE**: `crates/kild-core/src/sessions/handler.rs`
- **INSERT AFTER**: Line 377 (after `destroy_session` function)
- **IMPLEMENT**:

```rust
/// Completes a kild by checking PR status, optionally deleting remote branch, and destroying the session.
///
/// # Arguments
/// * `name` - Branch name or kild identifier
/// * `force` - If true, bypass git safety checks
///
/// # Returns
/// * `Ok(true)` - Completed and remote branch was deleted (PR was merged)
/// * `Ok(false)` - Completed but remote branch not deleted (PR not merged yet, or no remote)
///
/// # Workflow Detection
/// - If PR is merged: delete remote branch (since gh merge --delete-branch would have failed)
/// - If PR not merged: just destroy, let user's subsequent merge handle remote
pub fn complete_session(name: &str, force: bool) -> Result<bool, SessionError> {
    info!(
        event = "core.session.complete_started",
        name = name,
        force = force
    );

    let config = Config::new();

    // 1. Find session by name to get branch info
    let session =
        operations::find_session_by_name(&config.sessions_dir(), name)?.ok_or_else(|| {
            SessionError::NotFound {
                name: name.to_string(),
            }
        })?;

    let kild_branch = format!("kild_{}", name);

    // 2. Check if PR was merged (determines if we need to delete remote)
    let pr_merged = check_pr_merged(&session.worktree_path, &kild_branch);

    info!(
        event = "core.session.complete_pr_status",
        branch = name,
        pr_merged = pr_merged
    );

    // 3. If PR was merged, delete remote branch (it would be orphaned)
    let remote_deleted = if pr_merged {
        match delete_remote_branch(&session.worktree_path, &kild_branch) {
            Ok(()) => {
                info!(
                    event = "core.session.complete_remote_deleted",
                    branch = kild_branch
                );
                true
            }
            Err(e) => {
                // Non-fatal: remote might already be deleted or not exist
                warn!(
                    event = "core.session.complete_remote_delete_failed",
                    branch = kild_branch,
                    error = %e
                );
                false
            }
        }
    } else {
        false
    };

    // 4. Destroy the session (reuse existing logic)
    destroy_session(name, force)?;

    info!(
        event = "core.session.complete_completed",
        name = name,
        remote_deleted = remote_deleted
    );

    Ok(remote_deleted)
}

/// Check if there's a merged PR for the given branch using gh CLI.
/// Returns false if gh is not available, PR doesn't exist, or PR is not merged.
fn check_pr_merged(worktree_path: &std::path::Path, branch: &str) -> bool {
    let output = std::process::Command::new("gh")
        .current_dir(worktree_path)
        .args(["pr", "view", branch, "--json", "state", "-q", ".state"])
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let state = String::from_utf8_lossy(&output.stdout).trim().to_uppercase();
            state == "MERGED"
        }
        _ => false, // gh not available, no PR, or error - assume not merged
    }
}

/// Delete a remote branch using git push.
fn delete_remote_branch(worktree_path: &std::path::Path, branch: &str) -> Result<(), SessionError> {
    info!(
        event = "core.session.complete_remote_delete_started",
        branch = branch
    );

    let output = std::process::Command::new("git")
        .current_dir(worktree_path)
        .args(["push", "origin", "--delete", branch])
        .output()
        .map_err(|e| SessionError::RemoteBranchDeleteFailed {
            branch: branch.to_string(),
            message: format!("Failed to execute git: {}", e),
        })?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Check if branch doesn't exist (not an error)
        if stderr.contains("remote ref does not exist") {
            info!(
                event = "core.session.complete_remote_already_deleted",
                branch = branch
            );
            Ok(())
        } else {
            Err(SessionError::RemoteBranchDeleteFailed {
                branch: branch.to_string(),
                message: stderr.to_string(),
            })
        }
    }
}
```

- **VALIDATE**: `cargo build -p kild-core`

### Task 3: ADD `complete` subcommand to `crates/kild/src/app.rs`

- **ACTION**: Add complete subcommand definition
- **FILE**: `crates/kild/src/app.rs`
- **INSERT AFTER**: Line 102 (after destroy subcommand, before open subcommand)
- **IMPLEMENT**:

```rust
.subcommand(
    Command::new("complete")
        .about("Complete a kild: destroy and clean up remote branch if PR was merged")
        .long_about(
            "Completes a kild by destroying the worktree and optionally deleting the remote branch.\n\n\
            If the PR was already merged (user ran 'gh pr merge' first), this command also deletes\n\
            the orphaned remote branch. If the PR hasn't been merged yet, it just destroys the kild\n\
            so that 'gh pr merge --delete-branch' can work afterwards.\n\n\
            Works with either workflow:\n\
            - Complete first, then merge: kild complete → gh pr merge --delete-branch\n\
            - Merge first, then complete: gh pr merge → kild complete (deletes remote)"
        )
        .arg(
            Arg::new("branch")
                .help("Branch name of the kild to complete")
                .required(true)
                .index(1)
        )
        .arg(
            Arg::new("force")
                .long("force")
                .short('f')
                .help("Force completion, bypassing git uncommitted changes check")
                .action(ArgAction::SetTrue)
        )
)
```

- **VALIDATE**: `cargo build -p kild`

### Task 4: ADD command routing in `crates/kild/src/commands.rs`

- **ACTION**: Add match arm for complete command
- **FILE**: `crates/kild/src/commands.rs`
- **LOCATION**: Line 66 area (in `run_command` match)
- **INSERT** (after destroy line):

```rust
Some(("complete", sub_matches)) => handle_complete_command(sub_matches),
```

- **VALIDATE**: `cargo build -p kild` (will fail until handler exists)

### Task 5: ADD `handle_complete_command` in `crates/kild/src/commands.rs`

- **ACTION**: Add handler function
- **FILE**: `crates/kild/src/commands.rs`
- **INSERT AFTER**: `handle_destroy_command` function (around line 268)
- **IMPLEMENT**:

```rust
fn handle_complete_command(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let branch = matches
        .get_one::<String>("branch")
        .ok_or("Branch argument is required")?;

    if !is_valid_branch_name(branch) {
        eprintln!("Invalid branch name: {}", branch);
        error!(event = "cli.complete_invalid_branch", branch = branch);
        return Err("Invalid branch name".into());
    }

    let force = matches.get_flag("force");

    info!(
        event = "cli.complete_started",
        branch = branch,
        force = force
    );

    match session_handler::complete_session(branch, force) {
        Ok(remote_deleted) => {
            println!("✅ KILD '{}' completed!", branch);
            if remote_deleted {
                println!("   Remote branch also deleted (PR was merged)");
            } else {
                println!("   Remote branch preserved (merge will delete it)");
            }

            info!(
                event = "cli.complete_completed",
                branch = branch,
                remote_deleted = remote_deleted
            );

            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Failed to complete kild '{}': {}", branch, e);

            error!(
                event = "cli.complete_failed",
                branch = branch,
                error = %e
            );

            events::log_app_error(&e);
            Err(e.into())
        }
    }
}
```

- **VALIDATE**: `cargo build -p kild`

### Task 6: ADD CLI tests in `crates/kild/src/app.rs`

- **ACTION**: Add tests for complete command parsing
- **FILE**: `crates/kild/src/app.rs`
- **INSERT IN**: `#[cfg(test)]` module at end of file
- **IMPLEMENT**:

```rust
#[test]
fn test_cli_complete_command() {
    let app = build_cli();
    let matches = app.try_get_matches_from(vec!["kild", "complete", "test-branch"]);
    assert!(matches.is_ok());

    let matches = matches.unwrap();
    let complete_matches = matches.subcommand_matches("complete").unwrap();
    assert_eq!(
        complete_matches.get_one::<String>("branch").unwrap(),
        "test-branch"
    );
    assert!(!complete_matches.get_flag("force"));
}

#[test]
fn test_cli_complete_command_with_force() {
    let app = build_cli();
    let matches = app.try_get_matches_from(vec!["kild", "complete", "test-branch", "--force"]);
    assert!(matches.is_ok());

    let matches = matches.unwrap();
    let complete_matches = matches.subcommand_matches("complete").unwrap();
    assert!(complete_matches.get_flag("force"));
}

#[test]
fn test_cli_complete_command_force_short() {
    let app = build_cli();
    let matches = app.try_get_matches_from(vec!["kild", "complete", "test-branch", "-f"]);
    assert!(matches.is_ok());

    let matches = matches.unwrap();
    let complete_matches = matches.subcommand_matches("complete").unwrap();
    assert!(complete_matches.get_flag("force"));
}

#[test]
fn test_cli_complete_requires_branch() {
    let app = build_cli();
    let matches = app.try_get_matches_from(vec!["kild", "complete"]);
    assert!(matches.is_err());
}
```

- **VALIDATE**: `cargo test -p kild -- test_cli_complete`

### Task 7: UPDATE documentation

- **ACTION**: Add complete command to CLAUDE.md and update skill docs
- **FILES**:
  - `CLAUDE.md` - Add to Build & Development Commands section
  - `.claude/skills/kild/SKILL.md` - Add complete command documentation

**CLAUDE.md addition** (after line 85, in the kild commands section):
```markdown
cargo run -p kild -- complete my-branch          # Complete kild (check PR, cleanup)
cargo run -p kild -- complete my-branch --force  # Force complete (bypass git checks)
```

**.claude/skills/kild/SKILL.md addition** (add new section):
```markdown
### Complete a Kild (PR Cleanup)
```bash
kild complete <branch> [--force]
```

Completes a kild by destroying it and cleaning up the remote branch if the PR was merged.

**Workflow A: Complete first, then merge**
```bash
kild complete my-feature    # Destroys kild
gh pr merge 123 --delete-branch  # Merges PR, deletes remote (now works!)
```

**Workflow B: Merge first, then complete**
```bash
gh pr merge 123 --squash    # Merges PR (can't delete remote due to worktree)
kild complete my-feature    # Destroys kild AND deletes orphaned remote
```

**Flags**
- `--force` / `-f` - Force complete even with uncommitted changes
```

- **VALIDATE**: Review documentation for accuracy

---

## Testing Strategy

### Unit Tests to Write

| Test File | Test Cases | Validates |
|-----------|------------|-----------|
| `crates/kild/src/app.rs` | 4 CLI parsing tests | Command definition |

### Manual Verification

- [ ] `kild complete my-branch` works when PR is not merged (just destroys)
- [ ] `kild complete my-branch` works when PR is merged (destroys + deletes remote)
- [ ] `kild complete my-branch --force` bypasses git checks
- [ ] Error message is clear when kild doesn't exist
- [ ] Works without `gh` CLI (gracefully assumes PR not merged)

### Edge Cases Checklist

- [ ] Kild doesn't exist - clear error message
- [ ] PR doesn't exist - treats as not merged, just destroys
- [ ] gh CLI not installed - treats as not merged, just destroys
- [ ] Remote branch already deleted - success (not an error)
- [ ] Network error during remote delete - warning, continues with destroy
- [ ] Uncommitted changes without --force - blocks with clear message

---

## Validation Commands

### Level 1: STATIC_ANALYSIS

```bash
cargo fmt --check && cargo clippy --all -- -D warnings
```

**EXPECT**: Exit 0, no errors or warnings

### Level 2: UNIT_TESTS

```bash
cargo test -p kild -- test_cli_complete
cargo test -p kild-core
```

**EXPECT**: All tests pass

### Level 3: FULL_SUITE

```bash
cargo test --all && cargo build --all
```

**EXPECT**: All tests pass, build succeeds

### Level 4: MANUAL_TESTING

```bash
# Create a test kild
kild create test-complete-cmd

# Test completion without PR
kild complete test-complete-cmd
# Expected: destroys, says "Remote branch preserved"

# Create another, make a PR, merge it
kild create test-complete-merged
# ... create PR, merge it ...
kild complete test-complete-merged
# Expected: destroys AND says "Remote branch also deleted"
```

---

## Acceptance Criteria

- [ ] `kild complete <branch>` command exists and works
- [ ] Detects merged PR status via `gh pr view`
- [ ] Deletes remote branch only if PR was merged
- [ ] Reuses `destroy_session` for local cleanup
- [ ] Handles missing `gh` CLI gracefully
- [ ] All CLI parsing tests pass
- [ ] Documentation updated

---

## Completion Checklist

- [ ] Task 1: Error variants added to sessions/errors.rs
- [ ] Task 2: complete_session function added to handler.rs
- [ ] Task 3: CLI subcommand added to app.rs
- [ ] Task 4: Command routing added to commands.rs
- [ ] Task 5: Handler function added to commands.rs
- [ ] Task 6: CLI tests added and passing
- [ ] Task 7: Documentation updated
- [ ] Level 1: Static analysis passes
- [ ] Level 2: Unit tests pass
- [ ] Level 3: Full suite passes
- [ ] Level 4: Manual testing verified

---

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| gh CLI not installed | MED | LOW | Gracefully assume not merged, just destroy |
| Network errors during remote delete | MED | LOW | Log warning, continue with destroy |
| User expects auto-merge | LOW | MED | Clear documentation that this only cleans up |
| gh auth issues | LOW | LOW | gh handles auth, errors surface naturally |

---

## Notes

**Design Decision: Why not auto-merge?**
The `complete` command deliberately does NOT merge the PR. This keeps it simple and lets users control the merge strategy (squash, rebase, merge). The command's job is cleanup, not workflow automation.

**gh CLI Dependency**
The command degrades gracefully without `gh`:
- If `gh` not installed: assumes PR not merged, just destroys
- If `gh pr view` fails: assumes PR not merged, just destroys
- Only deletes remote when we're confident PR was merged

**Reusing destroy_session**
Rather than duplicating the destroy logic, `complete_session` calls `destroy_session` internally. This ensures any future improvements to destroy also benefit complete.
