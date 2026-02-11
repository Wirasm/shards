# Investigation: kild-peek assert prints no output on failure

**Issue**: #354 (https://github.com/Wirasm/kild/issues/354)
**Type**: BUG
**Investigated**: 2026-02-11

### Assessment

| Metric     | Value  | Reasoning                                                                                               |
| ---------- | ------ | ------------------------------------------------------------------------------------------------------- |
| Severity   | MEDIUM | CLI exit code works for scripts, but violates "No Silent Failures" principle and breaks agent workflows |
| Complexity | LOW    | Single file change in CLI layer; core logic already returns proper failure results                       |
| Confidence | HIGH   | Root cause confirmed via code tracing; two distinct failure paths identified with evidence               |

---

## Problem Statement

`kild-peek assert --app "NonExistentApp123" --exists` exits with code 1 but prints nothing. Success prints "Assertion: PASS" with context. The asymmetry violates the "No Silent Failures" principle and prevents AI agents from parsing failure diagnostics.

---

## Analysis

### Root Cause

There are **two failure paths** that produce no output, caused by the same architectural issue: window resolution happens BEFORE assertion execution, and errors from window resolution are silently dropped in `main.rs`.

### Evidence Chain

**WHY:** Assert failure produces no output
↓ BECAUSE: Window resolution at line 1063-1067 fails and returns `Err` before `run_assertion()` is ever called
Evidence: `crates/kild-peek/src/commands.rs:1063-1067`
```rust
let resolved_title = if wait_flag {
    resolve_window_title_with_wait(app_name, window_title, timeout_ms)?
} else {
    resolve_window_title(app_name, window_title)?
};
```

↓ BECAUSE: The `?` operator propagates the error to `main()`, where it's dropped without printing
Evidence: `crates/kild-peek/src/main.rs:15-21`
```rust
if let Err(e) = commands::run_command(&matches) {
    // Error already printed to user via eprintln! in command handlers.
    // ...
    drop(e);
    std::process::exit(1);
}
```

↓ ROOT CAUSE: The assert handler treats window-not-found as an infrastructure error (propagated `Err`) instead of as an assertion failure result (printed output + exit code 1). The comment at main.rs:16 is incorrect — window resolution errors are NOT printed via `eprintln!` in the handler.

**Secondary path:** When `--window` is used without `--app` (no early lookup), the assertion DOES execute via `run_assertion()` and the core correctly returns `Ok(AssertionResult::fail(...))`. The CLI code at lines 1099-1104 would print "Assertion: FAIL" and call `std::process::exit(1)` at line 1111. However, `process::exit()` does not flush stdout buffers, so output may be lost when stdout is piped/redirected.

### Affected Files

| File                                | Lines     | Action | Description                                                     |
| ----------------------------------- | --------- | ------ | --------------------------------------------------------------- |
| `crates/kild-peek/src/commands.rs`  | 1049-1123 | UPDATE | Catch window resolution errors and format as assertion failures |
| `crates/kild-peek/tests/cli_output.rs` | END    | UPDATE | Add tests for assert failure output                             |

### Integration Points

- `crates/kild-peek/src/main.rs:15-21` — top-level error handler assumes errors were already printed (incorrect for assert)
- `crates/kild-peek-core/src/assert/handler.rs:47-92` — core `assert_window_exists` already produces proper failure messages with context; this data is just never reached
- `crates/kild-peek-core/src/assert/types.rs:102-138` — `AssertionResult` type already supports `fail()` with details

### Git History

- **Introduced**: `0514fc4` - "Add kild-peek CLI for native application inspection (#122)"
- **Implication**: Original bug — assert failure output was never implemented for the early window resolution path

---

## Implementation Plan

### Step 1: Handle window resolution errors as assertion failures in assert handler

**File**: `crates/kild-peek/src/commands.rs`
**Lines**: 1062-1067
**Action**: UPDATE

**Current code:**
```rust
// Resolve the window using app and/or title, with optional wait
let resolved_title = if wait_flag {
    resolve_window_title_with_wait(app_name, window_title, timeout_ms)?
} else {
    resolve_window_title(app_name, window_title)?
};
```

**Required change:**
```rust
// Resolve the window using app and/or title, with optional wait
let resolved_title = match if wait_flag {
    resolve_window_title_with_wait(app_name, window_title, timeout_ms)
} else {
    resolve_window_title(app_name, window_title)
} {
    Ok(title) => title,
    Err(e) => {
        // For --exists/--visible, window-not-found is an assertion failure, not an error.
        // Print the failure output so agents and scripts get diagnostic info.
        if exists_flag || visible_flag {
            if json_output {
                let result = kild_peek_core::assert::AssertionResult::fail(e.to_string());
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("Assertion: FAIL");
                println!("  {}", e);
            }
            info!(event = "peek.cli.assert_completed", passed = false);
            std::process::exit(1);
        }
        return Err(e);
    }
};
```

**Why**: Window-not-found during `--exists`/`--visible` IS the assertion failure — it should produce "Assertion: FAIL" output, not silently exit.

---

### Step 2: Flush stdout before process::exit(1)

**File**: `crates/kild-peek/src/commands.rs`
**Lines**: 1109-1112
**Action**: UPDATE

**Current code:**
```rust
// Exit with code 1 if assertion failed
if !result.passed {
    std::process::exit(1);
}
```

