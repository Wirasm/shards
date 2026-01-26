# Implementation Plan: CLI Phase 2.5 - Bulk Open/Stop (`--all`)

**Source PRD**: `.claude/PRPs/prds/cli-core-features.prd.md`
**Phase**: 2.5
**Status**: READY FOR IMPLEMENTATION

---

## Summary

Add `--all` flag to both `open` and `stop` commands enabling bulk operations on shards. The `open --all` command launches agents in all shards with status=Stopped. The `stop --all` command stops all agents in shards with status=Active. Both operations handle partial failures gracefully, continuing with remaining shards and reporting results with counts at the end.

## User Story

As a power user or orchestrating agent, I want to open or stop all shards at once so that I can efficiently manage parallel AI workflows without running individual commands for each shard.

## Problem Statement

Users managing multiple shards must run individual `shards open <branch>` or `shards stop <branch>` commands for each shard. This is tedious for humans and inefficient for orchestrating agents. A common workflow is "stop all agents for review" or "resume all agents" which currently requires multiple commands.

## Solution Statement

Add `--all` flag to both `open` and `stop` commands that:
1. Filters sessions by status (stopped for open, active for stop)
2. Iterates through matching sessions and performs the operation
3. Collects successes and failures separately
4. Continues on failure (partial failure handling)
5. Reports results with counts and details

## Metadata

| Field | Value |
|-------|-------|
| Type | ENHANCEMENT |
| Complexity | LOW |
| Systems Affected | shards (CLI) |
| Dependencies | None - uses existing `open_session()`, `stop_session()`, `list_sessions()` |
| Estimated Tasks | 6 |

---

## Mandatory Reading

**CRITICAL: Implementation agent MUST read these files before starting any task:**

| Priority | File | Lines | Why Read This |
|----------|------|-------|---------------|
| P0 | `crates/shards/src/app.rs` | 97-122 | Existing open and stop command definitions |
| P0 | `crates/shards/src/commands.rs` | 290-345 | `handle_open_command()` and `handle_stop_command()` patterns |
| P1 | `crates/shards-core/src/sessions/handler.rs` | 186-206 | `list_sessions()` function |
| P1 | `crates/shards-core/src/sessions/handler.rs` | 529-638 | `open_session()` implementation |
| P1 | `crates/shards-core/src/sessions/handler.rs` | 643-736 | `stop_session()` implementation |
| P1 | `crates/shards-core/src/sessions/types.rs` | 90-95 | SessionStatus enum definition |

---

## Patterns to Mirror

**CLI_ARG_FLAG_PATTERN (with SetTrue action):**
```rust
// SOURCE: crates/shards/src/app.rs:88-94
.arg(
    Arg::new("force")
        .long("force")
        .short('f')
        .help("Force destroy, bypassing git uncommitted changes check")
        .action(ArgAction::SetTrue)
)
```

**CLI_HANDLER_PATTERN:**
```rust
// SOURCE: crates/shards/src/commands.rs:290-319
fn handle_open_command(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let branch = matches
        .get_one::<String>("branch")
        .ok_or("Branch argument is required")?;
    let agent_override = matches.get_one::<String>("agent").cloned();

    info!(event = "cli.open_started", branch = branch, agent_override = ?agent_override);

    match session_handler::open_session(branch, agent_override) {
        Ok(session) => {
            println!("Opened new agent in shard '{}'", branch);
            println!("   Agent: {}", session.agent);
            if let Some(pid) = session.process_id {
                println!("   PID: {}", pid);
            }
            info!(
                event = "cli.open_completed",
                branch = branch,
                session_id = session.id
            );
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to open shard '{}': {}", branch, e);
            error!(event = "cli.open_failed", branch = branch, error = %e);
            events::log_app_error(&e);
            Err(e.into())
        }
    }
}
```

**SESSION_FILTERING_PATTERN:**
```rust
// SOURCE: crates/shards-core/src/sessions/types.rs:90-95
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SessionStatus {
    Active,
    Stopped,
    Destroyed,
}
```

---

## Files to Change

| File | Action | Justification |
|------|--------|---------------|
| `crates/shards/src/app.rs` | UPDATE | Add `--all` flag to open command (lines 97-112) |
| `crates/shards/src/app.rs` | UPDATE | Add `--all` flag to stop command (lines 113-122) |
| `crates/shards/src/commands.rs` | UPDATE | Add `handle_open_all()` helper function |
| `crates/shards/src/commands.rs` | UPDATE | Add `handle_stop_all()` helper function |
| `crates/shards/src/commands.rs` | UPDATE | Update `handle_open_command()` to check for `--all` flag |
| `crates/shards/src/commands.rs` | UPDATE | Update `handle_stop_command()` to check for `--all` flag |

