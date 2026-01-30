# Feature: Smart Waiting for Interaction & Element Commands

## Summary

Add `--wait` and `--timeout` flags to all interaction commands (`click`, `type`, `key`) and element commands (`elements`, `find`) so they can poll for a window to appear before acting. This extends the existing wait pattern from screenshot/assert to the remaining kild-peek commands, enabling reliable E2E test scripts that launch an app and immediately interact with it.

## User Story

As a developer writing E2E test scripts for native macOS applications
I want interaction and element commands to wait for windows to appear
So that my scripts don't fail due to timing issues when apps are still launching

## Problem Statement

Currently, `click`, `type`, `key`, `elements`, and `find` commands fail immediately with `WindowNotFound` if the target window hasn't appeared yet. Scripts that launch an app and then interact with it require manual `sleep` commands or retry loops. The screenshot and assert commands already have `--wait` support via `poll_until_found()`, but this pattern hasn't been extended to the interaction and element commands.

## Solution Statement

Extend the existing `poll_until_found()` wait mechanism to all interaction and element commands by:
1. Adding `timeout_ms: Option<u64>` to all core request types
2. Adding `find_window_by_target_with_optional_wait()` helpers that dispatch to wait/non-wait window finders
3. Adding wait timeout error variants to `InteractionError` and `ElementError`
4. Adding `--wait` and `--timeout` CLI args to 5 commands
5. Wiring the CLI args through to core handlers

## Metadata

| Field            | Value                                                    |
| ---------------- | -------------------------------------------------------- |
| Type             | ENHANCEMENT                                              |
| Complexity       | MEDIUM                                                   |
| Systems Affected | kild-peek-core (interact, element), kild-peek (CLI)      |
| Dependencies     | None (uses existing poll_until_found from window module) |
| Estimated Tasks  | 8                                                        |

---

## UX Design

### Before State

```
                    BEFORE: Interaction without --wait

  ┌──────────────┐     ┌──────────────────┐     ┌─────────────────┐
  │ cargo run    │     │ kild-peek click  │     │  ERROR:         │
  │ -p kild-ui & │────►│ --app KILD       │────►│  Window not     │
  │              │     │ --text "Create"  │     │  found: 'KILD'  │
  └──────────────┘     └──────────────────┘     └─────────────────┘
                        App not ready yet         Script fails

  USER_FLOW: Launch app → immediately run click → fail
  PAIN_POINT: No way to wait for window; user must add manual sleep
  DATA_FLOW: CLI → find_window_by_target → WindowNotFound → error
```

### After State

```
                     AFTER: Interaction with --wait

  ┌──────────────┐     ┌──────────────────┐     ┌─────────────────┐
  │ cargo run    │     │ kild-peek click  │     │  SUCCESS:       │
  │ -p kild-ui & │────►│ --app KILD       │────►│  Clicked        │
  │              │     │ --text "Create"  │     │  "Create"       │
  └──────────────┘     │ --wait           │     └─────────────────┘
                       │ --timeout 10000  │
                       └────────┬─────────┘
                                │
                       polls every 100ms
                       until window appears
                       or 10s timeout
```

### Interaction Changes

| Command     | Before                          | After                                    | User Impact                              |
| ----------- | ------------------------------- | ---------------------------------------- | ---------------------------------------- |
| `click`     | Fails if window not found       | `--wait` polls until window appears      | Scripts work without manual sleep        |
| `type`      | Fails if window not found       | `--wait` polls until window appears      | Same                                     |
| `key`       | Fails if window not found       | `--wait` polls until window appears      | Same                                     |
| `elements`  | Fails if window not found       | `--wait` polls until window appears      | Can list elements on launching apps      |
| `find`      | Fails if window not found       | `--wait` polls until window appears      | Can find elements on launching apps      |

---

## Mandatory Reading

**CRITICAL: Implementation agent MUST read these files before starting any task:**