**Required change:**
```rust
// Exit with code 1 if assertion failed
if !result.passed {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    std::process::exit(1);
}
```

**Why**: `process::exit()` doesn't flush stdout. When output is piped (common for agents), block-buffered stdout may lose the "Assertion: FAIL" message. Also add flush before the new `process::exit(1)` in Step 1.

---

### Step 3: Add integration tests for assert failure output

**File**: `crates/kild-peek/tests/cli_output.rs`
**Action**: UPDATE (append tests)

**Test cases to add:**
```rust
// =============================================================================
// Assert Failure Output Tests
// =============================================================================

/// Verify that assert --exists prints failure message when app doesn't exist
#[test]
fn test_assert_exists_failure_prints_output() {
    let output = Command::new(env!("CARGO_BIN_EXE_kild-peek"))
        .args(["assert", "--app", "NonExistentApp99999", "--exists"])
        .output()
        .expect("Failed to execute 'kild-peek assert'");

    assert!(!output.status.success(), "Should exit with non-zero code");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Assertion: FAIL"),
        "Should print 'Assertion: FAIL', got stdout: {}",
        stdout
    );

    // Should contain diagnostic info about what wasn't found
    assert!(
        stdout.contains("NonExistentApp99999"),
        "Should mention the app name in failure output, got stdout: {}",
        stdout
    );
}

/// Verify that assert --exists --json prints JSON on failure
#[test]
fn test_assert_exists_failure_json_output() {
    let output = Command::new(env!("CARGO_BIN_EXE_kild-peek"))
        .args(["assert", "--app", "NonExistentApp99999", "--exists", "--json"])
        .output()
        .expect("Failed to execute 'kild-peek assert --json'");

    assert!(!output.status.success(), "Should exit with non-zero code");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should be valid JSON
    let result: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("Should output valid JSON, got '{}': {}", stdout, e));

    assert_eq!(result["passed"], false, "Should report passed: false");
    assert!(
        result["message"].as_str().unwrap_or("").contains("NonExistentApp99999"),
        "JSON message should mention app name, got: {}",
        result["message"]
    );
}

/// Verify that assert failure output does not contain JSON log noise
#[test]
fn test_assert_failure_output_is_clean() {
    let output = Command::new(env!("CARGO_BIN_EXE_kild-peek"))
        .args(["assert", "--app", "NonExistentApp99999", "--exists"])
        .output()
        .expect("Failed to execute 'kild-peek assert'");

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should NOT contain JSON log lines in default (quiet) mode
    assert!(
        !stderr.contains(r#""level":"ERROR""#),
        "Default mode should suppress ERROR JSON logs, got stderr: {}",
        stderr
    );
}
```

---

## Patterns to Follow

**From codebase — mirror the diff command's output pattern:**

```rust
// SOURCE: crates/kild-peek/src/commands.rs:357-395
// Pattern for status + details output with exit code
if json_output {
    println!("{}", serde_json::to_string_pretty(&result)?);
} else {
    let status = match result.is_similar() {
        true => "SIMILAR",
        false => "DIFFERENT",
    };
    println!("Image comparison: {}", status);
    println!("  Similarity: {}", result.similarity_percent());
    // ... more details ...
}
```

**From codebase — AssertionResult::fail() construction:**

```rust
// SOURCE: crates/kild-peek-core/src/assert/handler.rs:82-88
// Pattern for failure result with details
Ok(
    AssertionResult::fail(format!("Window '{}' not found", title)).with_details(
        serde_json::json!({
            "searched_title": title,
            "available_windows": available,
        }),
    ),
)
```

---

## Edge Cases & Risks

| Risk/Edge Case                          | Mitigation                                                                    |
| --------------------------------------- | ----------------------------------------------------------------------------- |
| `--similar` flag also uses window resolution | `--similar` has its own resolution in `build_similar_assertion_with_wait()`; the fix only intercepts `exists_flag \|\| visible_flag` |
| Piped stdout losing output before exit  | Explicit `stdout().flush()` before `process::exit(1)`                         |
| JSON output mode on failure             | Handled explicitly — construct `AssertionResult::fail()` and serialize        |
| diff command has same flush issue       | Out of scope — separate issue. Note in PR but don't fix                       |

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

1. `kild-peek assert --app "NonExistentApp99999" --exists` → should print "Assertion: FAIL" with app name
2. `kild-peek assert --app "NonExistentApp99999" --exists --json` → should print valid JSON with `passed: false`
3. `kild-peek assert --app Finder --exists` → should still print "Assertion: PASS" (no regression)
4. `kild-peek assert --app "NonExistentApp99999" --exists | cat` → piped output should still show failure (flush test)

---

## Scope Boundaries

**IN SCOPE:**

- Fix assert `--exists`/`--visible` failure output in CLI handler
- Flush stdout before `process::exit(1)` in assert handler
- Add integration tests for assert failure output

**OUT OF SCOPE (do not touch):**

- `kild-peek diff` command's identical flush issue (separate issue)
- Core `assert/handler.rs` logic (already correct — returns proper `AssertionResult::fail`)
- `--similar` assertion path (uses different resolution path)
- Window resolution logic in `resolve_window_title_impl` (working correctly)

---

## Metadata

- **Investigated by**: Claude
- **Timestamp**: 2026-02-11
- **Artifact**: `.claude/PRPs/issues/issue-354.md`
