# Feature: CLI `shards destroy --all` Command

## Summary

Add `--all` flag to the `destroy` command enabling bulk destruction of all shards for the current project. Unlike `open --all` and `stop --all`, this command includes a confirmation prompt (unless `--force` is specified) because destruction is a dangerous, irreversible operation that removes worktrees and can lose uncommitted work.

## User Story

As a power user managing multiple shards, I want to destroy all shards at once so that I can quickly clean up after a work session without running individual destroy commands for each shard.

## Problem Statement

Users finishing a work session must run individual `shards destroy <branch>` commands for each shard. This is tedious for humans and inefficient for orchestrating agents. A common workflow is "destroy all shards and start fresh" which currently requires multiple commands. Unlike stop/open operations, destroy is irreversible and can lose uncommitted work, so a confirmation step is needed.

## Solution Statement

Add `--all` flag to the `destroy` command that:
1. Lists all sessions for the current project
2. Prompts for confirmation (unless `--force` is specified)
3. Iterates through sessions and destroys each one using the existing `destroy_session()` function
4. Handles partial failures gracefully, continuing with remaining shards
5. Reports successes and failures with counts at the end

## Metadata

| Field | Value |
|-------|-------|
| Type | ENHANCEMENT |
| Complexity | LOW |
| Systems Affected | shards (CLI) |
| Dependencies | None - uses existing `destroy_session()` and `list_sessions()` |
| Estimated Tasks | 4 |

---

## Mandatory Reading

| Priority | File | Lines | Why Read This |
|----------|------|-------|---------------|
| P0 | `crates/shards/src/app.rs` | 79-95 | Current destroy command definition with `--force` flag pattern |
| P0 | `crates/shards/src/commands.rs` | 227-260 | Existing `handle_destroy_command()` implementation |
| P0 | `crates/shards/src/commands.rs` | 336-411 | `handle_open_all()` pattern to MIRROR for bulk operations |
| P0 | `crates/shards/src/commands.rs` | 446-516 | `handle_stop_all()` pattern to MIRROR for bulk operations |
| P1 | `crates/shards-core/src/sessions/handler.rs` | 237-367 | `destroy_session()` implementation (takes `name` and `force` params) |

---

## Patterns to Mirror

**BULK_OPERATION_PATTERN (from handle_stop_all):**
```rust
// SOURCE: crates/shards/src/commands.rs:446-516
/// Handle `shards stop --all` - stop all running shards
fn handle_stop_all() -> Result<(), Box<dyn std::error::Error>> {
    info!(event = "cli.stop_all_started");

    let sessions = session_handler::list_sessions()?;
    let active: Vec<_> = sessions
        .into_iter()
        .filter(|s| s.status == SessionStatus::Active)
        .collect();

    if active.is_empty() {
        println!("No running shards to stop.");
        info!(event = "cli.stop_all_completed", stopped = 0, failed = 0);
        return Ok(());
    }

    let mut stopped: Vec<String> = Vec::new();
    let mut errors: Vec<FailedOperation> = Vec::new();

    for session in active {
        match session_handler::stop_session(&session.branch) {
            Ok(()) => {
                info!(event = "cli.stop_completed", branch = session.branch);
                stopped.push(session.branch);
            }
            Err(e) => {
                error!(
                    event = "cli.stop_failed",
                    branch = session.branch,
                    error = %e
                );
                events::log_app_error(&e);
                errors.push((session.branch, e.to_string()));
            }
        }
    }

    // Report successes
    if !stopped.is_empty() {
        println!("Stopped {} shard(s):", stopped.len());
        for branch in &stopped {
            println!("   {}", branch);
        }
    }

    // Report failures
    if !errors.is_empty() {
        eprintln!("Failed to stop {} shard(s):", errors.len());
        for (branch, err) in &errors {
            eprintln!("   {}: {}", branch, err);
        }
    }

    info!(
        event = "cli.stop_all_completed",
        stopped = stopped.len(),
        failed = errors.len()
    );

    // Return error if any failures (for exit code)
    if !errors.is_empty() {
        let total_count = stopped.len() + errors.len();
        return Err(format!(
            "Partial failure: {} of {} shard(s) failed to stop",
            errors.len(),
            total_count
        )
        .into());
    }

    Ok(())
}
```

**CLI_FLAG_PATTERN (--all with conflicts_with):**
```rust
// SOURCE: crates/shards/src/app.rs:113-118 (from stop command)
.arg(
    Arg::new("all")
        .long("all")
        .help("Stop all running shards")
        .action(ArgAction::SetTrue)
        .conflicts_with("branch")
)
```

---

## Files to Change

| File | Action | Justification |
|------|--------|---------------|
| `crates/shards/src/app.rs` | UPDATE | Add `--all` flag to destroy command (lines 79-95) |
| `crates/shards/src/app.rs` | UPDATE | Add CLI tests for `--all` flag |
| `crates/shards/src/commands.rs` | UPDATE | Add `handle_destroy_all()` helper function |
| `crates/shards/src/commands.rs` | UPDATE | Update `handle_destroy_command()` to dispatch on `--all` flag |

