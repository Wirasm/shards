# Investigation: Suppress JSON log lines and raw error types in non-verbose mode

**Issue**: #243 (https://github.com/Wirasm/kild/issues/243)
**Type**: BUG
**Investigated**: 2026-02-09

### Assessment

| Metric     | Value  | Reasoning                                                                                                    |
| ---------- | ------ | ------------------------------------------------------------------------------------------------------------ |
| Severity   | MEDIUM | User-facing output is cluttered with internal details on every error, but the correct message is still shown  |
| Complexity | LOW    | 4 files to change, all with small isolated edits; no architectural changes                                   |
| Confidence | HIGH   | Root cause is clearly identified in `init_logging` and `main()` return type; issue description is precise     |

---

## Problem Statement

In non-verbose mode, every error command dumps three separate representations to stderr: (1) the intended user-facing message, (2) JSON log lines from `error!()` tracing events, and (3) a raw Rust `Error: NotFound { ... }` debug representation from `main()` returning `Err`. Only the first line should be visible without `-v`.

---

## Analysis

### Root Cause

**WHY 1**: Why do JSON log lines appear in non-verbose mode?
Because `init_logging(quiet=true)` sets the directive to `"kild=error"`, which still emits ERROR-level JSON.
Evidence: `crates/kild-core/src/logging/mod.rs:8` - `let directive = if quiet { "kild=error" } else { "kild=info" };`

**WHY 2**: Why is `"kild=error"` the quiet default instead of OFF?
Because the original implementation assumed "quiet" meant "errors only" rather than "no JSON at all". The user-facing `eprintln!` messages already handle error display.
Evidence: Commit `1d84987` - "Flip logging default: quiet by default, --verbose to enable"

**WHY 3**: Why does `Error: NotFound { name: "foo" }` appear?
Because `main()` returns `Result<(), Box<dyn std::error::Error>>` and Rust's stdlib prints the `Debug` representation of any returned error.
Evidence: `crates/kild/src/main.rs:7` - `fn main() -> Result<(), Box<dyn std::error::Error>>`

**ROOT CAUSE**: Two independent issues:
1. `init_logging` sets quiet mode to ERROR level instead of OFF
2. `main()` propagates errors via `?` which triggers Rust's Debug error printing

### Affected Files

| File                                        | Lines | Action | Description                                          |
| ------------------------------------------- | ----- | ------ | ---------------------------------------------------- |
| `crates/kild-core/src/logging/mod.rs`       | 3-8   | UPDATE | Change quiet directive from `"kild=error"` to `"off"` |
| `crates/kild-peek-core/src/logging/mod.rs`  | 3-8   | UPDATE | Change quiet directive from `"kild_peek=error"` to `"off"` |
| `crates/kild/src/main.rs`                   | 7-18  | UPDATE | Handle error without propagating to Rust runtime     |
| `crates/kild-peek/src/main.rs`              | 7-18  | UPDATE | Handle error without propagating to Rust runtime     |
| `crates/kild/tests/cli_output.rs`           | 43-66, 95-122, 216-244 | UPDATE | Update tests to assert no JSON/Debug in error output |

### Integration Points

- `crates/kild-core/src/lib.rs` re-exports `init_logging` - no change needed, just the function body
- All command handlers (`crates/kild/src/commands/*.rs`) use `eprintln!` + `error!()` + `events::log_app_error()` + `Err(e.into())` - no changes needed, the logging fix handles suppression
- `events::log_app_error` (`crates/kild-core/src/events/mod.rs:14-20`) emits ERROR-level event - suppressed by the OFF directive

### Git History

- **Introduced**: `1d84987` - "Flip logging default: quiet by default, --verbose to enable (#133)"
- **Implication**: The original flip to quiet-by-default was correct in intent but set the wrong level (ERROR instead of OFF)

---

## Implementation Plan

### Step 1: Change kild-core logging quiet directive to OFF

**File**: `crates/kild-core/src/logging/mod.rs`
**Lines**: 3-8
**Action**: UPDATE

**Current code:**

```rust
/// Initialize logging with quiet mode control.
///
/// When `quiet` is true, only error-level events are emitted (default via CLI).
/// When `quiet` is false, info-level and above events are emitted (via -v/--verbose).
pub fn init_logging(quiet: bool) {
    let directive = if quiet { "kild=error" } else { "kild=info" };
```

