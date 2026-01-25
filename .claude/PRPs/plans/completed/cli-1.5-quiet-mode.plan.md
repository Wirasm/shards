# Implementation Plan: CLI Phase 1.5 - Quiet Mode (`-q`/`--quiet`)

**Source PRD**: `.claude/PRPs/prds/cli-core-features.prd.md`
**Phase**: 1.5
**Status**: READY FOR IMPLEMENTATION

---

## Summary

Add a global `-q`/`--quiet` flag to the CLI that suppresses log output, showing only essential information for clean scripted use. When quiet mode is enabled, the logging level is set to `error` only, suppressing `info`, `debug`, and `warn` level tracing events while preserving the user-facing `println!` output for success/failure messages.

## User Story

As a power user or script author, I want to suppress verbose logging output so that I can get clean, predictable output for scripting and automation.

## Problem Statement

Currently, every CLI command emits JSON-formatted tracing logs to stderr. While useful for debugging, this clutters output when:
- Piping commands to other tools
- Running from scripts
- Capturing output programmatically
- Using in CI/CD pipelines

## Solution Statement

Add a global `--quiet` (short: `-q`) flag that:
1. Is parsed before logging initialization
2. Configures the tracing subscriber to only emit `error` level events
3. Preserves user-facing `println!` output (success/failure messages)

## Metadata

| Field | Value |
|-------|-------|
| Type | ENHANCEMENT |
| Complexity | LOW |
| Systems Affected | shards (CLI), shards-core (logging) |
| Dependencies | None |
| Estimated Tasks | 6 |

---

## Mandatory Reading

**CRITICAL: Implementation agent MUST read these files before starting any task:**

| Priority | File | Lines | Why Read This |
|----------|------|-------|---------------|
| P0 | `crates/shards/src/main.rs` | 1-17 | Entry point, current logging init sequence |
| P0 | `crates/shards/src/app.rs` | 1-50 | CLI definition pattern with clap builder API |
| P0 | `crates/shards-core/src/logging/mod.rs` | 1-30 | Current logging setup |
| P1 | `crates/shards/src/commands.rs` | 52-135 | Command handler pattern (not affected, just understand flow) |

---

## Patterns to Mirror

**CLAP_GLOBAL_FLAG_PATTERN:**
```rust
// SOURCE: crates/shards/src/app.rs:58-63 (force flag on destroy - adapt for global)
.arg(
    Arg::new("quiet")
        .short('q')
        .long("quiet")
        .help("Suppress log output, show only essential information")
        .action(ArgAction::SetTrue)
        .global(true)
)
```

**LOGGING_INIT_PATTERN:**
```rust
// SOURCE: crates/shards-core/src/logging/mod.rs:3-16
pub fn init_logging() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(std::io::stderr)
                .with_current_span(false)
                .with_span_list(false),
        )
        .with(
            EnvFilter::from_default_env()
                .add_directive("shards=info".parse().expect("Invalid log directive")),
        )
        .init();
}
```

---

## Files to Change

| File | Action | Justification |
|------|--------|---------------|
| `crates/shards-core/src/logging/mod.rs` | UPDATE | Accept `quiet: bool` parameter, set level to error when true |
| `crates/shards/src/app.rs` | UPDATE | Add global `-q`/`--quiet` flag |
| `crates/shards/src/main.rs` | UPDATE | Parse quiet flag first, pass to init_logging |

---

## NOT Building (Scope Limits)

- **No verbose mode (`-v`)** - Future enhancement, not in this phase
- **No custom log level flag** - Use `RUST_LOG` env var for that
- **No config file setting** - YAGNI for now
- **No changes to user-facing println output** - Quiet suppresses logs, not results
- **No changes to command handlers** - They already use tracing correctly

---

## Step-by-Step Tasks

### Task 1: UPDATE logging/mod.rs to accept quiet parameter

- **ACTION**: Add `quiet: bool` parameter to `init_logging()`
- **FILE**: `crates/shards-core/src/logging/mod.rs`
- **IMPLEMENT**:
```rust
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize logging with optional quiet mode.
///
/// When `quiet` is true, only error-level events are emitted.
/// When `quiet` is false, info-level and above events are emitted (default).
pub fn init_logging(quiet: bool) {
    let directive = if quiet {
        "shards=error"
    } else {
        "shards=info"
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(std::io::stderr)
                .with_current_span(false)
                .with_span_list(false),
        )
        .with(
            EnvFilter::from_default_env()
                .add_directive(directive.parse().expect("Invalid log directive")),
        )
        .init();
}
```
- **VALIDATE**: `cargo check -p shards-core`

### Task 2: UPDATE lib.rs re-export (if needed)

- **ACTION**: Verify the re-export of `init_logging` still works with new signature
- **FILE**: `crates/shards-core/src/lib.rs`
- **NOTE**: The re-export `pub use logging::init_logging;` should still work as the function signature change is compatible
- **VALIDATE**: `cargo check -p shards-core`

### Task 3: ADD global quiet flag to CLI