---

## NOT Building (Scope Limits)

- **No `--json` output for bulk destroy** - Human-readable first; can add later
- **No `--dry-run` flag** - YAGNI; confirmation prompt serves similar purpose
- **No `--parallel` flag** - Sequential is simpler and safer for destructive operations
- **No filtering by status** - Destroy all shards regardless of status

---

## Step-by-Step Tasks

### Task 1: ADD `--all` flag to destroy command in app.rs

- **ACTION**: Add `--all` argument with `conflicts_with("branch")` and `required_unless_present`
- **FILE**: `crates/shards/src/app.rs`
- **LOCATION**: In `.subcommand(Command::new("destroy")...)` section (lines 79-95)
- **IMPLEMENT**:
```rust
.subcommand(
    Command::new("destroy")
        .about("Remove shard completely")
        .arg(
            Arg::new("branch")
                .help("Branch name of the shard to destroy")
                .index(1)
                .required_unless_present("all")  // CHANGE: was .required(true)
        )
        .arg(
            Arg::new("force")
                .long("force")
                .short('f')
                .help("Force destroy, bypassing git uncommitted changes check and confirmation prompt")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("all")
                .long("all")
                .help("Destroy all shards for current project")
                .action(ArgAction::SetTrue)
                .conflicts_with("branch")
        )
)
```
- **GOTCHA**: Change `branch` from `.required(true)` to `.required_unless_present("all")`
- **VALIDATE**: `cargo check -p shards && cargo run -- destroy --help`

### Task 2: ADD CLI tests for `--all` flag on destroy command

- **ACTION**: Add tests for new CLI argument combinations
- **FILE**: `crates/shards/src/app.rs`
- **LOCATION**: In `#[cfg(test)] mod tests` section
- **IMPLEMENT**:
```rust
#[test]
fn test_cli_destroy_all_flag() {
    let app = build_cli();
    let matches = app.try_get_matches_from(vec!["shards", "destroy", "--all"]);
    assert!(matches.is_ok());

    let matches = matches.unwrap();
    let destroy_matches = matches.subcommand_matches("destroy").unwrap();
    assert!(destroy_matches.get_flag("all"));
    assert!(destroy_matches.get_one::<String>("branch").is_none());
}

#[test]
fn test_cli_destroy_all_conflicts_with_branch() {
    let app = build_cli();
    let matches = app.try_get_matches_from(vec!["shards", "destroy", "--all", "some-branch"]);
    assert!(matches.is_err());
}

#[test]
fn test_cli_destroy_all_with_force() {
    let app = build_cli();
    let matches = app.try_get_matches_from(vec!["shards", "destroy", "--all", "--force"]);
    assert!(matches.is_ok());

    let matches = matches.unwrap();
    let destroy_matches = matches.subcommand_matches("destroy").unwrap();
    assert!(destroy_matches.get_flag("all"));
    assert!(destroy_matches.get_flag("force"));
}

#[test]
fn test_cli_destroy_requires_branch_or_all() {
    let app = build_cli();
    let matches = app.try_get_matches_from(vec!["shards", "destroy"]);
    assert!(matches.is_err());
}
```
- **VALIDATE**: `cargo test -p shards test_cli_destroy`

### Task 3: ADD `handle_destroy_all()` helper function

