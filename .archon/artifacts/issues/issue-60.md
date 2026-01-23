# Investigation: Ghostty terminal spawning fails due to $$ escaping in PID capture

**Issue**: #60 (https://github.com/Wirasm/shards/issues/60)
**Type**: BUG
**Investigated**: 2026-01-23T12:10:00Z

### Assessment

| Metric | Value | Reasoning |
|--------|-------|-----------|
| Severity | MEDIUM | Feature partially broken - only affects short-lived test commands, real agent usage (claude, kiro) works because process stays running and gets detected via fallback |
| Complexity | LOW | Single function fix in one file, isolated change with clear escaping pattern to follow |
| Confidence | HIGH | Clear root cause identified with evidence from error message showing `echo $` instead of `echo $$`, well-understood shell escaping mechanism |

---

## Problem Statement

When spawning a terminal with Ghostty, the `$$` (shell PID variable) in the PID capture wrapper is being interpreted by the outer shell layer before the inner shell can use it. This causes the PID file to be written with an empty or incorrect value, breaking PID-based process tracking for short-lived commands.

---

## Analysis

### Root Cause / Change Rationale

The command passes through multiple shell layers, and `$$` is not properly escaped to survive all layers.

### Evidence Chain

WHY: `echo $` appears in Ghostty error instead of `echo $$`
  Evidence: Error message shows `/usr/bin/login -flp rasmus sh -c ... && sh -c 'echo $ > ...`

↓ BECAUSE: The outer `sh -c` (from Ghostty/open command) interprets `$$` before the inner `sh -c` runs
  Evidence: Ghostty executes via `open -na Ghostty.app --args -e sh -c <ghostty_command>` at `ghostty.rs:61-68`

↓ BECAUSE: `wrap_command_with_pid_capture` doesn't escape `$$` for the additional shell layer that Ghostty adds
  Evidence: `pid_file.rs:165-168` - `format!("sh -c 'echo $$ > ...")`

↓ ROOT CAUSE: The function `wrap_command_with_pid_capture` produces a command with `$$` that works when directly executed but fails when wrapped in another `sh -c` layer
  Evidence: `pid_file.rs:165-168`:
  ```rust
  format!(
      "sh -c 'echo $$ > '\\''{}'\\'' && exec {}'",
      escaped_path, escaped_command
  )
  ```

### Affected Files

| File | Lines | Action | Description |
|------|-------|--------|-------------|
| `crates/shards-core/src/process/pid_file.rs` | 160-169 | UPDATE | Escape `$$` as `'$$'` to prevent outer shell interpretation |
| `crates/shards-core/src/process/pid_file.rs` | 283-299 | UPDATE | Update tests to verify proper escaping |

### Integration Points

- `terminal/handler.rs:156` calls `wrap_command_with_pid_capture` to wrap commands with PID capture
- The wrapped command is passed to `SpawnConfig` which reaches the Ghostty backend
- Ghostty backend at `ghostty.rs:61-68` passes the command to `open -na Ghostty.app --args -e sh -c`
- iTerm and Terminal.app backends use AppleScript and don't have this issue (different shell layering)

### Git History

- **Introduced**: d712a73b - 2026-01-22 - Function added as part of PID file tracking feature
- **Last modified**: d712a73b - same commit
- **Implication**: Original bug - the function was designed without accounting for Ghostty's additional shell layer

---

## Implementation Plan

### Step 1: Fix the $$ escaping in wrap_command_with_pid_capture

**File**: `crates/shards-core/src/process/pid_file.rs`
**Lines**: 165-168
**Action**: UPDATE

**Current code:**
```rust
// Line 165-168
    format!(
        "sh -c 'echo $$ > '\\''{}'\\'' && exec {}'",
        escaped_path, escaped_command
    )
```

**Required change:**
```rust
    // Use '$$' (single-quoted) to prevent outer shell from interpreting it.
    // The inner sh -c will see 'echo '\''$$'\'' > ...' which evaluates $$ correctly.
    format!(
        "sh -c 'echo '\\''$$'\\'' > '\\''{}'\\'' && exec {}'",
        escaped_path, escaped_command
    )
```

**Why**: Single-quoting `$$` as `'$$'` prevents the outer shell from interpreting it. The inner `sh -c` receives the literal `$$` and expands it correctly to its own PID. This follows the established pattern in the codebase for escaping single quotes: `'\''`.

