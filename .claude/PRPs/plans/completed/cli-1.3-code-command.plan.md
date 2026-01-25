# Implementation Plan: CLI Phase 1.3 - Open in Editor (`shards code`)

**Source PRD**: `.claude/PRPs/prds/cli-core-features.prd.md`
**Phase**: 1.3
**Status**: READY FOR IMPLEMENTATION

---

## Summary

Add a new `shards code <branch>` command that opens a shard's worktree directory in the user's preferred code editor. This is a simple, focused feature that:
- Opens the shard's worktree in an editor
- Uses `$EDITOR` environment variable, defaults to 'code' (VS Code)
- Optional `--editor` flag to override
- Fire-and-forget behavior (spawns editor and exits)

## User Story

As a power user, I want to quickly open a shard's worktree in my editor so that I can start working on code without manually navigating to the directory.

## Problem Statement

After creating a shard, users must manually find and open the worktree directory in their editor. This breaks the flow and requires remembering paths.

## Solution Statement

Add `shards code <branch>` command that:
1. Looks up the session by branch name
2. Determines editor: CLI flag > $EDITOR > "code" (VS Code default)
3. Spawns the editor with the worktree path
4. Reports success/failure

## Metadata

| Field | Value |
|-------|-------|
| Type | NEW FEATURE |
| Complexity | LOW |
| Systems Affected | shards (CLI only) |
| Dependencies | None |
| Estimated Tasks | 3 |

---

## Mandatory Reading

**CRITICAL: Implementation agent MUST read these files before starting any task:**

| Priority | File | Lines | Why Read This |
|----------|------|-------|---------------|
| P0 | `crates/shards/src/app.rs` | 48-92 | Command definition patterns |
| P0 | `crates/shards/src/commands.rs` | 296-365 | Handler pattern (status command) |
| P1 | `crates/shards-core/src/sessions/handler.rs` | 207-220 | get_session() function |
| P1 | `crates/shards-core/src/terminal/backends/ghostty.rs` | 61-70 | std::process::Command pattern |

---

## Patterns to Mirror

**CLI_COMMAND_WITH_OPTIONAL_ARG:**
```rust
// SOURCE: crates/shards/src/app.rs:69-78 (open command with --agent)
.subcommand(
    Command::new("open")
        .about("Open a new agent terminal in an existing shard")
        .arg(
            Arg::new("branch")
                .help("Branch name of the shard")
                .required(true)
                .index(1)
        )
        .arg(
            Arg::new("agent")
                .long("agent")
                .short('a')
                .help("Agent to use (overrides shard's default)")
        )
)
```

**PROCESS_SPAWN_PATTERN:**
```rust
// SOURCE: crates/shards-core/src/terminal/backends/ghostty.rs:61-70
let status = std::process::Command::new("open")
    .arg("-na")
    .arg("Ghostty.app")
    .spawn()?;
```

---

## Files to Change

| File | Action | Justification |
|------|--------|---------------|
| `crates/shards/src/app.rs` | UPDATE | Add `code` subcommand definition |
| `crates/shards/src/commands.rs` | UPDATE | Add `handle_code_command()` and wire into `run_command()` |

**Note**: No changes to shards-core needed - this is a pure CLI feature using `std::process::Command`.

---

## NOT Building (Scope Limits)

- **No editor validation** - Let the OS handle "command not found"
- **No config file option for editor** - YAGNI (PRD says $EDITOR + flag is sufficient)
- **No special handling for GUI vs TUI editors** - Fire-and-forget works for both
- **No shards-core changes** - This is a pure CLI feature

---

## Step-by-Step Tasks

### Task 1: ADD `code` subcommand to CLI definition

- **ACTION**: Add new subcommand after the `stop` command
- **FILE**: `crates/shards/src/app.rs`
- **LOCATION**: After line 92 (after `.subcommand(Command::new("stop")...`)
- **IMPLEMENT**:
```rust
.subcommand(
    Command::new("code")
        .about("Open shard's worktree in your code editor")
        .arg(
            Arg::new("branch")
                .help("Branch name of the shard to open")
                .required(true)
                .index(1)
        )
        .arg(
            Arg::new("editor")
                .long("editor")
                .short('e')
                .help("Editor to use (defaults to $EDITOR or 'code')")
        )
)
```
- **MIRROR**: Lines 69-78 (open command pattern)
- **VALIDATE**: `cargo check -p shards`

### Task 2: ADD command handler and wire into router

- **ACTION**: Add `handle_code_command()` function and match arm
- **FILE**: `crates/shards/src/commands.rs`
- **LOCATION**:
  1. Add match arm in `run_command()` (after line 64): `Some(("code", sub_matches)) => handle_code_command(sub_matches),`
  2. Add handler function after `handle_stop_command()`
