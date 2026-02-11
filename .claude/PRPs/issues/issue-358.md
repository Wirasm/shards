# Investigation: complete should fail on branches with no PR instead of warning

**Issue**: #358 (https://github.com/Wirasm/kild/issues/358)
**Type**: BUG
**Investigated**: 2026-02-11T17:00:00Z

### Assessment

| Metric     | Value  | Reasoning                                                                                                              |
| ---------- | ------ | ---------------------------------------------------------------------------------------------------------------------- |
| Severity   | MEDIUM | Completing a never-pushed branch silently succeeds, blurring the semantic distinction between `complete` and `destroy`. The workaround is to use `destroy` instead, but users may not realize they used the wrong command. |
| Complexity | LOW    | 2 files changed (core handler + CLI), 1 file for new error variant. Isolated change with no integration risk.          |
| Confidence | HIGH   | Root cause is clear: `complete_session` treats "no PR" as a success variant, not an error. The fix path is straightforward. |

---

## Problem Statement

`kild complete` succeeds even when no PR exists and the branch was never pushed. The `complete` command semantically means "this work is done and merged." Completing a never-pushed branch with no PR is almost certainly a user mistake — they probably meant `destroy`. The command should error, not warn.

---

## Analysis

### Root Cause

WHY: `kild complete test` prints success (checkmark) even when no PR exists?
-> BECAUSE: `complete_session()` returns `Ok(CompleteResult::PrNotMerged)` when no PR is found, which is a success variant.
Evidence: `crates/kild-core/src/sessions/complete.rs:104-105`

```rust
} else if pr_merged == Some(false) {
    CompleteResult::PrNotMerged
```

WHY: `pr_merged` is `Some(false)` when no PR exists?
-> BECAUSE: `is_pr_merged()` returns `Ok(false)` when `gh pr view` reports "no pull requests found" — treating "no PR" as "not merged" rather than as an error.
Evidence: `crates/kild-core/src/forge/backends/github.rs:56-61`

```rust
if stderr.contains("no pull requests found")
    || stderr.contains("Could not resolve")
    || stderr.contains("no open pull requests")
{
    debug!(event = "core.forge.pr_merge_check_no_pr", branch = branch,);
    Ok(false)
```

ROOT CAUSE: `complete_session()` uses `is_pr_merged()` which conflates "no PR found" with "PR exists but not merged." It should use `check_pr_exists()` first (which returns `PrCheckResult::NotFound` vs `PrCheckResult::Exists`) and fail early when no PR is found.

### Affected Files

| File                                               | Lines   | Action | Description                                           |
| -------------------------------------------------- | ------- | ------ | ----------------------------------------------------- |
| `crates/kild-core/src/sessions/errors.rs`          | ~73     | UPDATE | Add `NoPrFound` error variant                         |
| `crates/kild-core/src/sessions/complete.rs`        | 47-108  | UPDATE | Add PR existence check before merge check; fail early |
| `crates/kild/src/commands/complete.rs`              | 37-64   | UPDATE | Remove pre-check warnings (core now handles it); update error display |

### Integration Points

- `crates/kild-core/src/state/dispatch.rs:72-74` — dispatches `Command::CompleteKild` to `complete_session()`. The `?` propagates errors. No change needed since new error type propagates naturally.
- `crates/kild-core/src/forge/backends/github.rs:76-124` — `check_pr_exists()` already returns `PrCheckResult` with `NotFound` variant. Already exists, just unused by complete.
- `crates/kild-core/src/sessions/types.rs:106-117` — `CompleteResult` enum. Remove `PrNotMerged` and `PrCheckUnavailable` variants since those cases now error.
- `crates/kild-core/src/forge/registry.rs` — `get_forge_backend()` returns `Option<Box<dyn ForgeBackend>>`. Used for PR checking.

### Git History

- **Introduced**: 9c53d24 — 2026-02-07 — "refactor: split sessions/handler.rs into focused modules" (original complete logic)
- **Last modified**: 959ebf8 — 2026-02-11 — "refactor: decompose git/operations.rs into focused modules" (minor branch naming change)
- **Implication**: Original design treated all PR states as non-fatal. This is a semantic bug, not a regression.