| Priority | File                                                  | Lines  | Why Read This                                      |
| -------- | ----------------------------------------------------- | ------ | -------------------------------------------------- |
| P0       | `crates/kild-peek-core/src/window/handler.rs`         | 1-52   | `poll_until_found()` pattern to reuse               |
| P0       | `crates/kild-peek-core/src/interact/handler.rs`       | 44-71  | `resolve_and_focus_window` + `find_window_by_target` |
| P0       | `crates/kild-peek-core/src/element/handler.rs`        | 27-54  | `find_window_by_target` + `map_window_error`         |
| P1       | `crates/kild-peek-core/src/interact/types.rs`         | 1-83   | Request types to extend                              |
| P1       | `crates/kild-peek-core/src/element/types.rs`          | 86-124 | Request types to extend                              |
| P1       | `crates/kild-peek-core/src/interact/errors.rs`        | 1-102  | Error enum to extend                                 |
| P1       | `crates/kild-peek-core/src/element/errors.rs`         | 1-63   | Error enum to extend                                 |
| P1       | `crates/kild-peek-core/src/window/errors.rs`          | 1-64   | WaitTimeout error variants to map                    |
| P2       | `crates/kild-peek/src/app.rs`                         | 168-376| CLI arg definitions (elements through assert)        |
| P2       | `crates/kild-peek/src/commands.rs`                    | 446-710| CLI handlers for elements/find/click/type/key        |

---

## Patterns to Mirror

**WAIT_FLAG_CLI_PATTERN:**
```rust
// SOURCE: crates/kild-peek/src/app.rs:363-375
// COPY THIS PATTERN for click, type, key, elements, find commands:
.arg(
    Arg::new("wait")
        .long("wait")
        .help("Wait for window to appear (polls until found or timeout)")
        .action(ArgAction::SetTrue),
)
.arg(
    Arg::new("timeout")
        .long("timeout")
        .help("Timeout in milliseconds when using --wait (default: 30000)")
        .value_parser(clap::value_parser!(u64))
        .default_value("30000"),
)
```

**WINDOW_WAIT_DISPATCH_PATTERN:**
```rust
// SOURCE: crates/kild-peek-core/src/window/handler.rs:13-52
// The poll_until_found() function already exists and handles:
// - 100ms poll interval
// - Retryable vs non-retryable error distinction
// - Timeout checking
// We reuse the existing find_window_by_*_with_wait() public functions.
```

**WINDOW_FIND_TARGET_PATTERN:**
```rust
// SOURCE: crates/kild-peek-core/src/interact/handler.rs:64-71
// Current pattern - dispatches to non-wait functions:
fn find_window_by_target(target: &InteractionTarget) -> Result<WindowInfo, InteractionError> {
    let result = match target {
        InteractionTarget::Window { title } => find_window_by_title(title),
        InteractionTarget::App { app } => find_window_by_app(app),
        InteractionTarget::AppAndWindow { app, title } => find_window_by_app_and_title(app, title),
    };
    result.map_err(map_window_error)
}
```

**ERROR_VARIANT_PATTERN:**
```rust
// SOURCE: crates/kild-peek-core/src/window/errors.rs:17-28
// Wait timeout errors follow this naming pattern:
#[error("Window '{title}' not found after {timeout_ms}ms")]
WaitTimeoutByTitle { title: String, timeout_ms: u64 },
```

**ERROR_MAPPING_PATTERN:**
```rust
// SOURCE: crates/kild-peek-core/src/interact/handler.rs:121-136
// Map WindowError to domain error:
fn map_window_error(error: WindowError) -> InteractionError {
    match error {
        WindowNotFound { title } => InteractionError::WindowNotFound { title },
        WindowNotFoundByApp { app } => InteractionError::WindowNotFoundByApp { app },
        other => InteractionError::WindowLookupFailed { reason: other.to_string() },
    }
}
```

**TEST_WAIT_TIMEOUT_PATTERN:**
```rust
// SOURCE: crates/kild-peek-core/src/window/handler.rs:1018-1028
// Test with short timeout and non-existent window:
#[test]
fn test_find_window_by_title_with_wait_timeout() {
    let result = find_window_by_title_with_wait("NONEXISTENT_WINDOW_UNIQUE_12345", 200);
    assert!(result.is_err());
    if let Err(e) = result {
        assert_eq!(e.error_code(), "WINDOW_WAIT_TIMEOUT_BY_TITLE");
    }
}
```