**Required change:**

```rust
/// Initialize logging with quiet mode control.
///
/// When `quiet` is true, all log output is suppressed (default via CLI).
/// When `quiet` is false, info-level and above events are emitted (via -v/--verbose).
pub fn init_logging(quiet: bool) {
    let directive = if quiet { "kild=off" } else { "kild=info" };
```

**Why**: Setting the directive to `"off"` suppresses all JSON log output in non-verbose mode. The `eprintln!` user-facing messages are unaffected since they bypass tracing entirely.

---

### Step 2: Change kild-peek-core logging quiet directive to OFF

**File**: `crates/kild-peek-core/src/logging/mod.rs`
**Lines**: 3-8
**Action**: UPDATE

**Current code:**

```rust
/// Initialize logging with optional quiet mode.
///
/// When `quiet` is true, only error-level events are emitted.
/// When `quiet` is false, info-level and above events are emitted (default).
pub fn init_logging(quiet: bool) {
    let directive = if quiet {
        "kild_peek=error"
    } else {
        "kild_peek=info"
    };
```

**Required change:**

```rust
/// Initialize logging with optional quiet mode.
///
/// When `quiet` is true, all log output is suppressed.
/// When `quiet` is false, info-level and above events are emitted (default).
pub fn init_logging(quiet: bool) {
    let directive = if quiet {
        "kild_peek=off"
    } else {
        "kild_peek=info"
    };
```

**Why**: Same fix as Step 1, applied to the kild-peek crate.

---

### Step 3: Suppress Rust's Debug error output in kild main

**File**: `crates/kild/src/main.rs`
**Lines**: 7-18
**Action**: UPDATE

**Current code:**

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = app::build_cli();
    let matches = app.get_matches();

    let verbose = matches.get_flag("verbose");
    let quiet = !verbose;
    init_logging(quiet);

    commands::run_command(&matches)?;

    Ok(())
}
```

**Required change:**

```rust
fn main() {
    let app = app::build_cli();
    let matches = app.get_matches();

    let verbose = matches.get_flag("verbose");
    let quiet = !verbose;
    init_logging(quiet);

    if let Err(e) = commands::run_command(&matches) {
        // Error already printed to user via eprintln! in command handlers.
        // In verbose mode, JSON logs were also emitted.
        // Exit with non-zero code without printing Rust's Debug representation.
        drop(e);
        std::process::exit(1);
    }
}
```

**Why**: By not returning `Result` from `main()`, Rust's stdlib won't print the `Debug` representation. The command handlers already print user-friendly error messages via `eprintln!`. We `drop(e)` explicitly to show the error is intentionally discarded (the command handler already printed it). `std::process::exit(1)` sets a non-zero exit code for scripts/callers.

---

### Step 4: Suppress Rust's Debug error output in kild-peek main

**File**: `crates/kild-peek/src/main.rs`
**Lines**: 7-18
**Action**: UPDATE

**Current code:**

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = app::build_cli();
    let matches = app.get_matches();

    let verbose = matches.get_flag("verbose");
    let quiet = !verbose;
    init_logging(quiet);

    commands::run_command(&matches)?;

    Ok(())
}
```

**Required change:**

```rust
fn main() {
    let app = app::build_cli();
    let matches = app.get_matches();

    let verbose = matches.get_flag("verbose");
    let quiet = !verbose;
    init_logging(quiet);

    if let Err(e) = commands::run_command(&matches) {
        drop(e);
        std::process::exit(1);
    }
}
```

**Why**: Same fix as Step 3, applied to the kild-peek binary.

---

### Step 5: Update integration tests

**File**: `crates/kild/tests/cli_output.rs`
**Action**: UPDATE

**Test updates needed:**

1. **`test_list_stdout_is_clean`** (line 43-66): Update to assert stderr contains no JSON at all in quiet mode (not just no INFO).

2. **`test_default_mode_suppresses_info_logs`** (line 95-122): Add assertion that ERROR-level logs are also suppressed.

3. **`test_diff_nonexistent_branch_error`** (line 216-244): Add assertions that error output contains NO JSON log lines and NO `Error:` debug representation.