---

## NOT Building (Scope Limits)

Explicit exclusions to prevent scope creep:

- **No `--json` output for bulk operations** - Can be added later if needed; human-readable first
- **No confirmation prompts** - Power users and agents need speed (per personas.md)
- **No `--agent` with `--all` for stop** - Stop doesn't take agent anyway
- **No progress indicators** - Operations are fast enough; simple output
- **No `--dry-run` flag** - YAGNI
- **No `--parallel` flag** - Sequential is fine, simpler error handling

---

## Step-by-Step Tasks

Execute in order. Each task is atomic and independently verifiable.

### Task 1: ADD `--all` flag to open command

- **ACTION**: Add `--all` argument with conflict_with for branch
- **FILE**: `crates/shards/src/app.rs`
- **LOCATION**: In `.subcommand(Command::new("open")...)` section (lines 97-112), after the `--agent` arg
- **IMPLEMENT**:
```rust
.subcommand(
    Command::new("open")
        .about("Open a new agent terminal in an existing shard (additive)")
        .arg(
            Arg::new("branch")
                .help("Branch name or shard identifier")
                .index(1)  // CHANGE: Remove .required(true)
        )
        .arg(
            Arg::new("agent")
                .long("agent")
                .short('a')
                .help("Agent to launch (default: shard's original agent)")
                .value_parser(["claude", "kiro", "gemini", "codex", "aether"])
        )
        .arg(
            Arg::new("all")
                .long("all")
                .help("Open agents in all stopped shards")
                .action(ArgAction::SetTrue)
                .conflicts_with("branch")
        )
)
```
- **MIRROR**: Arg definition pattern at lines 88-94 (`--force` flag)
- **GOTCHA**: Remove `.required(true)` from branch arg - either branch OR --all is required, not both
- **VALIDATE**: `cargo check -p shards && cargo run -- open --help`

### Task 2: ADD `--all` flag to stop command

- **ACTION**: Add `--all` argument with conflict_with for branch
- **FILE**: `crates/shards/src/app.rs`
- **LOCATION**: In `.subcommand(Command::new("stop")...)` section (lines 113-122), after branch arg
- **IMPLEMENT**:
```rust
.subcommand(
    Command::new("stop")
        .about("Stop agent(s) in a shard without destroying the worktree")
        .arg(
            Arg::new("branch")
                .help("Branch name or shard identifier")
                .index(1)  // CHANGE: Remove .required(true)
        )
        .arg(
            Arg::new("all")
                .long("all")
                .help("Stop all running shards")
                .action(ArgAction::SetTrue)
                .conflicts_with("branch")
        )
)
```
- **MIRROR**: Same pattern as Task 1
- **GOTCHA**: Remove `.required(true)` from branch arg
- **VALIDATE**: `cargo check -p shards && cargo run -- stop --help`

### Task 3: ADD CLI test for --all flags

- **ACTION**: Add tests for new CLI argument combinations
- **FILE**: `crates/shards/src/app.rs`
- **LOCATION**: In `#[cfg(test)] mod tests` section (after line 550)
- **IMPLEMENT**:
```rust
#[test]
fn test_cli_open_all_flag() {
    let app = build_cli();
    let matches = app.try_get_matches_from(vec!["shards", "open", "--all"]);
    assert!(matches.is_ok());

    let matches = matches.unwrap();
    let open_matches = matches.subcommand_matches("open").unwrap();
    assert!(open_matches.get_flag("all"));
    assert!(open_matches.get_one::<String>("branch").is_none());
}

#[test]
fn test_cli_open_all_conflicts_with_branch() {
    let app = build_cli();
    // --all and branch should conflict
    let matches = app.try_get_matches_from(vec!["shards", "open", "--all", "some-branch"]);
    assert!(matches.is_err());
}

#[test]
fn test_cli_open_all_with_agent() {
    let app = build_cli();
    let matches = app.try_get_matches_from(vec!["shards", "open", "--all", "--agent", "claude"]);
    assert!(matches.is_ok());

    let matches = matches.unwrap();
    let open_matches = matches.subcommand_matches("open").unwrap();
    assert!(open_matches.get_flag("all"));
    assert_eq!(open_matches.get_one::<String>("agent").unwrap(), "claude");
}

#[test]
fn test_cli_stop_all_flag() {
    let app = build_cli();
    let matches = app.try_get_matches_from(vec!["shards", "stop", "--all"]);
    assert!(matches.is_ok());

    let matches = matches.unwrap();
    let stop_matches = matches.subcommand_matches("stop").unwrap();
    assert!(stop_matches.get_flag("all"));
}

#[test]
fn test_cli_stop_all_conflicts_with_branch() {
    let app = build_cli();
    let matches = app.try_get_matches_from(vec!["shards", "stop", "--all", "some-branch"]);
    assert!(matches.is_err());
}
```
- **VALIDATE**: `cargo test -p shards test_cli_open_all test_cli_stop_all`