**CLI_WAIT_EXTRACTION_PATTERN:**
```rust
// SOURCE: crates/kild-peek/src/commands.rs:720-730
// How assert command extracts wait flags:
let wait_flag = matches.get_flag("wait");
let timeout_ms = *matches.get_one::<u64>("timeout").unwrap_or(&30000);
```

---

## Files to Change

| File                                                   | Action | Justification                                   |
| ------------------------------------------------------ | ------ | ----------------------------------------------- |
| `crates/kild-peek-core/src/interact/types.rs`          | UPDATE | Add `timeout_ms` to request types               |
| `crates/kild-peek-core/src/interact/errors.rs`         | UPDATE | Add wait timeout error variants                 |
| `crates/kild-peek-core/src/interact/handler.rs`        | UPDATE | Add wait-aware window resolution                |
| `crates/kild-peek-core/src/element/types.rs`           | UPDATE | Add `timeout_ms` to request types               |
| `crates/kild-peek-core/src/element/errors.rs`          | UPDATE | Add wait timeout error variants                 |
| `crates/kild-peek-core/src/element/handler.rs`         | UPDATE | Add wait-aware window resolution                |
| `crates/kild-peek/src/app.rs`                          | UPDATE | Add --wait/--timeout args to 5 commands         |
| `crates/kild-peek/src/commands.rs`                     | UPDATE | Extract wait flags, pass to request types       |

---

## NOT Building (Scope Limits)