---

## Implementation Plan

### Step 1: Add `NoPrFound` error variant to `SessionError`

**File**: `crates/kild-core/src/sessions/errors.rs`
**Lines**: After line 73 (after `UncommittedChanges`)
**Action**: UPDATE

**Current code (line 70-73):**

```rust
#[error(
    "Cannot complete '{name}' with uncommitted changes. Use 'kild destroy --force' to remove."
)]
UncommittedChanges { name: String },
```

**Required change — add after `UncommittedChanges`:**

```rust
#[error(
    "Cannot complete '{name}': no PR found for this branch.\n   If the work landed, push the branch and create a PR first.\n   To remove the kild without completing, use 'kild destroy {name}'."
)]
NoPrFound { name: String },
```

Also add to `error_code()` match:

```rust
SessionError::NoPrFound { .. } => "SESSION_NO_PR_FOUND",
```

And add to `is_user_error()` match (it IS a user error):

```rust
| SessionError::NoPrFound { .. }
```

### Step 2: Simplify `CompleteResult` enum — remove unreachable variants

**File**: `crates/kild-core/src/sessions/types.rs`
**Lines**: 106-117
**Action**: UPDATE

**Current code:**

```rust
pub enum CompleteResult {
    /// PR was merged and remote branch was successfully deleted
    RemoteDeleted,
    /// PR was merged but remote branch deletion failed (logged as warning, non-fatal)
    RemoteDeleteFailed,
    /// PR was not merged, remote branch preserved for future merge
    PrNotMerged,
    /// Could not verify PR merge status (no forge, CLI error, no remote)
    PrCheckUnavailable,
}
```

**Required change:**

```rust
pub enum CompleteResult {
    /// PR was merged and remote branch was successfully deleted
    RemoteDeleted,
    /// PR was merged but remote branch deletion failed (logged as warning, non-fatal)
    RemoteDeleteFailed,
    /// PR exists but is not yet merged. Session destroyed, remote branch preserved.
    PrNotMerged,
}
```

Remove `PrCheckUnavailable` — when we can't check PR status (no forge, no remote), the complete command should still require a PR to exist. If the check is unavailable, we can't confirm the work landed, so we error. Keep `PrNotMerged` for the case where a PR EXISTS but hasn't been merged yet (user may want to complete the kild before merging the PR on GitHub).

**Wait — reconsider.** The issue says `complete` means "this work landed" — period. If the PR isn't merged, does complete still make sense? The existing doc comment on the complete command says:

```
- Complete first, then merge: kild complete → gh pr merge --delete-branch
- Merge first, then complete: gh pr merge → kild complete (deletes remote)
```