- **IMPLEMENT**:
```rust
fn handle_code_command(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let branch = matches
        .get_one::<String>("branch")
        .ok_or("Branch argument is required")?;
    let editor_override = matches.get_one::<String>("editor").cloned();

    info!(
        event = "cli.code_started",
        branch = branch,
        editor_override = ?editor_override
    );

    // 1. Look up the session to get worktree path
    let session = match session_handler::get_session(branch) {
        Ok(session) => session,
        Err(e) => {
            eprintln!("Failed to find shard '{}': {}", branch, e);
            error!(event = "cli.code_failed", branch = branch, error = %e);
            events::log_app_error(&e);
            return Err(e.into());
        }
    };

    // 2. Determine editor: CLI flag > $EDITOR > "code"
    let editor = editor_override
        .or_else(|| std::env::var("EDITOR").ok())
        .unwrap_or_else(|| "code".to_string());

    info!(
        event = "cli.code_editor_selected",
        branch = branch,
        editor = editor
    );

    // 3. Spawn editor with worktree path
    match std::process::Command::new(&editor)
        .arg(&session.worktree_path)
        .spawn()
    {
        Ok(_) => {
            println!("Opening '{}' in {}", branch, editor);
            println!("   Path: {}", session.worktree_path.display());
            info!(
                event = "cli.code_completed",
                branch = branch,
                editor = editor,
                worktree_path = %session.worktree_path.display()
            );
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to open editor '{}': {}", editor, e);
            eprintln!("   Hint: Make sure '{}' is installed and in your PATH", editor);
            error!(
                event = "cli.code_failed",
                branch = branch,
                editor = editor,
                error = %e
            );
            Err(e.into())
        }
    }
}
```
- **MIRROR**: `handle_status_command()` at lines 296-365
- **VALIDATE**: `cargo check -p shards`

### Task 3: ADD CLI tests for code command

- **ACTION**: Add tests to verify CLI argument parsing
- **FILE**: `crates/shards/src/app.rs`
- **LOCATION**: In the `mod tests` block (after line 312)
- **IMPLEMENT**:
```rust
#[test]
fn test_cli_code_command() {
    let app = build_cli();
    let matches = app.try_get_matches_from(vec!["shards", "code", "test-branch"]);
    assert!(matches.is_ok());

    let matches = matches.unwrap();
    let code_matches = matches.subcommand_matches("code").unwrap();
    assert_eq!(
        code_matches.get_one::<String>("branch").unwrap(),
        "test-branch"
    );
}

#[test]
fn test_cli_code_command_with_editor() {
    let app = build_cli();
    let matches = app.try_get_matches_from(vec![
        "shards",
        "code",
        "test-branch",
        "--editor",
        "vim",
    ]);
    assert!(matches.is_ok());

    let matches = matches.unwrap();
    let code_matches = matches.subcommand_matches("code").unwrap();
    assert_eq!(
        code_matches.get_one::<String>("branch").unwrap(),
        "test-branch"
    );
    assert_eq!(
        code_matches.get_one::<String>("editor").unwrap(),
        "vim"
    );
}
```
- **VALIDATE**: `cargo test -p shards test_cli_code`

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

```bash
# Create a test shard first
shards create test-code --agent claude

# Test with default editor (VS Code)
shards code test-code
# Expected: VS Code opens with worktree

# Test with $EDITOR
EDITOR=vim shards code test-code
# Expected: vim opens with worktree

# Test with --editor flag
shards code test-code --editor nano
# Expected: nano opens with worktree

# Test error case - non-existent shard
shards code non-existent
# Expected: Error message "Failed to find shard 'non-existent'"

# Test error case - non-existent editor
shards code test-code --editor fake-editor-xyz
# Expected: Error message with hint about PATH

# Clean up
shards destroy --force test-code
```

---

## Acceptance Criteria

- [ ] `shards code <branch>` opens worktree in editor
- [ ] Uses `$EDITOR` environment variable when set
- [ ] Defaults to 'code' (VS Code) when `$EDITOR` not set
- [ ] `--editor` flag overrides both `$EDITOR` and default
- [ ] Clear error message when shard not found
- [ ] Clear error message when editor not found (with PATH hint)
- [ ] All validation commands pass with exit 0
- [ ] Unit tests for CLI argument parsing

---

## Completion Checklist

- [ ] Task 1: `code` subcommand added to CLI definition
- [ ] Task 2: Handler function implemented and wired into router
- [ ] Task 3: CLI tests added
- [ ] Level 1-4 validation commands pass
- [ ] Manual testing completed