4. **`test_rust_log_overrides_default_quiet`** (line 249-272): Update comment and assertion since quiet mode is now OFF, not ERROR. `RUST_LOG=kild=debug` + quiet directive `kild=off` — the `add_directive("kild=off")` takes precedence.

**New test cases to add:**

```rust
/// Verify that error output in default mode contains ONLY the user-facing message
#[test]
fn test_error_output_is_clean_in_default_mode() {
    let output = Command::new(env!("CARGO_BIN_EXE_kild"))
        .args(["status", "nonexistent-branch-that-does-not-exist"])
        .output()
        .expect("Failed to execute 'kild status'");

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should contain the user-facing error message
    assert!(
        stderr.contains("❌"),
        "Should contain user-facing error indicator, got: {}",
        stderr
    );

    // Should NOT contain JSON log lines
    assert!(
        !stderr.contains(r#""level":"ERROR""#),
        "Default mode should suppress ERROR JSON logs, got: {}",
        stderr
    );

    // Should NOT contain raw Rust Debug representation
    assert!(
        !stderr.contains("Error: NotFound"),
        "Should not show Rust Debug error representation, got: {}",
        stderr
    );
}

/// Verify that verbose mode shows JSON logs on error
#[test]
fn test_error_output_verbose_shows_json() {
    let output = Command::new(env!("CARGO_BIN_EXE_kild"))
        .args(["-v", "status", "nonexistent-branch-that-does-not-exist"])
        .output()
        .expect("Failed to execute 'kild -v status'");

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should contain user-facing error
    assert!(stderr.contains("❌"));

    // Should contain JSON error logs in verbose mode
    assert!(
        stderr.contains(r#""level":"ERROR""#),
        "Verbose mode should show ERROR JSON logs, got: {}",
        stderr
    );
}
```

---

## Patterns to Follow

**From codebase - mirror the existing test helper pattern:**

```rust
// SOURCE: crates/kild/tests/cli_output.rs:8-22
fn run_kild_list() -> std::process::Output {
    let output = Command::new(env!("CARGO_BIN_EXE_kild"))
        .arg("list")
        .output()
        .expect("Failed to execute 'kild list'");
    // ...
    output
}
```

**From codebase - the `drop(e)` + `process::exit(1)` pattern is idiomatic for CLI tools that handle their own error display:**

This avoids the common Rust pitfall where `main() -> Result<...>` prints `Error: <Debug>` which is unhelpful for end users.

---

## Edge Cases & Risks

| Risk/Edge Case                          | Mitigation                                                                  |
| --------------------------------------- | --------------------------------------------------------------------------- |
| `RUST_LOG` env var override             | `add_directive("kild=off")` takes precedence over `RUST_LOG` in EnvFilter; test already covers this |
| Exit code changes                       | `std::process::exit(1)` preserves non-zero exit code for script consumers   |
| Destructors not running with `process::exit` | Only the error value needs cleanup; `drop(e)` is called before `exit`     |
| kild-peek not updated                   | Both binaries are updated in parallel (Steps 2 and 4)                       |

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

1. Run `cargo run -p kild -- status nonexistent-branch` — should show only `❌ Failed to get status...` line
2. Run `cargo run -p kild -- -v status nonexistent-branch` — should show `❌` line AND JSON log lines
3. Run `cargo run -p kild -- list` — stderr should be completely empty
4. Run `cargo run -p kild -- -v list` — stderr should contain INFO JSON logs

---

## Scope Boundaries

**IN SCOPE:**

- Change quiet log level from ERROR to OFF in both `init_logging` functions
- Change `main()` in both CLIs to not return `Result` (suppress Debug output)
- Update and add integration tests for clean error output

**OUT OF SCOPE (do not touch):**

- Command handler error patterns (`eprintln!` + `error!()` + `events::log_app_error`) — these remain unchanged
- The `events::log_app_error` helper — it still emits ERROR events, they're just suppressed in quiet mode
- Log format or structure
- Verbose mode behavior (should remain identical)

---

## Metadata

- **Investigated by**: Claude
- **Timestamp**: 2026-02-09
- **Artifact**: `.claude/PRPs/issues/issue-243.md`
