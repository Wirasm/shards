# Implementation Plan: CLI Phase 1.4 - JSON Output (`--json`)

**Source PRD**: `.claude/PRPs/prds/cli-core-features.prd.md`
**Phase**: 1.4
**Status**: READY FOR IMPLEMENTATION

---

## Summary

Add `--json` flag to `list` and `status` commands for machine-readable output. This enables scripting and automation workflows like `shards list --json | jq '.[] | select(.status == "Active") | .branch'`.

The implementation is straightforward because:
1. `serde_json` is already a dependency in the CLI crate
2. The `Session` struct already has `#[derive(Serialize, Deserialize)]`
3. The health command already implements the exact same pattern we need

## User Story

As a power user or automation script, I want to get shard information in JSON format so that I can pipe it to `jq`, parse it programmatically, or integrate with other tools.

## Problem Statement

The current CLI output is human-readable tables that are difficult to parse programmatically. Scripts and automation tools need structured data.

## Solution Statement

Add `--json` flag to `list` and `status` commands that:
1. Outputs valid JSON instead of formatted tables
2. Uses `serde_json::to_string_pretty()` for readable output
3. Follows the existing pattern from the `health` command

## Metadata

| Field | Value |
|-------|-------|
| Type | ENHANCEMENT |
| Complexity | LOW |
| Systems Affected | shards (CLI only) |
| Dependencies | None (serde_json already present) |
| Estimated Tasks | 6 |

---

## Mandatory Reading

**CRITICAL: Implementation agent MUST read these files before starting any task:**

| Priority | File | Lines | Why Read This |
|----------|------|-------|---------------|
| P0 | `crates/shards/src/commands.rs` | 449-467 | handle_health_command() - exact pattern to follow |
| P0 | `crates/shards/src/app.rs` | 163-168 | health command --json flag definition |
| P1 | `crates/shards/src/commands.rs` | 137-166 | handle_list_command() - to modify |
| P1 | `crates/shards/src/commands.rs` | 296-365 | handle_status_command() - to modify |
| P2 | `crates/shards-core/src/sessions/types.rs` | 21 | Session struct serde derives (already complete) |

---

## Patterns to Mirror

**CLAP_JSON_FLAG_PATTERN:**
```rust
// SOURCE: crates/shards/src/app.rs:164-168
.arg(
    Arg::new("json")
        .long("json")
        .help("Output in JSON format")
        .action(clap::ArgAction::SetTrue)
)
```

**JSON_OUTPUT_PATTERN:**
```rust
// SOURCE: crates/shards/src/commands.rs:449-467 (health command)
let json_output = matches.get_flag("json");
// ...
if json_output {
    println!("{}", serde_json::to_string_pretty(&health_output)?);
} else {
    print_health_table(&health_output);
}
```

---

## Files to Change

| File | Action | Justification |
|------|--------|---------------|
| `crates/shards/src/app.rs` | UPDATE | Add `--json` flag to `list` subcommand |
| `crates/shards/src/app.rs` | UPDATE | Add `--json` flag to `status` subcommand |
| `crates/shards/src/commands.rs` | UPDATE | Handle `--json` flag in `handle_list_command()` |
| `crates/shards/src/commands.rs` | UPDATE | Handle `--json` flag in `handle_status_command()` |

**Note**: No changes to shards-core needed - Session already has Serialize derive.

---

## NOT Building (Scope Limits)

- **No custom JSON schema** - Use Session struct directly (already has serde)
- **No `--json` on other commands** - PRD specifies only list and status
- **No envelope/metadata wrapper** - Keep it simple, just the data

---

## Step-by-Step Tasks

### Task 1: ADD `--json` flag to `list` command

- **ACTION**: Add json flag argument to the list subcommand
- **FILE**: `crates/shards/src/app.rs`
- **LOCATION**: Lines 45-48 (inside the `list` subcommand)
- **IMPLEMENT**:
```rust
.subcommand(
    Command::new("list")
        .about("List all shards for current project")
        .arg(
            Arg::new("json")
                .long("json")
                .help("Output in JSON format")
                .action(ArgAction::SetTrue)
        )
)
```
- **MIRROR**: Health command at lines 163-168
- **VALIDATE**: `cargo check -p shards`

### Task 2: ADD `--json` flag to `status` command

- **ACTION**: Add json flag argument to the status subcommand
- **FILE**: `crates/shards/src/app.rs`
- **LOCATION**: Lines 110-119 (inside the `status` subcommand)
- **IMPLEMENT**:
```rust
.subcommand(
    Command::new("status")
        .about("Show detailed status of a shard")
        .arg(
            Arg::new("branch")
                .help("Branch name of the shard to check")
                .required(true)
                .index(1)
        )
        .arg(
            Arg::new("json")
                .long("json")
                .help("Output in JSON format")
                .action(ArgAction::SetTrue)
        )
)
```
- **MIRROR**: Health command at lines 163-168
- **VALIDATE**: `cargo check -p shards`

### Task 3: UPDATE `handle_list_command()` to accept ArgMatches and handle json flag

- **ACTION**: Change function signature to receive matches, extract json flag
- **FILE**: `crates/shards/src/commands.rs`
- **LOCATION**: Lines 137-166
- **IMPLEMENT**:

First, update the match arm in `run_command()`:
```rust
Some(("list", sub_matches)) => handle_list_command(sub_matches),
```

Then update the handler:
```rust
fn handle_list_command(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let json_output = matches.get_flag("json");

    info!(event = "cli.list_started", json_output = json_output);

    match session_handler::list_sessions() {
        Ok(sessions) => {
            if json_output {
                println!("{}", serde_json::to_string_pretty(&sessions)?);
            } else {
                if sessions.is_empty() {
                    println!("No active shards found.");
                } else {
                    println!("Active shards:");
                    let formatter = crate::table::TableFormatter::new(&sessions);
                    formatter.print_table(&sessions);
                }
            }

            info!(event = "cli.list_completed", count = sessions.len());
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to list shards: {}", e);
            error!(event = "cli.list_failed", error = %e);
            events::log_app_error(&e);
            Err(e.into())
        }
    }
}
```
- **GOTCHA**: Update `run_command()` match arm from `Some(("list", _))` to `Some(("list", sub_matches))`
- **MIRROR**: `handle_health_command()` at lines 449-467
- **VALIDATE**: `cargo check -p shards`

### Task 4: UPDATE `handle_status_command()` to handle json flag

- **ACTION**: Extract json flag and conditionally output JSON
- **FILE**: `crates/shards/src/commands.rs`
- **LOCATION**: Lines 296-365
- **IMPLEMENT**:
```rust
fn handle_status_command(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let branch = matches
        .get_one::<String>("branch")
        .ok_or("Branch argument is required")?;
    let json_output = matches.get_flag("json");

    info!(event = "cli.status_started", branch = branch, json_output = json_output);

    match session_handler::get_session(branch) {
        Ok(session) => {
            if json_output {
                println!("{}", serde_json::to_string_pretty(&session)?);
            } else {
                // Existing table output code (lines 305-342)
                println!("Shard Status: {}", branch);
                println!("+-------------------------------------------------------------+");
                // ... rest of existing output
            }

            info!(
                event = "cli.status_completed",
                branch = branch,
                process_id = session.process_id
            );
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to get status for shard '{}': {}", branch, e);
            error!(event = "cli.status_failed", branch = branch, error = %e);
            events::log_app_error(&e);
            Err(e.into())
        }
    }
}
```
- **MIRROR**: `handle_health_command()` at lines 505-535
- **VALIDATE**: `cargo check -p shards`

### Task 5: ADD CLI test for list --json flag

- **ACTION**: Add test for `--json` flag on list command
- **FILE**: `crates/shards/src/app.rs`
- **LOCATION**: After `test_cli_list_command()` (around line 226)
- **IMPLEMENT**:
```rust
#[test]
fn test_cli_list_json_flag() {
    let app = build_cli();
    let matches = app.try_get_matches_from(vec!["shards", "list", "--json"]);
    assert!(matches.is_ok());

    let matches = matches.unwrap();
    let list_matches = matches.subcommand_matches("list").unwrap();
    assert!(list_matches.get_flag("json"));
}
```
- **VALIDATE**: `cargo test -p shards test_cli_list_json`

### Task 6: ADD CLI test for status --json flag

- **ACTION**: Add test for `--json` flag on status command
- **FILE**: `crates/shards/src/app.rs`
- **LOCATION**: After the new list json test
- **IMPLEMENT**:
```rust
#[test]
fn test_cli_status_json_flag() {
    let app = build_cli();
    let matches = app.try_get_matches_from(vec!["shards", "status", "test-branch", "--json"]);
    assert!(matches.is_ok());

    let matches = matches.unwrap();
    let status_matches = matches.subcommand_matches("status").unwrap();
    assert_eq!(status_matches.get_one::<String>("branch").unwrap(), "test-branch");
    assert!(status_matches.get_flag("json"));
}
```
- **VALIDATE**: `cargo test -p shards test_cli_status_json`

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
# Create a test shard
shards create test-json --agent claude

# Test list --json
shards list --json
# Expected: JSON array of sessions

# Test list --json with jq
shards list --json | jq '.[0].branch'
# Expected: First branch name as string

# Test status --json
shards status test-json --json
# Expected: JSON object with session details

# Test status --json with jq
shards status test-json --json | jq '.status'
# Expected: "Active" or "Stopped"

# Verify human output still works
shards list
# Expected: Table format

shards status test-json
# Expected: Box format

# Cleanup
shards destroy --force test-json

# Test empty list
shards list --json
# Expected: [] (empty array)
```

---

## Acceptance Criteria

- [ ] `shards list --json` outputs valid JSON array of sessions
- [ ] `shards status <branch> --json` outputs valid JSON object for single session
- [ ] JSON output can be piped to `jq` and parsed correctly
- [ ] Human-readable output remains unchanged when `--json` is not specified
- [ ] Empty list returns `[]` not an error
- [ ] All validation commands pass with exit 0
- [ ] Unit tests verify flag parsing

---

## Completion Checklist

- [ ] Task 1: `--json` flag added to list command definition
- [ ] Task 2: `--json` flag added to status command definition
- [ ] Task 3: `handle_list_command()` handles json flag
- [ ] Task 4: `handle_status_command()` handles json flag
- [ ] Task 5: Test added for list --json
- [ ] Task 6: Test added for status --json
- [ ] Level 1-4 validation commands pass
- [ ] Manual testing completed