So the intended workflow supports completing BEFORE merging. The key requirement is that a PR must EXIST. The fix should:
- Error if no PR found (`PrCheckResult::NotFound`)
- Error if PR check unavailable and no remote (can't verify work landed)
- Allow completion if PR exists (merged or not)
- Allow completion if check is genuinely unavailable (forge CLI missing) but remote exists — degrade gracefully

Revised `CompleteResult`:

```rust
pub enum CompleteResult {
    /// PR was merged and remote branch was successfully deleted
    RemoteDeleted,
    /// PR was merged but remote branch deletion failed (logged as warning, non-fatal)
    RemoteDeleteFailed,
    /// PR exists but is not yet merged. Session destroyed, remote branch preserved.
    PrNotMerged,
    /// Could not verify PR status (forge CLI unavailable). Session destroyed, remote preserved.
    PrCheckUnavailable,
}
```

Keep all four variants. The difference is that `PrCheckUnavailable` is now only reachable when the forge CLI is genuinely unavailable (not when "no PR found").

### Step 3: Rewrite PR check logic in `complete_session` to fail on no PR

**File**: `crates/kild-core/src/sessions/complete.rs`
**Lines**: 47-108
**Action**: UPDATE

**Current code (lines 47-108):** Uses `is_pr_merged()` which conflates "no PR" with "not merged."

**Required change:** Replace the PR check section with a two-phase approach:

1. First check if PR exists using `check_pr_exists()` — fail early on `NotFound`
2. Then check if PR is merged using `is_pr_merged()` — determine cleanup behavior

```rust
// 2. Check PR existence — complete requires a PR to exist
// Skip PR check entirely for repos without a remote configured
let has_remote = super::destroy::has_remote_configured(&session.worktree_path);

if !has_remote {
    // No remote = branch was never pushed = no PR possible
    error!(
        event = "core.session.complete_no_pr",
        name = name,
        reason = "no_remote"
    );
    return Err(SessionError::NoPrFound {
        name: name.to_string(),
    });
}

let forge_backend = crate::forge::get_forge_backend(&session.worktree_path, forge_override);

// Check PR existence first (uses check_pr_exists for explicit NotFound detection)
if let Some(ref backend) = forge_backend {
    match backend.check_pr_exists(&session.worktree_path, &kild_branch) {
        PrCheckResult::Exists => {
            debug!(event = "core.session.complete_pr_exists", branch = name);
        }
        PrCheckResult::NotFound => {
            error!(
                event = "core.session.complete_no_pr",
                name = name,
                reason = "not_found"
            );
            return Err(SessionError::NoPrFound {
                name: name.to_string(),
            });
        }
        PrCheckResult::Unavailable => {
            // Forge CLI issue — can't confirm PR exists, proceed with warning
            warn!(
                event = "core.session.complete_pr_check_unavailable",
                branch = name,
                "Cannot verify PR status — proceeding anyway"
            );
        }
    }
}

// 3. Check if PR was merged (determines if we need to delete remote)
let pr_merged = match forge_backend {
    Some(backend) => match backend.is_pr_merged(&session.worktree_path, &kild_branch) {
        Ok(merged) => Some(merged),
        Err(e) => {
            warn!(
                event = "core.session.complete_pr_check_failed",
                branch = name,
                error = %e,
            );
            None
        }
    },
    None => {
        debug!(event = "core.session.complete_no_forge", branch = name);
        None
    }
};

info!(
    event = "core.session.complete_pr_status",
    branch = name,
    pr_merged = ?pr_merged
);

// 4. Determine the result based on PR status and remote deletion outcome
let result = if pr_merged == Some(true) {
    // PR was merged - attempt to delete remote branch
    match crate::git::cli::delete_remote_branch(&session.worktree_path, "origin", &kild_branch)
    {
        Ok(()) => {
            info!(
                event = "core.session.complete_remote_deleted",
                branch = kild_branch
            );
            CompleteResult::RemoteDeleted
        }
        Err(e) => {
            warn!(
                event = "core.session.complete_remote_delete_failed",
                branch = kild_branch,
                worktree_path = %session.worktree_path.display(),
                error = %e
            );
            CompleteResult::RemoteDeleteFailed
        }
    }
} else if pr_merged == Some(false) {
    CompleteResult::PrNotMerged
} else {
    CompleteResult::PrCheckUnavailable
};
```

Key changes:
- Add early return with `SessionError::NoPrFound` when no remote is configured
- Add early return with `SessionError::NoPrFound` when `check_pr_exists()` returns `NotFound`
- When `check_pr_exists()` returns `Unavailable`, proceed gracefully (forge CLI issue shouldn't block)
- Keep the existing `is_pr_merged()` flow for determining remote cleanup behavior
- Need to add `use crate::forge::types::PrCheckResult;` import

### Step 4: Simplify CLI pre-check — remove redundant warnings handled by core

**File**: `crates/kild/src/commands/complete.rs`
**Lines**: 24-64
**Action**: UPDATE

The CLI currently does its own pre-check safety info display including "No PR found" warnings. Since the core now errors on no-PR, the CLI pre-check is redundant for that case. However, the pre-check for uncommitted changes is still useful (it shows detailed file counts before the core blocks).

**Current code (lines 24-64):** Displays all safety warnings, blocks on uncommitted changes.

**Required change:** Keep the pre-check for uncommitted changes (better UX with details), but the "No PR found" warning from safety_info is now redundant since core will error. No code change needed here — the pre-check will still run, and if there ARE uncommitted changes, it blocks. If there aren't, it proceeds to `complete_session()` which will now error if no PR exists.

Actually, looking more carefully: the current flow runs `get_destroy_safety_info()` which shows the "No PR found" warning BEFORE calling `complete_session()`. With the fix, `complete_session()` will error on "no PR" and the CLI error handler will show the error message. The warning is still harmless (it just shows before the error), but for a cleaner UX, we could remove the pre-check entirely and let `complete_session()` handle everything. But the pre-check for uncommitted changes with detailed file counts is genuinely useful...

Best approach: keep the pre-check as-is. The flow will be:
1. Pre-check shows warnings (e.g., "No PR found", "Branch has never been pushed")
2. If uncommitted changes → block
3. Call `complete_session()` → errors with `NoPrFound` → CLI shows error message

This is slightly redundant (warning + error) but not confusing. The warning says "No PR found for this branch" and then the error says "Cannot complete: no PR found." Both are consistent.

Actually, on reflection this IS confusing — showing a warning (implying advisory) then immediately erroring. Better to let the core handle it. Remove the pre-check warnings for the no-PR case.

**Revised approach:** Keep the pre-check ONLY for the uncommitted changes block (since core also blocks but without the detailed file count display). Skip the complete call if blocked. For non-blocking warnings, skip showing them — let `complete_session()` either succeed or error.

**Simpler:** Just remove the entire pre-check section. The core's `complete_session()` already checks uncommitted changes (line 110-121) and will now also check PR existence. The CLI pre-check is defense-in-depth but makes the output redundant.

**Simplest correct approach:** Remove the pre-check. Let `complete_session()` be the single source of truth. The error messages from `SessionError::UncommittedChanges` and `SessionError::NoPrFound` are sufficient.

```rust
pub(crate) fn handle_complete_command(
    matches: &ArgMatches,
) -> Result<(), Box<dyn std::error::Error>> {
    let branch = matches
        .get_one::<String>("branch")
        .ok_or("Branch argument is required")?;

    if !is_valid_branch_name(branch) {
        eprintln!("Invalid branch name: {}", branch);
        error!(event = "cli.complete_invalid_branch", branch = branch);
        return Err("Invalid branch name".into());
    }

    info!(event = "cli.complete_started", branch = branch);

    match session_ops::complete_session(branch) {
        Ok(result) => {
            use kild_core::CompleteResult;

            println!("\u{2705} KILD '{}' completed!", branch);
            match result {
                CompleteResult::RemoteDeleted => {
                    println!("   Remote branch deleted (PR was merged)");
                }
                CompleteResult::RemoteDeleteFailed => {
                    println!("   Remote branch deletion failed (PR was merged, check logs)");
                }
                CompleteResult::PrNotMerged => {
                    println!("   Remote branch preserved (merge will delete it)");
                }
                CompleteResult::PrCheckUnavailable => {
                    println!(
                        "   Could not verify PR merge status \u{2014} remote branch preserved"
                    );
                }
            }

            info!(
                event = "cli.complete_completed",
                branch = branch,
                result = ?result
            );

            Ok(())
        }
        Err(e) => {
            eprintln!("\u{274c} {}", e);

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

Key changes:
- Remove the entire pre-check section (lines 24-64)
- Simplify error display: just print the error message (which now includes actionable guidance)
- Remove unused `warn` import

### Step 5: Add tests

**File**: `crates/kild-core/src/sessions/complete.rs`
**Action**: UPDATE — add tests to existing test module

```rust
#[test]
fn test_no_pr_found_error_display() {
    let error = SessionError::NoPrFound {
        name: "test".to_string(),
    };
    assert!(error.to_string().contains("no PR found"));
    assert!(error.to_string().contains("kild destroy test"));
    assert_eq!(error.error_code(), "SESSION_NO_PR_FOUND");
    assert!(error.is_user_error());
}
```

**File**: `crates/kild-core/src/sessions/errors.rs`
**Action**: UPDATE — add test for new error variant

```rust
#[test]
fn test_no_pr_found_error() {
    let error = SessionError::NoPrFound {
        name: "my-feature".to_string(),
    };
    assert!(error.to_string().contains("Cannot complete 'my-feature'"));
    assert!(error.to_string().contains("no PR found"));
    assert!(error.to_string().contains("kild destroy my-feature"));
    assert_eq!(error.error_code(), "SESSION_NO_PR_FOUND");
    assert!(error.is_user_error());
}
```

---

## Patterns to Follow

**From codebase — mirror the `UncommittedChanges` error pattern exactly:**

```rust
// SOURCE: crates/kild-core/src/sessions/errors.rs:70-73
#[error(
    "Cannot complete '{name}' with uncommitted changes. Use 'kild destroy --force' to remove."
)]
UncommittedChanges { name: String },
```

**From codebase — mirror the early-return error pattern in complete_session:**

```rust
// SOURCE: crates/kild-core/src/sessions/complete.rs:110-121
let safety_info = super::destroy::get_destroy_safety_info(name)?;
if safety_info.should_block() {
    error!(
        event = "core.session.complete_blocked",
        name = name,
        reason = "uncommitted_changes"
    );
    return Err(SessionError::UncommittedChanges {
        name: name.to_string(),
    });
}
```

**From codebase — `check_pr_exists()` is already used in destroy safety info:**

```rust
// SOURCE: crates/kild-core/src/sessions/destroy.rs:509-520
let pr_status = if has_remote_configured(&session.worktree_path) {
    crate::forge::get_forge_backend(&session.worktree_path, forge_override)
        .map(|backend| backend.check_pr_exists(&session.worktree_path, &kild_branch))
        .unwrap_or(PrCheckResult::Unavailable)
} else {
    PrCheckResult::Unavailable
};
```

---

## Edge Cases & Risks

| Risk/Edge Case                                | Mitigation                                                                                 |
| --------------------------------------------- | ------------------------------------------------------------------------------------------ |
| No forge CLI installed (gh not available)      | `check_pr_exists()` returns `Unavailable` — proceed with warning, don't block              |
| No remote configured (local-only repo)         | Error with `NoPrFound` — if there's no remote, there's no PR, so complete is wrong         |
| Network error during PR check                  | `check_pr_exists()` returns `Unavailable` — proceed with warning                           |
| PR exists but in closed (not merged) state     | `check_pr_exists()` returns `Exists` — allow completion (user may reopen)                  |
| UI dispatch path (`Command::CompleteKild`)     | Error propagates via `?` naturally — UI will receive the error event                       |

---

## Validation

### Automated Checks

```bash
cargo fmt --check
cargo clippy --all -- -D warnings
cargo test --all
cargo build --all
```

### Manual Verification

1. Create a kild with `--no-agent --no-daemon`, don't push, run `kild complete` — should error
2. Create a kild, push branch, create PR, run `kild complete` — should succeed
3. Create a kild, push branch (no PR), run `kild complete` — should error
4. Create a kild with uncommitted changes, run `kild complete` — should error (existing behavior)

---

## Scope Boundaries

**IN SCOPE:**

- Add `NoPrFound` error variant to `SessionError`
- Add PR existence check to `complete_session()` that errors on no PR
- Simplify CLI complete handler (remove redundant pre-check)
- Add tests for new error variant

**OUT OF SCOPE (do not touch):**

- `destroy` command behavior (unchanged — it uses warnings, not errors)
- `DestroySafetyInfo` / `should_block()` logic (unchanged)
- `is_pr_merged()` behavior in forge backends (unchanged)
- `--force` flag for complete (explicitly rejected in issue — that's what destroy is for)
- UI store dispatch (error propagates naturally)

---

## Metadata

- **Investigated by**: Claude
- **Timestamp**: 2026-02-11T17:00:00Z
- **Artifact**: `.claude/PRPs/issues/issue-358.md`
