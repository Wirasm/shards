# Feature: CLI `shards diff` Command

## Summary

Implement a new `shards diff <branch>` command that shows git diff output for a shard's worktree without requiring the user to cd into the worktree directory. This enables quick inspection of uncommitted changes across multiple shards.

## User Story

As a power user managing multiple shards, I want to see git changes in a shard without cd-ing into it, so that I can quickly check work progress across shards.

## Problem Statement

From PRD section 2.2: Users managing multiple shards lack visibility into git activity. Currently, to see changes in a shard, users must:
1. Find the worktree path with `shards cd <branch>`
2. Navigate to that directory
3. Run `git diff`

This friction makes it difficult to monitor progress across multiple parallel development branches.

## Solution Statement

Add a `shards diff <branch>` command that:
1. Looks up the session by branch name to get the worktree path
2. Executes `git diff` (or `git diff --staged` with flag) in that worktree directory
3. Outputs the diff to stdout for the user to view

## Metadata

| Field | Value |
|-------|-------|
| Type | NEW_CAPABILITY |
| Complexity | LOW |
| Systems Affected | CLI (crates/shards) |
| Dependencies | None - uses existing session lookup and std::process::Command |
| Estimated Tasks | 2 |

---

## Mandatory Reading

| Priority | File | Lines | Why Read This |
|----------|------|-------|---------------|
| P0 | `crates/shards/src/commands.rs` | 591-643 | `handle_focus_command` is the EXACT pattern to mirror - simple session lookup + single operation |
| P0 | `crates/shards/src/app.rs` | 153-162 | `focus` command registration - matches the structure we need |
| P1 | `crates/shards-core/src/sessions/handler.rs` | 208-221 | `get_session` function for session lookup by branch name |
| P1 | `crates/shards-core/src/sessions/types.rs` | 16-88 | Session struct with `worktree_path: PathBuf` field |

---

## Patterns to Mirror

**COMMAND_REGISTRATION_PATTERN:**
```rust
// SOURCE: crates/shards/src/app.rs:153-162
// COPY THIS PATTERN for the diff command:
.subcommand(
    Command::new("focus")
        .about("Bring a shard's terminal window to the foreground")
        .arg(
            Arg::new("branch")
                .help("Branch name of the shard to focus")
                .required(true)
                .index(1)
        )
)
```

**COMMAND_HANDLER_PATTERN:**
```rust
// SOURCE: crates/shards/src/commands.rs:591-643
// MIRROR this pattern for handle_diff:
fn handle_focus_command(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let branch = matches
        .get_one::<String>("branch")
        .ok_or("Branch argument is required")?;

    info!(event = "cli.focus_started", branch = branch);

    // 1. Look up the session
    let session = match session_handler::get_session(branch) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("... Failed to find shard '{}': {}", branch, e);
            error!(event = "cli.focus_failed", branch = branch, error = %e);
            events::log_app_error(&e);
            return Err(e.into());
        }
    };

    // 2. Perform operation (for diff: run git diff)
    // 3. Report success/failure
}
```

---

## Files to Change

| File | Action | Justification |
|------|--------|---------------|
| `crates/shards/src/app.rs` | UPDATE | Add `diff` subcommand with `branch` positional arg and `--staged` flag |
| `crates/shards/src/commands.rs` | UPDATE | Add `handle_diff_command` function and wire it into `run_command` match |

---

## Step-by-Step Tasks

### Task 1: ADD diff command to app.rs

**ACTION**: Add Diff subcommand to CLI definition

**IMPLEMENT**:
```rust
// Add after the focus subcommand (around line 162)
.subcommand(
    Command::new("diff")
        .about("Show git diff for a shard's worktree")
        .arg(
            Arg::new("branch")
                .help("Branch name of the shard")
                .required(true)
                .index(1)
        )
        .arg(
            Arg::new("staged")
                .long("staged")
                .help("Show only staged changes (git diff --staged)")
                .action(ArgAction::SetTrue)
        )
)
```

**MIRROR**: Follow the exact structure of the `focus` command definition (lines 153-162)

**VALIDATE**:
```bash
cargo check -p shards
cargo run -- diff --help
```

### Task 2: IMPLEMENT handle_diff_command in commands.rs

**ACTION**: Add handle_diff_command function and wire it into run_command

**IMPLEMENT**:
```rust
// In run_command match statement (around line 71), add:
Some(("diff", sub_matches)) => handle_diff_command(sub_matches),

// New function:
fn handle_diff_command(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let branch = matches
        .get_one::<String>("branch")
        .ok_or("Branch argument is required")?;
    let staged = matches.get_flag("staged");

    info!(event = "cli.diff_started", branch = branch, staged = staged);

    // 1. Look up the session
    let session = match session_handler::get_session(branch) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to find shard '{}': {}", branch, e);
            error!(event = "cli.diff_failed", branch = branch, error = %e);
            events::log_app_error(&e);
            return Err(e.into());
        }
    };

    // 2. Build git diff command
    let mut cmd = std::process::Command::new("git");
    cmd.current_dir(&session.worktree_path);
    cmd.arg("diff");

    if staged {
        cmd.arg("--staged");
    }

    // 3. Execute and stream output to stdout
    let status = cmd.status()?;

    info!(
        event = "cli.diff_completed",
        branch = branch,
        staged = staged,
        exit_code = status.code()
    );

    if !status.success() {
        return Err(format!("git diff exited with status: {:?}", status.code()).into());
    }

    Ok(())
}
```

**MIRROR**: Follow pattern from `handle_focus_command` (lines 591-643)

**LOGGING**:
- `info!(event = "cli.diff_started", branch = branch, staged = staged)`
- `error!(event = "cli.diff_failed", branch = branch, error = %e)`
- `info!(event = "cli.diff_completed", branch = branch, staged = staged, exit_code = ...)`

**VALIDATE**:
```bash
cargo build -p shards
cargo run -- diff --help
```

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

### Level 3: UNIT_TESTS
```bash
cargo test --all
```

### Level 4: MANUAL_TEST
```bash
# Create a test shard
cargo run -- create test-diff --note "Testing diff command"

# Make a change in the worktree
echo "test change" >> "$(cargo run -- cd test-diff)/test-file.txt"

# Test diff command - should show the uncommitted change
cargo run -- diff test-diff

# Stage the change
cd "$(cargo run -- cd test-diff)" && git add test-file.txt && cd -

# Test staged flag
cargo run -- diff test-diff --staged

# Test error case - non-existent branch
cargo run -- diff nonexistent-branch

# Cleanup
cargo run -- destroy test-diff --force
```

---

## Acceptance Criteria

- [ ] `shards diff <branch>` shows uncommitted changes in the shard's worktree
- [ ] `shards diff <branch> --staged` shows only staged changes
- [ ] Error handling for non-existent branch returns user-friendly error
- [ ] Command follows existing CLI patterns (help text, logging)
- [ ] Passes `cargo fmt --check`
- [ ] Passes `cargo clippy --all -- -D warnings`
- [ ] All existing tests pass

---

## Completion Checklist

- [ ] Task 1: diff command added to app.rs
- [ ] Task 2: handle_diff_command implemented in commands.rs
- [ ] Level 1 validation passes (fmt + clippy)
- [ ] Level 2 validation passes (build)
- [ ] Level 3 validation passes (tests)
- [ ] Level 4 manual testing successful