- **Element-level waiting** (`--wait` to poll until a specific *element* appears, not just the window) - That's a separate concern; this phase only waits for the window to appear
- **`wait` standalone command** (proposed in issue #141 Phase 3 as `kild-peek wait --text "x"`) - Deferred to a follow-up; this plan focuses on `--wait` flag on existing commands
- **`--until-gone` flag** (wait for element to disappear) - Deferred; different polling semantics
- **Retry on element-not-found within a found window** - Out of scope; if window is found but element is missing, that's still an immediate error

---

## Step-by-Step Tasks

Execute in order. Each task is atomic and independently verifiable.

### Task 1: UPDATE `crates/kild-peek-core/src/interact/types.rs` - Add timeout_ms to request types

- **ACTION**: Add `timeout_ms: Option<u64>` field to `ClickRequest`, `TypeRequest`, `KeyComboRequest`, and `ClickTextRequest`
- **IMPLEMENT**:
  - Add `pub timeout_ms: Option<u64>` field to each struct
  - Update constructors to set `timeout_ms: None` (backward compatible)
  - Add builder method `pub fn with_wait(mut self, timeout_ms: u64) -> Self` to each type
- **MIRROR**: Follow existing struct pattern at `interact/types.rs:14-26`
- **GOTCHA**: `ClickTextRequest` has private fields with getters - add `pub fn timeout_ms(&self) -> Option<u64>` getter
- **TESTS**: Add test for each type's `with_wait()` builder
- **VALIDATE**: `cargo test -p kild-peek-core --lib interact::types`

### Task 2: UPDATE `crates/kild-peek-core/src/interact/errors.rs` - Add wait timeout error variants

- **ACTION**: Add 3 wait timeout variants to `InteractionError`
- **IMPLEMENT**:
  ```rust
  #[error("Window '{title}' not found after {timeout_ms}ms")]
  WaitTimeoutByTitle { title: String, timeout_ms: u64 },

  #[error("Window for app '{app}' not found after {timeout_ms}ms")]
  WaitTimeoutByApp { app: String, timeout_ms: u64 },

  #[error("Window '{title}' in app '{app}' not found after {timeout_ms}ms")]
  WaitTimeoutByAppAndTitle { app: String, title: String, timeout_ms: u64 },
  ```
- **ALSO UPDATE**: `error_code()` match arms with codes `"INTERACTION_WAIT_TIMEOUT_BY_TITLE"`, `"INTERACTION_WAIT_TIMEOUT_BY_APP"`, `"INTERACTION_WAIT_TIMEOUT_BY_APP_AND_TITLE"`
- **ALSO UPDATE**: `is_user_error()` - all 3 are user errors (same as window module pattern)
- **ALSO UPDATE**: `map_window_error()` in `handler.rs` to map `WindowError::WaitTimeout*` → `InteractionError::WaitTimeout*`
- **MIRROR**: `window/errors.rs:17-28` for naming, `interact/errors.rs:61-101` for trait impl
- **TESTS**: Add tests for each new variant (display, error_code, is_user_error)
- **VALIDATE**: `cargo test -p kild-peek-core --lib interact::errors`

### Task 3: UPDATE `crates/kild-peek-core/src/interact/handler.rs` - Add wait-aware window resolution

- **ACTION**: Modify `find_window_by_target()` and `resolve_and_focus_window()` to accept optional timeout
- **IMPLEMENT**:
  - Change `find_window_by_target` signature to accept `timeout_ms: Option<u64>`
  - When `Some(timeout)`, dispatch to `find_window_by_*_with_wait()` variants
  - When `None`, dispatch to `find_window_by_*()` variants (current behavior)
  - Change `resolve_and_focus_window` signature to accept `timeout_ms: Option<u64>`, pass through
  - Update `click()`, `type_text()`, `send_key_combo()` to pass `request.timeout_ms` to `resolve_and_focus_window()`
  - Update `click_text()` to pass `request.timeout_ms()` to `find_window_by_target()`
  - Add imports: `find_window_by_title_with_wait`, `find_window_by_app_with_wait`, `find_window_by_app_and_title_with_wait`
- **MIRROR**: `window/handler.rs:375-412` for how wait functions are called, `interact/handler.rs:64-71` for current dispatch
- **GOTCHA**: `click_text()` at line 378 calls `find_window_by_target()` directly (not `resolve_and_focus_window`), so update that call site too
- **TESTS**: Existing tests should still pass (timeout_ms defaults to None)
- **VALIDATE**: `cargo test -p kild-peek-core --lib interact`

### Task 4: UPDATE `crates/kild-peek-core/src/element/types.rs` - Add timeout_ms to element request types

- **ACTION**: Add `timeout_ms: Option<u64>` to `ElementsRequest` and `FindRequest`
- **IMPLEMENT**:
  - Add field to both structs, default to None in constructors
  - Add `pub fn with_wait(mut self, timeout_ms: u64) -> Self` builder method
  - Add `pub fn timeout_ms(&self) -> Option<u64>` getter (both have private fields)
- **MIRROR**: `element/types.rs:87-124` for existing struct pattern
- **TESTS**: Add tests for `with_wait()` builder on both types
- **VALIDATE**: `cargo test -p kild-peek-core --lib element::types`

### Task 5: UPDATE `crates/kild-peek-core/src/element/errors.rs` - Add wait timeout error variants

- **ACTION**: Add 3 wait timeout variants to `ElementError`
- **IMPLEMENT**:
  ```rust
  #[error("Window '{title}' not found after {timeout_ms}ms")]
  WaitTimeoutByTitle { title: String, timeout_ms: u64 },

  #[error("Window for app '{app}' not found after {timeout_ms}ms")]
  WaitTimeoutByApp { app: String, timeout_ms: u64 },

  #[error("Window '{title}' in app '{app}' not found after {timeout_ms}ms")]
  WaitTimeoutByAppAndTitle { app: String, title: String, timeout_ms: u64 },
  ```
- **ALSO UPDATE**: `error_code()` with `"ELEMENT_WAIT_TIMEOUT_BY_TITLE"`, `"ELEMENT_WAIT_TIMEOUT_BY_APP"`, `"ELEMENT_WAIT_TIMEOUT_BY_APP_AND_TITLE"`
- **ALSO UPDATE**: `is_user_error()` - all 3 are user errors
- **ALSO UPDATE**: `map_window_error()` in `handler.rs` to map `WindowError::WaitTimeout*` → `ElementError::WaitTimeout*`
- **MIRROR**: `interact/errors.rs` Task 2 pattern
- **TESTS**: Add tests for each new variant
- **VALIDATE**: `cargo test -p kild-peek-core --lib element::errors`

### Task 6: UPDATE `crates/kild-peek-core/src/element/handler.rs` - Add wait-aware window resolution

- **ACTION**: Modify `find_window_by_target()` to accept optional timeout
- **IMPLEMENT**:
  - Change `find_window_by_target` signature to accept `timeout_ms: Option<u64>`
  - When `Some(timeout)`, dispatch to `find_window_by_*_with_wait()` variants
  - When `None`, dispatch to `find_window_by_*()` variants (current behavior)
  - Update `list_elements()` to pass `request.timeout_ms()` to `find_window_by_target()`
  - Update `find_element()` - it calls `list_elements()` internally via `ElementsRequest`, so propagate timeout: create the inner `ElementsRequest` with `.with_wait()` if the `FindRequest` has a timeout
  - Add imports for `find_window_by_*_with_wait` functions
- **MIRROR**: Task 3 pattern in interact/handler.rs
- **GOTCHA**: `find_element()` creates an inner `ElementsRequest` at line 108 - must propagate timeout through
- **TESTS**: Existing tests should still pass
- **VALIDATE**: `cargo test -p kild-peek-core --lib element`

### Task 7: UPDATE `crates/kild-peek/src/app.rs` - Add --wait/--timeout CLI args

- **ACTION**: Add `--wait` and `--timeout` args to `elements`, `find`, `click`, `type`, and `key` subcommands
- **IMPLEMENT**: Add the two arg definitions (copy from assert command at lines 363-375) to each of the 5 subcommands
- **MIRROR**: `app.rs:363-375` exactly
- **TESTS**: Add CLI parsing tests for each command:
  - Test `--wait` flag is parsed
  - Test `--wait --timeout 5000` is parsed
  - Test default timeout is 30000 when `--wait` without `--timeout`
- **VALIDATE**: `cargo test -p kild-peek --lib`

### Task 8: UPDATE `crates/kild-peek/src/commands.rs` - Wire wait flags to request types

- **ACTION**: Extract `--wait`/`--timeout` in each handler and pass to request types
- **IMPLEMENT**:
  - In `handle_click_command()`: extract wait flags, create request with `.with_wait()` when wait is true
  - In `handle_click_text()`: accept timeout_ms parameter, create `ClickTextRequest` with `.with_wait()` when Some
  - In `handle_type_command()`: same pattern
  - In `handle_key_command()`: same pattern
  - In `handle_elements_command()`: same pattern
  - In `handle_find_command()`: same pattern
- **MIRROR**: Assert command wait extraction at `commands.rs:720-730`:
  ```rust
  let wait_flag = matches.get_flag("wait");
  let timeout_ms = *matches.get_one::<u64>("timeout").unwrap_or(&30000);
  ```
- **GOTCHA**: `handle_click_command()` dispatches to `handle_click_text()` for `--text` mode - pass timeout through
- **VALIDATE**: `cargo clippy --all -- -D warnings && cargo test --all && cargo build --all`

---

## Testing Strategy

### Unit Tests to Write

| Test File                                                | Test Cases                                        | Validates           |
| -------------------------------------------------------- | ------------------------------------------------- | ------------------- |
| `crates/kild-peek-core/src/interact/types.rs`            | with_wait builder on all 4 types, default None    | Request types       |
| `crates/kild-peek-core/src/interact/errors.rs`           | 3 timeout variants: display, code, is_user_error  | Error types         |
| `crates/kild-peek-core/src/element/types.rs`             | with_wait builder on 2 types, default None        | Request types       |
| `crates/kild-peek-core/src/element/errors.rs`            | 3 timeout variants: display, code, is_user_error  | Error types         |
| `crates/kild-peek/src/app.rs`                            | --wait/--timeout parsing for 5 commands           | CLI args            |

### Edge Cases Checklist

- [x] Default timeout_ms is None (backward compatible)
- [x] --timeout without --wait still works (timeout is always parsed but only used when wait=true)
- [x] Timeout errors have distinct error codes from WindowNotFound
- [x] Non-retryable errors (e.g. EnumerationFailed) propagate immediately even with --wait
- [x] WaitTimeout errors are marked as user errors

---

## Validation Commands

### Level 1: STATIC_ANALYSIS

```bash
cargo fmt --check && cargo clippy --all -- -D warnings
```

**EXPECT**: Exit 0, no errors or warnings

### Level 2: UNIT_TESTS

```bash
cargo test -p kild-peek-core --lib interact && cargo test -p kild-peek-core --lib element && cargo test -p kild-peek
```

**EXPECT**: All tests pass

### Level 3: FULL_SUITE

```bash
cargo test --all && cargo build --all
```

**EXPECT**: All tests pass, build succeeds

### Level 6: MANUAL_VALIDATION

```bash
# Test --wait on click (should timeout with clear error)
cargo run -p kild-peek -- click --app "NONEXISTENT_APP_12345" --at 100,50 --wait --timeout 1000

# Test --wait on elements (should timeout with clear error)
cargo run -p kild-peek -- elements --app "NONEXISTENT_APP_12345" --wait --timeout 1000

# Test --wait on find (should timeout with clear error)
cargo run -p kild-peek -- find --app "NONEXISTENT_APP_12345" --text "foo" --wait --timeout 1000

# Test --wait on type (should timeout with clear error)
cargo run -p kild-peek -- type --app "NONEXISTENT_APP_12345" "hello" --wait --timeout 1000

# Test --wait on key (should timeout with clear error)
cargo run -p kild-peek -- key --app "NONEXISTENT_APP_12345" "enter" --wait --timeout 1000

# Test backward compat (no --wait, should fail immediately)
cargo run -p kild-peek -- click --app "NONEXISTENT_APP_12345" --at 100,50

# Test --wait with real app (should succeed)
cargo run -p kild-peek -- elements --app Finder --wait --timeout 5000
```

---

## Acceptance Criteria

- [ ] All 5 commands (click, type, key, elements, find) accept `--wait` and `--timeout` flags
- [ ] With `--wait`, commands poll every 100ms until window appears or timeout
- [ ] Without `--wait`, commands behave exactly as before (immediate failure)
- [ ] Timeout errors display time waited (e.g., "Window for app 'X' not found after 5000ms")
- [ ] Default timeout is 30000ms (matches screenshot/assert)
- [ ] Non-retryable errors propagate immediately even with `--wait`
- [ ] Level 1-3 validation commands pass
- [ ] No regressions in existing tests (788+ tests)

---

## Completion Checklist

- [ ] All tasks completed in dependency order
- [ ] Each task validated immediately after completion
- [ ] Level 1: Static analysis (fmt + clippy) passes
- [ ] Level 2: Unit tests pass
- [ ] Level 3: Full test suite + build succeeds
- [ ] All acceptance criteria met
- [ ] CLAUDE.md updated with new CLI examples if warranted

---

## Risks and Mitigations

| Risk                                        | Likelihood | Impact | Mitigation                                                           |
| ------------------------------------------- | ---------- | ------ | -------------------------------------------------------------------- |
| Breaking existing tests by changing fn sigs | LOW        | MED    | timeout_ms defaults to None; all existing call sites unchanged       |
| Forgetting to propagate timeout in find_element inner call | MED | MED | Task 6 explicitly calls this out; test verifies                    |
| click_text has different window lookup path  | LOW        | MED    | Task 3 explicitly handles click_text's direct find_window_by_target |

---

## Notes

- The `poll_until_found()` function in `window/handler.rs` already handles all the wait/retry mechanics. This plan does NOT create any new polling logic - it reuses the existing `find_window_by_*_with_wait()` public functions.
- The `--wait` flag only waits for the **window** to appear. If the window appears but an element is not found (for `click --text` or `find`), that's still an immediate error. Element-level waiting would be a separate feature.
- Issue #141 Phase 3 also proposes a standalone `wait` command and `--until-gone`. These are deferred to follow-up work as they involve different semantics.