### Task 4: ADD `handle_open_all()` helper function

- **ACTION**: Add function to handle `open --all` bulk operation
- **FILE**: `crates/shards/src/commands.rs`
- **LOCATION**: After `handle_open_command()` (around line 320)
- **IMPLEMENT**:
```rust
/// Handle `shards open --all` - open agents in all stopped shards
fn handle_open_all(agent_override: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    info!(event = "cli.open_all_started", agent_override = ?agent_override);

    let sessions = session_handler::list_sessions()?;
    let stopped: Vec<_> = sessions
        .iter()
        .filter(|s| s.status == shards_core::SessionStatus::Stopped)
        .collect();

    if stopped.is_empty() {
        println!("No stopped shards to open.");
        info!(event = "cli.open_all_completed", opened = 0, failed = 0);
        return Ok(());
    }

    let mut opened: Vec<(String, String)> = Vec::new(); // (branch, agent)
    let mut errors: Vec<(String, String)> = Vec::new(); // (branch, error_message)

    for session in stopped {
        match session_handler::open_session(&session.branch, agent_override.clone()) {
            Ok(s) => {
                opened.push((s.branch.clone(), s.agent.clone()));
            }
            Err(e) => {
                errors.push((session.branch.clone(), e.to_string()));
            }
        }
    }

    // Report successes
    if !opened.is_empty() {
        println!("Opened {} shards:", opened.len());
        for (branch, agent) in &opened {
            println!("   {} ({})", branch, agent);
        }
    }

    // Report failures
    if !errors.is_empty() {
        eprintln!("Failed to open {} shards:", errors.len());
        for (branch, err) in &errors {
            eprintln!("   {}: {}", branch, err);
        }
    }

    info!(
        event = "cli.open_all_completed",
        opened = opened.len(),
        failed = errors.len()
    );

    // Return error if any failures (for exit code)
    if !errors.is_empty() {
        return Err(format!("Failed to open {} shard(s)", errors.len()).into());
    }

    Ok(())
}
```
- **MIRROR**: Error handling pattern from `handle_open_command()` at lines 290-319
- **IMPORTS**: Need `shards_core::SessionStatus` - add to imports at top of file if not present
- **GOTCHA**: Use `session_handler::list_sessions()` not `shards_core::list_sessions()`
- **VALIDATE**: `cargo check -p shards`

### Task 5: ADD `handle_stop_all()` helper function

- **ACTION**: Add function to handle `stop --all` bulk operation
- **FILE**: `crates/shards/src/commands.rs`
- **LOCATION**: After `handle_stop_command()` (around line 345)
- **IMPLEMENT**:
```rust
/// Handle `shards stop --all` - stop all running shards
fn handle_stop_all() -> Result<(), Box<dyn std::error::Error>> {
    info!(event = "cli.stop_all_started");

    let sessions = session_handler::list_sessions()?;
    let active: Vec<_> = sessions
        .iter()
        .filter(|s| s.status == shards_core::SessionStatus::Active)
        .collect();

    if active.is_empty() {
        println!("No running shards to stop.");
        info!(event = "cli.stop_all_completed", stopped = 0, failed = 0);
        return Ok(());
    }

    let mut stopped: Vec<String> = Vec::new(); // branch names
    let mut errors: Vec<(String, String)> = Vec::new(); // (branch, error_message)

    for session in active {
        match session_handler::stop_session(&session.branch) {
            Ok(()) => {
                stopped.push(session.branch.clone());
            }
            Err(e) => {
                errors.push((session.branch.clone(), e.to_string()));
            }
        }
    }

    // Report successes
    if !stopped.is_empty() {
        println!("Stopped {} shards:", stopped.len());
        for branch in &stopped {
            println!("   {}", branch);
        }
    }

    // Report failures
    if !errors.is_empty() {
        eprintln!("Failed to stop {} shards:", errors.len());
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
        return Err(format!("Failed to stop {} shard(s)", errors.len()).into());
    }

    Ok(())
}
```
- **MIRROR**: Same pattern as `handle_open_all()`
- **VALIDATE**: `cargo check -p shards`