- **ACTION**: Add `-q`/`--quiet` as a global flag on the main Command
- **FILE**: `crates/shards/src/app.rs`
- **LOCATION**: Add before `.subcommand_required(true)`
- **IMPLEMENT**:
```rust
pub fn build_cli() -> Command {
    Command::new("shards")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Manage parallel AI development agents in isolated Git worktrees")
        .long_about("Shards creates isolated git worktrees and launches AI coding agents in dedicated terminal windows. Each 'shard' is a disposable work context where an AI agent can operate autonomously without disrupting your main working directory.")
        .arg(
            Arg::new("quiet")
                .short('q')
                .long("quiet")
                .help("Suppress log output, show only essential information")
                .action(ArgAction::SetTrue)
                .global(true)
        )
        .subcommand_required(true)
        .arg_required_else_help(true)
        // ... rest of subcommands unchanged
```
- **KEY**: The `.global(true)` makes the flag available to all subcommands
- **VALIDATE**: `cargo check -p shards`

### Task 4: UPDATE main.rs to parse quiet flag before logging init

- **ACTION**: Parse CLI args, extract quiet flag, then init logging
- **FILE**: `crates/shards/src/main.rs`
- **IMPLEMENT**:
```rust
use shards_core::init_logging;

mod app;
mod commands;
mod table;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = app::build_cli();
    let matches = app.get_matches();

    // Extract quiet flag before initializing logging
    let quiet = matches.get_flag("quiet");
    init_logging(quiet);

    commands::run_command(&matches)?;

    Ok(())
}
```
- **KEY**: Parse args first, then init logging with the quiet value
- **VALIDATE**: `cargo check -p shards`

### Task 5: ADD tests for quiet flag parsing

- **ACTION**: Add tests to verify quiet flag is recognized
- **FILE**: `crates/shards/src/app.rs`
- **LOCATION**: In the tests module
- **IMPLEMENT**:
```rust
#[test]
fn test_cli_quiet_flag_short() {
    let app = build_cli();
    let matches = app.try_get_matches_from(vec!["shards", "-q", "list"]);
    assert!(matches.is_ok());

    let matches = matches.unwrap();
    assert!(matches.get_flag("quiet"));
}

#[test]
fn test_cli_quiet_flag_long() {
    let app = build_cli();
    let matches = app.try_get_matches_from(vec!["shards", "--quiet", "list"]);
    assert!(matches.is_ok());

    let matches = matches.unwrap();
    assert!(matches.get_flag("quiet"));
}

#[test]
fn test_cli_quiet_flag_with_subcommand_args() {
    let app = build_cli();
    // Quiet flag should work regardless of position (before or after subcommand)
    let matches = app.try_get_matches_from(vec!["shards", "-q", "create", "test-branch"]);
    assert!(matches.is_ok());

    let matches = matches.unwrap();
    assert!(matches.get_flag("quiet"));

    let create_matches = matches.subcommand_matches("create").unwrap();
    assert_eq!(create_matches.get_one::<String>("branch").unwrap(), "test-branch");
}
```
- **VALIDATE**: `cargo test -p shards test_cli_quiet`

### Task 6: UPDATE any other callers of init_logging

- **ACTION**: Search for and update any other places that call init_logging
- **FILES**: Potentially `crates/shards-ui/` if it has its own main.rs
- **CHECK**: `grep -r "init_logging" crates/`
- **NOTE**: If shards-ui calls init_logging, it should pass `false` to maintain current behavior
- **VALIDATE**: `cargo check --all`

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

**EXPECT**: All tests pass (including new quiet flag tests)

### Level 5: MANUAL_VALIDATION

```bash
# Verify quiet mode reduces output
shards list 2>&1 | wc -l      # Normal output (includes JSON logs)
shards -q list 2>&1 | wc -l   # Quiet output (should be less)

# Test quiet with create (shows the output difference clearly)
shards create test-quiet --agent claude
# Shows JSON logs for cli.create_started, cli.create_completed, etc.

shards -q create test-quiet2 --agent claude
# Shows only the success message, no info-level logs

# Test quiet flag position (should work before subcommand)
shards -q list
shards --quiet list

# Errors should still show (they use eprintln! and error! level)
shards -q destroy nonexistent-branch
# Should show: "Failed to destroy shard 'nonexistent-branch': ..."

# Cleanup
shards destroy --force test-quiet
shards destroy --force test-quiet2
```

---

## Acceptance Criteria

- [ ] `-q` and `--quiet` flags are recognized globally on CLI
- [ ] Quiet mode suppresses info/debug/warn level tracing events
- [ ] Quiet mode preserves user-facing println output (success/failure messages)
- [ ] Error level events still emit in quiet mode
- [ ] Quiet flag works regardless of position (before subcommand)
- [ ] All existing tests pass
- [ ] New tests for quiet flag parsing pass
- [ ] All validation commands pass with exit 0

---

## Completion Checklist

- [ ] Task 1: init_logging accepts quiet parameter
- [ ] Task 2: lib.rs re-export verified
- [ ] Task 3: Global quiet flag added to CLI
- [ ] Task 4: main.rs parses quiet flag before logging init
- [ ] Task 5: Tests added for quiet flag parsing
- [ ] Task 6: Other callers of init_logging updated
- [ ] Level 1-4 validation commands pass
- [ ] Manual testing completed

---

## Notes

- **Why error level not off**: Errors are critical; users need to know when something fails even in quiet mode
- **Why global flag**: Consistent with Unix conventions (e.g., `curl -s`, `git -q`)
- **Why before logging init**: Must configure subscriber before any tracing events fire
- **stdout vs stderr**: User output goes to stdout (println!), logs go to stderr (tracing). Quiet affects only stderr.