- **ACTION**: Add function to handle `destroy --all` bulk operation with confirmation prompt
- **FILE**: `crates/shards/src/commands.rs`
- **LOCATION**: After `handle_destroy_command()` (around line 260)
- **IMPLEMENT**:
```rust
/// Handle `shards destroy --all` - destroy all shards for current project
fn handle_destroy_all(force: bool) -> Result<(), Box<dyn std::error::Error>> {
    info!(event = "cli.destroy_all_started", force = force);

    let sessions = session_handler::list_sessions()?;

    if sessions.is_empty() {
        println!("No shards to destroy.");
        info!(event = "cli.destroy_all_completed", destroyed = 0, failed = 0);
        return Ok(());
    }

    // Confirmation prompt unless --force is specified
    if !force {
        use std::io::{self, Write};

        print!("Destroy ALL {} shard(s)? This cannot be undone. [y/N] ", sessions.len());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input = input.trim().to_lowercase();
        if input != "y" && input != "yes" {
            println!("Aborted.");
            info!(event = "cli.destroy_all_aborted");
            return Ok(());
        }
    }

    let mut destroyed: Vec<String> = Vec::new();
    let mut errors: Vec<FailedOperation> = Vec::new();

    for session in sessions {
        match session_handler::destroy_session(&session.branch, force) {
            Ok(()) => {
                info!(event = "cli.destroy_completed", branch = session.branch);
                destroyed.push(session.branch);
            }
            Err(e) => {
                error!(
                    event = "cli.destroy_failed",
                    branch = session.branch,
                    error = %e
                );
                events::log_app_error(&e);
                errors.push((session.branch, e.to_string()));
            }
        }
    }

    // Report successes
    if !destroyed.is_empty() {
        println!("Destroyed {} shard(s):", destroyed.len());
        for branch in &destroyed {
            println!("   {}", branch);
        }
    }

    // Report failures
    if !errors.is_empty() {
        eprintln!("Failed to destroy {} shard(s):", errors.len());
        for (branch, err) in &errors {
            eprintln!("   {}: {}", branch, err);
        }
    }

    info!(
        event = "cli.destroy_all_completed",
        destroyed = destroyed.len(),
        failed = errors.len()
    );

    // Return error if any failures (for exit code)
    if !errors.is_empty() {
        let total_count = destroyed.len() + errors.len();
        return Err(format!(
            "Partial failure: {} of {} shard(s) failed to destroy",
            errors.len(),
            total_count
        )
        .into());
    }

    Ok(())
}
```
- **MIRROR**: `handle_stop_all()` pattern at lines 446-516
- **KEY DIFFERENCE**: Added confirmation prompt using `std::io::stdin().read_line()`
- **VALIDATE**: `cargo check -p shards`

### Task 4: UPDATE `handle_destroy_command()` to dispatch on `--all` flag

- **ACTION**: Update `handle_destroy_command()` to check for `--all` flag first
- **FILE**: `crates/shards/src/commands.rs`
- **LOCATION**: Modify existing handler at lines 227-260
- **IMPLEMENT**:
```rust
fn handle_destroy_command(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let force = matches.get_flag("force");

    // Check for --all flag first
    if matches.get_flag("all") {
        return handle_destroy_all(force);
    }

    // Single branch operation (existing code)
    let branch = matches
        .get_one::<String>("branch")
        .ok_or("Branch argument is required (or use --all)")?;

    info!(
        event = "cli.destroy_started",
        branch = branch,
        force = force
    );

    match session_handler::destroy_session(branch, force) {
        Ok(()) => {
            println!("Shard '{}' destroyed successfully!", branch);
            info!(event = "cli.destroy_completed", branch = branch);
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to destroy shard '{}': {}", branch, e);
            error!(
                event = "cli.destroy_failed",
                branch = branch,
                error = %e
            );
            events::log_app_error(&e);
            Err(e.into())
        }
    }
}
```
- **GOTCHA**: Move `force` extraction before the `--all` check
- **VALIDATE**: `cargo check -p shards`

---

## Validation Commands

### Level 1: STATIC_ANALYSIS
```bash
cargo fmt --check && cargo clippy --all -- -D warnings
```

### Level 2: BUILD
```bash
cargo build --all
```

### Level 3: TESTS
```bash
cargo test --all
```

### Level 4: MANUAL_VALIDATION
```bash
# Setup: Create multiple test shards
cargo run -- create test-destroy-1 --note "Destroy test 1"
cargo run -- create test-destroy-2 --note "Destroy test 2"
cargo run -- create test-destroy-3 --note "Destroy test 3"

# Verify all created
cargo run -- list

# Test destroy --all with prompt (type 'n' to abort)
cargo run -- destroy --all
# Type 'n'
# Expected: "Aborted."

# Verify shards still exist
cargo run -- list

# Test destroy --all with confirmation (type 'y')
cargo run -- destroy --all
# Type 'y'

# Verify all destroyed
cargo run -- list

# Test edge case: destroy --all with no shards
cargo run -- destroy --all

# Test destroy --all --force (no prompt)
cargo run -- create test-force-1
cargo run -- create test-force-2
cargo run -- destroy --all --force

# Test conflict: --all with branch
cargo run -- destroy --all some-branch
```

---

## Acceptance Criteria

- [ ] `shards destroy --all` prompts for confirmation before destroying
- [ ] `shards destroy --all --force` skips confirmation and forces destruction
- [ ] Confirmation accepts 'y' or 'yes' (case-insensitive), rejects anything else
- [ ] Reports success/failure for each shard with counts
- [ ] Error in one shard doesn't stop destruction of others
- [ ] `--all` and branch argument conflict (clap error)
- [ ] "No shards to destroy" message when no sessions exist
- [ ] Exit code is non-zero when any operation fails
- [ ] All validation commands pass

---

## Completion Checklist

- [ ] Task 1: Added `--all` flag to destroy command in app.rs
- [ ] Task 2: Added CLI tests for `--all` flag
- [ ] Task 3: Added `handle_destroy_all()` helper function
- [ ] Task 4: Updated `handle_destroy_command()` to dispatch on `--all` flag
- [ ] Level 1-4 validation commands pass
- [ ] Manual testing completed