### Task 6: UPDATE command handlers to dispatch on --all flag

- **ACTION**: Update `handle_open_command()` and `handle_stop_command()` to check for `--all`
- **FILE**: `crates/shards/src/commands.rs`
- **LOCATION**: Modify existing handlers at lines 290 and 321
- **IMPLEMENT for handle_open_command()**:
```rust
fn handle_open_command(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    // Check for --all flag first
    if matches.get_flag("all") {
        let agent_override = matches.get_one::<String>("agent").cloned();
        return handle_open_all(agent_override);
    }

    // Single branch operation (existing code)
    let branch = matches
        .get_one::<String>("branch")
        .ok_or("Branch argument is required (or use --all)")?;
    let agent_override = matches.get_one::<String>("agent").cloned();

    // ... rest of existing code unchanged ...
}
```
- **IMPLEMENT for handle_stop_command()**:
```rust
fn handle_stop_command(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    // Check for --all flag first
    if matches.get_flag("all") {
        return handle_stop_all();
    }

    // Single branch operation (existing code)
    let branch = matches
        .get_one::<String>("branch")
        .ok_or("Branch argument is required (or use --all)")?;

    // ... rest of existing code unchanged ...
}
```
- **GOTCHA**: Update error message to hint about `--all` when branch missing
- **VALIDATE**: `cargo check -p shards`

---

## Testing Strategy

### Manual CLI Tests

```bash
# Setup: Create some test shards
shards create test-bulk-1 --note "Bulk test 1"
shards create test-bulk-2 --note "Bulk test 2"
shards create test-bulk-3 --note "Bulk test 3"

# Verify all are active
shards list

# Test stop --all
shards stop --all
# Expected: Stopped 3 shards: test-bulk-1, test-bulk-2, test-bulk-3

# Verify all are stopped
shards list

# Test open --all
shards open --all
# Expected: Opened 3 shards: test-bulk-1 (claude), test-bulk-2 (claude), test-bulk-3 (claude)

# Test open --all with --agent override
shards stop --all
shards open --all --agent kiro

# Test edge case: open --all with no stopped shards
shards open --all
# Expected: "No stopped shards to open."

# Test edge case: stop --all with no active shards
shards stop --all
shards stop --all
# Expected: "No running shards to stop."

# Test conflict: --all with branch
shards open --all test-bulk-1
# Expected: Error from clap about conflicting arguments

# Cleanup
shards destroy --force test-bulk-1
shards destroy --force test-bulk-2
shards destroy --force test-bulk-3
```

---

## Validation Commands

### Level 1: STATIC_ANALYSIS

```bash
cargo fmt --check && cargo clippy --all -- -D warnings
```

**EXPECT**: Exit 0, no errors or warnings

### Level 2: TYPE_CHECK

```bash
cargo check --all
```

**EXPECT**: Exit 0, no type errors

### Level 3: BUILD

```bash
cargo build --all
```

**EXPECT**: Exit 0, clean build

### Level 4: TESTS

```bash
cargo test --all
```

**EXPECT**: All tests pass

### Level 5: MANUAL_VALIDATION

See Testing Strategy above for manual test steps.

---

## Acceptance Criteria

- [ ] `shards open --all` launches agents in all stopped shards
- [ ] `shards stop --all` stops all running shards
- [ ] `shards open --all --agent <agent>` uses specified agent for all
- [ ] `--all` and branch argument conflict (clap error)
- [ ] Partial failures are handled gracefully (continues with remaining)
- [ ] Output shows count and list of affected shards
- [ ] Edge case: "No stopped/running shards" message when applicable
- [ ] Exit code is non-zero when any operation fails
- [ ] All validation commands (fmt, clippy, check, test, build) pass

---

## Completion Checklist

- [ ] Task 1: Added `--all` flag to open command
- [ ] Task 2: Added `--all` flag to stop command
- [ ] Task 3: Added CLI tests for new flags
- [ ] Task 4: Added `handle_open_all()` helper function
- [ ] Task 5: Added `handle_stop_all()` helper function
- [ ] Task 6: Updated handlers to dispatch on --all flag
- [ ] Level 1-4 validation commands pass
- [ ] Manual testing completed