The pattern breakdown:
- `'echo '` - start single-quoted string containing "echo "
- `\\''` - end quote, escape a literal quote, start new quote (produces `'`)
- `$$` - the literal $$ that inner shell will interpret
- `'\\''` - end quote, escape a literal quote, start new quote (produces `'`)
- ` > ...` - continue the command

---

### Step 2: Update the test to verify proper escaping

**File**: `crates/shards-core/src/process/pid_file.rs`
**Lines**: 283-299
**Action**: UPDATE

**Current test:**
```rust
// Line 283-290
#[test]
fn test_wrap_command_with_pid_capture() {
    let pid_file = Path::new("/tmp/test.pid");

    let wrapped = wrap_command_with_pid_capture("claude", pid_file);
    assert!(wrapped.contains("echo $$"));
    assert!(wrapped.contains("/tmp/test.pid"));
    assert!(wrapped.contains("exec claude"));
}
```

**Required change:**
```rust
#[test]
fn test_wrap_command_with_pid_capture() {
    let pid_file = Path::new("/tmp/test.pid");

    let wrapped = wrap_command_with_pid_capture("claude", pid_file);
    // $$ should be single-quoted to survive outer shell interpretation
    assert!(wrapped.contains("'$$'"), "expected single-quoted $$ for shell escaping: {}", wrapped);
    assert!(wrapped.contains("/tmp/test.pid"));
    assert!(wrapped.contains("exec claude"));
}
```

**Why**: The test should verify that `$$` is properly quoted to survive multiple shell layers.

---

### Step 3: Add test for shell layer survival

**File**: `crates/shards-core/src/process/pid_file.rs`
**Lines**: After line 299
**Action**: UPDATE (add new test)

**Test case to add:**
```rust
#[test]
fn test_wrap_command_survives_outer_shell() {
    // Verify the command can be wrapped in another sh -c and still work
    let pid_file = Path::new("/tmp/test.pid");
    let wrapped = wrap_command_with_pid_capture("echo test", pid_file);

    // Simulate what Ghostty does: wrap in another sh -c
    // The $$ should still be present after one level of shell interpretation
    let outer_wrapped = format!("sh -c '{}'", wrapped.replace('\'', "'\\''"));

    // After outer shell strips one layer of quoting, $$ should still be quoted
    // This is a structural test - actual execution would require integration testing
    assert!(wrapped.contains("'$$'"), "inner command should have quoted $$: {}", wrapped);
}
```

**Why**: This test documents the requirement that the command must survive being wrapped in an additional shell layer.

---

## Patterns to Follow

**From codebase - shell escaping pattern:**

```rust
// SOURCE: crates/shards-core/src/terminal/common/escape.rs:6-8
// Pattern for escaping single quotes in shell commands
pub fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\"'\"'"))
}
```

The codebase uses two patterns for single-quote escaping:
1. `'\"'\"'` - end quote, double-quoted single quote, start quote (in `shell_escape`)
2. `'\''` - end quote, escaped literal quote, start quote (in `wrap_command_with_pid_capture`)

Both are valid; the `'\''` pattern is already used in `wrap_command_with_pid_capture`, so we'll extend it.

---

## Edge Cases & Risks

| Risk/Edge Case | Mitigation |
|----------------|------------|
| Breaking iTerm/Terminal.app backends | They use AppleScript, not `sh -c` wrapping - verify they still work |
| Breaking commands with `$` in them | Commands are already escaped via `escaped_command` - `$` in command content is safe |
| Three or more shell layers | Unlikely in practice; current fix handles the known Ghostty double-layer case |

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

1. Create a shard with Ghostty and a short-lived command:
   ```bash
   shards create test-branch --terminal ghostty --startup-command "echo test && sleep 5"
   ```
   Verify the PID file is created with the correct PID.

2. Create a shard with Ghostty and a long-lived agent:
   ```bash
   shards create test-branch --terminal ghostty --agent claude
   ```
   Verify the session is created and process tracking works.

3. Run the e2e tests:
   ```bash
   cargo test --package shards-core -- --test-threads=1
   ```

---

## Scope Boundaries

**IN SCOPE:**
- Fixing `$$` escaping in `wrap_command_with_pid_capture`
- Updating existing tests
- Adding test for shell layer survival

**OUT OF SCOPE (do not touch):**
- Ghostty backend implementation (the fix is in the shared PID capture function)
- iTerm/Terminal.app backends (they don't have this shell layering issue)
- Other escaping functions in `escape.rs` (they handle different concerns)

---

## Metadata

- **Investigated by**: Claude
- **Timestamp**: 2026-01-23T12:10:00Z
- **Artifact**: `.archon/artifacts/issues/issue-60.md`
