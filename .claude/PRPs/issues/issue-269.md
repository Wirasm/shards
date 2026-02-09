# Investigation: Ghostty windows invisible to System Events — replace AppleScript with Core Graphics API

**Issue**: #269 (https://github.com/Wirasm/kild/issues/269)
**Type**: BUG
**Investigated**: 2026-02-09T12:00:00Z

### Assessment

| Metric     | Value    | Reasoning                                                                                                                                                                   |
| ---------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Severity   | HIGH     | Three core commands (`focus`, `hide`, `is_window_open`) are completely broken on Ghostty — the default and most popular terminal backend — with no workaround for users.     |
| Complexity | MEDIUM   | 3 files to modify (ghostty.rs, errors.rs, Cargo.toml) plus 1 new module (native/macos.rs). Changes are isolated to the Ghostty backend with clear integration points.       |
| Confidence | HIGH     | Root cause is confirmed and documented upstream. Working solution (xcap/Core Graphics) is proven in kild-peek-core. The exact API surface and FFI patterns are well understood. |

---

## Problem Statement

Ghostty (v1.2.3) uses GPU rendering which bypasses the native widget tree that macOS System Events relies on. As a result, `System Events` reports 0 windows for the Ghostty process, causing `focus_window`, `hide_window`, and `is_window_open` to always fail. The Core Graphics API (`CGWindowListCopyWindowInfo`) sees Ghostty windows correctly — kild-peek already proves this with the `xcap` crate.

---

## Analysis

### Root Cause

WHY: `kild focus`, `kild hide`, and status detection fail for Ghostty sessions
↓ BECAUSE: The Ghostty backend's `focus_by_pid`, `focus_by_title`, `hide_by_pid`, `hide_by_title`, `check_window_by_pid`, and `check_window_by_title` all use AppleScript via System Events
Evidence: `crates/kild-core/src/terminal/backends/ghostty.rs:155-169` — AppleScript `tell application "System Events"` with `count of windows`

↓ BECAUSE: System Events queries the native widget tree to enumerate windows
↓ BECAUSE: Ghostty uses Metal GPU rendering which bypasses the native widget tree
Evidence: `osascript -e 'tell application "System Events" to get count of windows of process "Ghostty"'` → 0

↓ ROOT CAUSE: Ghostty windows are invisible to macOS System Events due to GPU rendering. This is a known Ghostty limitation (tracked upstream at ghostty-org/ghostty#2353). The fix is to use Core Graphics API (`CGWindowListCopyWindowInfo` via xcap crate) for window enumeration and macOS Accessibility API for window manipulation — both of which DO see Ghostty windows.

### Evidence Chain

**System Events sees 0 windows:**
```bash
osascript -e 'tell application "System Events" to get count of windows of process "Ghostty"'
# → 0
```

**Core Graphics sees them fine:**
```bash
kild-peek list windows --app Ghostty
# → 1 window: "✳ Claude Code" (ID: 26048)
```

**Working implementation in kild-peek-core:**
- `crates/kild-peek-core/src/window/handler.rs:55-197` — `list_windows()` uses `xcap::Window::all()` successfully
- `crates/kild-peek-core/src/window/handler.rs:714-795` — `find_window_by_app_and_title()` finds Ghostty windows by app + title

### Affected Files

| File                                                     | Lines     | Action | Description                                                       |
| -------------------------------------------------------- | --------- | ------ | ----------------------------------------------------------------- |
| `crates/kild-core/Cargo.toml`                            | 8-24      | UPDATE | Add xcap, accessibility-sys, core-foundation as macOS-only deps   |
| `crates/kild-core/src/terminal/native/mod.rs`            | NEW       | CREATE | Module declaration with `cfg(target_os = "macos")`                |
| `crates/kild-core/src/terminal/native/types.rs`          | NEW       | CREATE | Minimal `NativeWindowInfo` struct (id, title, app_name, pid, is_minimized) |
| `crates/kild-core/src/terminal/native/macos.rs`          | NEW       | CREATE | Core Graphics enumeration + Accessibility API manipulation        |
| `crates/kild-core/src/terminal/mod.rs`                   | 1-8       | UPDATE | Add `pub mod native;` declaration                                 |
| `crates/kild-core/src/terminal/errors.rs`                | 4-40      | UPDATE | Add `NativeWindowError` variant for CG/AX failures               |
| `crates/kild-core/src/terminal/backends/ghostty.rs`      | 144-589   | UPDATE | Replace 6 AppleScript functions with native API calls             |

### Integration Points

- `ghostty.rs:821-838` — `focus_window`, `hide_window`, `is_window_open` trait impl calls `with_ghostty_window` which dispatches to `focus_by_pid`/`focus_by_title`, etc.
- `ghostty.rs:598-676` — `with_ghostty_window` orchestrates PID → title fallback strategy
- `ghostty.rs:21-94` — `find_ghostty_pid_by_session` provides PID discovery (KEEP - useful for CG window matching by PID)
- `terminal/operations.rs:165-230` — `focus_terminal_window`, `hide_terminal_window`, `is_terminal_window_open` call backend methods
- `sessions/info.rs:118-122` — `check_agent_process_status` calls `is_terminal_window_open` for status detection

### Git History

- **Most recent**: `6ccfb49` — "refactor: deduplicate terminal backend common patterns (#273)"
- **Window lookup helper**: `e34fa27` — "fix: extract shared Ghostty window lookup helper (#268)"
- **Hide command**: `b38f06f` — "feat: add hide command and window metadata (#253)"
- **Stale window ID fix**: `061ff19` — "fix: resolve stale window ID after stop/open cycle in Ghostty (#239)"
- **Implication**: The AppleScript approach has been iteratively refined but fundamentally cannot work with Ghostty's GPU rendering

---

## Implementation Plan

### Step 1: Add macOS-only dependencies to kild-core

**File**: `crates/kild-core/Cargo.toml`
**Lines**: After line 24
**Action**: UPDATE

**Current code:**
```toml
[dependencies]
thiserror.workspace = true
# ... existing deps ...
which.workspace = true
```

**Required change:**
```toml
[dependencies]
thiserror.workspace = true
# ... existing deps ...
which.workspace = true

[target.'cfg(target_os = "macos")'.dependencies]
xcap.workspace = true
accessibility-sys.workspace = true
core-foundation.workspace = true
```

**Why**: xcap provides Core Graphics window enumeration, accessibility-sys provides AXUIElement FFI for window focus/minimize, core-foundation provides CFRelease and type helpers. All are macOS-only.

---

### Step 2: Create native window types

**File**: `crates/kild-core/src/terminal/native/types.rs`
**Action**: CREATE

**Content:**
```rust
/// Minimal window info from Core Graphics API.
/// Only contains the fields needed for Ghostty window management.
#[derive(Debug, Clone)]
pub struct NativeWindowInfo {
    /// Core Graphics window ID
    pub id: u32,
    /// Window title
    pub title: String,
    /// Application name
    pub app_name: String,
    /// Process ID (if available)
    pub pid: Option<i32>,
    /// Whether the window is minimized
    pub is_minimized: bool,
}
```

**Why**: Minimal subset of kild-peek-core's `WindowInfo` — only the fields needed for find/focus/hide operations. No position/size needed.

---

### Step 3: Create native macOS window management module

**File**: `crates/kild-core/src/terminal/native/macos.rs`
**Action**: CREATE

**Required functionality (3 public functions):**

```rust
use crate::terminal::errors::TerminalError;
use super::types::NativeWindowInfo;

/// Find a window by app name and partial title match using Core Graphics API (via xcap).
///
/// Enumerates all visible windows, filters to those belonging to `app_name`,
/// then finds one whose title contains `title_contains` (case-insensitive).
pub fn find_window(app_name: &str, title_contains: &str) -> Result<Option<NativeWindowInfo>, TerminalError> {
    // 1. Call xcap::Window::all() to enumerate windows
    // 2. Filter by app_name (case-insensitive)
    // 3. Find window where title contains title_contains (case-insensitive)
    // 4. Return NativeWindowInfo or None if not found
    //
    // Mirror pattern from kild-peek-core/src/window/handler.rs:714-795
    // but simplified: only extract id, title, app_name, pid, is_minimized
}

/// Find a window by app name and PID using Core Graphics API (via xcap).
///
/// Enumerates all visible windows, filters to those belonging to `app_name`
/// with matching PID.
pub fn find_window_by_pid(app_name: &str, pid: u32) -> Result<Option<NativeWindowInfo>, TerminalError> {
    // 1. Call xcap::Window::all() to enumerate windows
    // 2. Filter by app_name AND pid match
    // 3. Return first matching NativeWindowInfo or None
}

/// Focus (raise) a specific window using the macOS Accessibility API.
///
/// Uses AXUIElementCreateApplication(pid) to get the app's AX element,
/// then iterates its windows to find the one matching the window title,
/// and performs AXRaise + NSRunningApplication.activate to bring it to front.
pub fn focus_window(window: &NativeWindowInfo) -> Result<(), TerminalError> {
    // 1. Get PID from window info (required for Accessibility API)
    // 2. AXUIElementCreateApplication(pid)
    // 3. AXUIElementSetMessagingTimeout for safety
    // 4. Get windows via kAXWindowsAttribute
    // 5. Find matching window by title (kAXTitleAttribute)
    // 6. AXUIElementSetAttributeValue(window, kAXRaisedAttribute, kCFBooleanTrue)
    //    OR performAction(kAXRaiseAction) — try both approaches
    // 7. Activate the app via NSRunningApplication or `tell application "Ghostty" to activate`
    // 8. CFRelease all created elements
    //
    // FFI pattern from kild-peek-core/src/element/accessibility.rs:101-132
    //
    // NOTE: If Accessibility API fails (Ghostty doesn't expose AX windows),
    // fall back to `tell application "Ghostty" to activate` which at least
    // brings the app to foreground (just can't target specific window)
}

/// Minimize a specific window using the macOS Accessibility API.
///
/// Uses AXUIElementCreateApplication(pid) to get the app's AX element,
/// then sets kAXMinimizedAttribute to true on the matching window.
pub fn minimize_window(window: &NativeWindowInfo) -> Result<(), TerminalError> {
    // 1. Get PID from window info
    // 2. AXUIElementCreateApplication(pid)
    // 3. AXUIElementSetMessagingTimeout
    // 4. Get windows via kAXWindowsAttribute
    // 5. Find matching window by title
    // 6. AXUIElementSetAttributeValue(window, kAXMinimizedAttribute, kCFBooleanTrue)
    // 7. CFRelease all created elements
    //
    // NOTE: If Accessibility API fails (Ghostty doesn't expose AX windows),
    // fall back to `tell application "System Events" to set visible of process "Ghostty" to false`
    // which hides all windows (less precise but functional)
}
```

**Important implementation notes:**
- Use `xcap::Window::all()` for enumeration (proven to work with Ghostty)
- For Accessibility API: Ghostty may NOT expose AX windows (the same GPU rendering issue). If AXRaise/AXMinimized fail, use fallback strategies:
  - Focus fallback: `tell application "Ghostty" to activate` (brings app to front, just can't target specific window)
  - Hide fallback: `tell application "System Events" to set visible of process "Ghostty" to false`
- Always CFRelease AXUIElementRef manually (no RAII wrapper available)
- Set messaging timeout to 1.0s to avoid hangs on unresponsive apps
- Log all operations with `core.terminal.native.*` event names

---

### Step 4: Create native module declaration

**File**: `crates/kild-core/src/terminal/native/mod.rs`
**Action**: CREATE

**Content:**
```rust
#[cfg(target_os = "macos")]
mod macos;
mod types;

pub use types::NativeWindowInfo;

#[cfg(target_os = "macos")]
pub use macos::{find_window, find_window_by_pid, focus_window, minimize_window};
```

**Why**: Platform-gated module — only the macOS implementation is compiled on macOS. Types are available on all platforms for cross-compilation.

---

### Step 5: Register native module in terminal mod.rs

**File**: `crates/kild-core/src/terminal/mod.rs`
**Lines**: 1-8
**Action**: UPDATE

**Current code:**
```rust
pub mod backends;
pub mod common;
pub mod errors;
pub mod handler;
pub mod operations;
pub mod registry;
pub mod traits;
pub mod types;
```

**Required change:**
```rust
pub mod backends;
pub mod common;
pub mod errors;
pub mod handler;
pub mod native;
pub mod operations;
pub mod registry;
pub mod traits;
pub mod types;
```

**Why**: Expose the new native window management module.

---

### Step 6: Add error variant for native window operations

**File**: `crates/kild-core/src/terminal/errors.rs`
**Lines**: 4-40
**Action**: UPDATE

**Current code (add after HideFailed):**
```rust
    #[error("Failed to hide terminal window: {message}")]
    HideFailed { message: String },
```

**Required change (add new variant):**
```rust
    #[error("Failed to hide terminal window: {message}")]
    HideFailed { message: String },

    #[error("Native window operation failed: {message}")]
    NativeWindowError { message: String },
```

Also update `error_code()` match:
```rust
TerminalError::NativeWindowError { .. } => "NATIVE_WINDOW_ERROR",
```

And `is_user_error()` match — add `TerminalError::NativeWindowError { .. }` to the list.

**Why**: Distinct error type for Core Graphics / Accessibility API failures, separate from AppleScript errors.

---

### Step 7: Replace Ghostty AppleScript functions with native API calls

**File**: `crates/kild-core/src/terminal/backends/ghostty.rs`
**Lines**: 144-589
**Action**: UPDATE

**Replace the 6 AppleScript-based functions** (`focus_by_pid`, `focus_by_title`, `hide_by_pid`, `hide_by_title`, `check_window_by_pid`, `check_window_by_title`) with native API implementations.

**Simplify `with_ghostty_window`** — the current PID→title fallback strategy can be simplified since both CG lookup approaches (by title and by PID) now use the same underlying `xcap::Window::all()` call:

```rust
/// Look up a Ghostty window using Core Graphics API, with PID fallback.
///
/// Strategy:
/// 1. Try to find by app name + title match (via xcap)
/// 2. If title doesn't match (agent changed it), try PID-based lookup
#[cfg(target_os = "macos")]
fn find_ghostty_native_window(window_id: &str) -> Result<Option<NativeWindowInfo>, TerminalError> {
    use crate::terminal::native;

    // Primary: find by title (most common case - title matches session ID)
    if let Some(window) = native::find_window("Ghostty", window_id)? {
        return Ok(Some(window));
    }

    // Fallback: find by PID (when agent has changed the window title)
    if let Some(pid) = find_ghostty_pid_by_session(window_id) {
        if let Some(window) = native::find_window_by_pid("Ghostty", pid)? {
            return Ok(Some(window));
        }
    }

    Ok(None)
}
```

**New trait implementations:**

```rust
#[cfg(target_os = "macos")]
fn focus_window(&self, window_id: &str) -> Result<(), TerminalError> {
    use crate::terminal::native;

    debug!(event = "core.terminal.focus_ghostty_started", window_id = %window_id);

    match find_ghostty_native_window(window_id)? {
        Some(window) => {
            native::focus_window(&window)?;
            info!(event = "core.terminal.focus_completed", terminal = "Ghostty", window_id = %window_id);
            Ok(())
        }
        None => Err(TerminalError::FocusFailed {
            message: format!("No Ghostty window found matching '{}'", window_id),
        }),
    }
}

#[cfg(target_os = "macos")]
fn hide_window(&self, window_id: &str) -> Result<(), TerminalError> {
    use crate::terminal::native;

    debug!(event = "core.terminal.hide_ghostty_started", window_id = %window_id);

    match find_ghostty_native_window(window_id)? {
        Some(window) => {
            native::minimize_window(&window)?;
            info!(event = "core.terminal.hide_completed", terminal = "Ghostty", window_id = %window_id);
            Ok(())
        }
        None => Err(TerminalError::HideFailed {
            message: format!("No Ghostty window found matching '{}'", window_id),
        }),
    }
}

#[cfg(target_os = "macos")]
fn is_window_open(&self, window_id: &str) -> Result<Option<bool>, TerminalError> {
    debug!(event = "core.terminal.check_ghostty_started", window_id = %window_id);

    match find_ghostty_native_window(window_id)? {
        Some(window) => {
            let is_open = !window.is_minimized;
            debug!(
                event = "core.terminal.check_ghostty_completed",
                window_id = %window_id,
                is_open = is_open,
                is_minimized = window.is_minimized
            );
            Ok(Some(is_open))
        }
        None => {
            debug!(
                event = "core.terminal.check_ghostty_not_found",
                window_id = %window_id
            );
            Ok(Some(false))
        }
    }
}
```

**What to remove:**
- `focus_by_pid` (lines 144-219) — replaced by `native::focus_window`
- `focus_by_title` (lines 223-295) — replaced by `native::focus_window`
- `hide_by_pid` (lines 299-370) — replaced by `native::minimize_window`
- `hide_by_title` (lines 374-445) — replaced by `native::minimize_window`
- `check_window_by_pid` (lines 449-512) — replaced by `native::find_window`
- `check_window_by_title` (lines 516-589) — replaced by `native::find_window`
- `with_ghostty_window` (lines 591-676) — replaced by `find_ghostty_native_window`

**What to keep:**
- `find_ghostty_pid_by_session` (lines 21-94) — still useful for PID-based CG lookup fallback
- `is_ghostty_process` (lines 123-142) — used by `find_ghostty_pid_by_session`
- `get_parent_pid` (lines 98-119) — used by `find_ghostty_pid_by_session`
- `escape_regex` import — still used in `close_window`
- `execute_spawn` — unchanged (uses `open -a Ghostty`, works fine)
- `close_window` — unchanged (uses `pkill -f`, works fine)

**What to remove from imports:**
- `applescript_escape` — no longer used (was used in `focus_by_title` and `hide_by_title`)

---

### Step 8: Update tests

**File**: `crates/kild-core/src/terminal/backends/ghostty.rs`
**Lines**: 848-1013
**Action**: UPDATE

**Tests to keep as-is:**
- `test_ghostty_backend_name`
- `test_ghostty_backend_display_name`
- `test_ghostty_close_window_skips_when_no_id`
- `test_ghostty_pkill_pattern_escaping`
- `test_ghostty_spawn_command_structure`
- `test_is_ghostty_process_helper`
- `test_find_ghostty_pid_no_match`
- `test_pid_parsing_handles_malformed_output`
- `test_ghostty_comm_matching_is_case_insensitive`
- `test_get_parent_pid_for_current_process`
- `test_get_parent_pid_nonexistent_process`
- `test_close_window_pkill_pattern_no_ghostty_prefix`

**Tests to update:**
- `test_is_window_open_returns_option_type` — keep behavior test, update if needed for new implementation
- `test_is_window_open_ghostty_not_running` — keep as ignored integration test

**Tests to add:**

```rust
#[cfg(target_os = "macos")]
#[test]
fn test_find_ghostty_native_window_not_found() {
    // When no Ghostty window matches, should return None
    let result = find_ghostty_native_window("nonexistent-window-xyz-12345");
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[cfg(target_os = "macos")]
#[test]
fn test_native_window_info_fields() {
    use crate::terminal::native::NativeWindowInfo;
    let info = NativeWindowInfo {
        id: 12345,
        title: "test-window".to_string(),
        app_name: "Ghostty".to_string(),
        pid: Some(9999),
        is_minimized: false,
    };
    assert_eq!(info.id, 12345);
    assert_eq!(info.title, "test-window");
    assert_eq!(info.app_name, "Ghostty");
    assert_eq!(info.pid, Some(9999));
    assert!(!info.is_minimized);
}
```

---

## Patterns to Follow

**From kild-peek-core — xcap window enumeration pattern:**

```rust
// SOURCE: crates/kild-peek-core/src/window/handler.rs:55-60
// Pattern for enumerating windows via Core Graphics API
let windows = xcap::Window::all().map_err(|e| WindowError::EnumerationFailed {
    message: e.to_string(),
})?;
```

**From kild-peek-core — Accessibility API setup pattern:**

```rust
// SOURCE: crates/kild-peek-core/src/element/accessibility.rs:104-113
// Pattern for creating AX element and setting timeout
let app_element = unsafe { AXUIElementCreateApplication(pid) };
if app_element.is_null() {
    return Err(format!("Failed to create AX element for PID {}", pid));
}
unsafe {
    AXUIElementSetMessagingTimeout(app_element, AX_MESSAGING_TIMEOUT);
}
```

**From kild-peek-core — CFRelease pattern:**

```rust
// SOURCE: crates/kild-peek-core/src/element/accessibility.rs:127-129
// Pattern for manual memory management (no RAII wrapper for AXUIElementRef)
unsafe {
    core_foundation::base::CFRelease(app_element as *mut c_void);
}
```

**From ghostty.rs — event naming pattern:**

```rust
// SOURCE: crates/kild-core/src/terminal/backends/ghostty.rs:714
// Pattern for Ghostty-specific terminal events
debug!(event = "core.terminal.spawn_ghostty_starting", ...);
info!(event = "core.terminal.focus_completed", terminal = "Ghostty", ...);
```

---

## Edge Cases & Risks

| Risk/Edge Case                                   | Mitigation                                                                                                                        |
| ------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------- |
| Ghostty AX windows may be invisible too          | Fall back to `tell application "Ghostty" to activate` for focus (brings app to front). For hide, use `set visible of process to false`. Log the fallback. |
| Multiple Ghostty windows with similar titles     | Title matching uses `contains` (partial match). Session IDs are unique enough to avoid false matches.                             |
| Agent changes window title (e.g., Claude changes title to "✳ Claude Code") | PID-based fallback via `find_ghostty_pid_by_session` → `find_window_by_pid` catches this case.                                   |
| xcap fails to enumerate (permissions)            | Return `TerminalError::NativeWindowError` with clear message about Screen Recording permissions.                                 |
| Accessibility API requires permissions           | macOS may prompt for Accessibility permissions. Surface the error clearly if denied.                                              |
| Linux/non-macOS compilation                      | All new code is `cfg(target_os = "macos")` gated. `NativeWindowInfo` type is available on all platforms.                          |
| Memory leaks from AXUIElement                    | Follow kild-peek-core pattern: always `CFRelease` in a finally-like scope. Consider using `scopeguard` if cleanup paths are complex. |

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

1. `kild create test-branch --agent claude` — create a Ghostty session
2. `kild focus test-branch` — verify window comes to foreground (was always failing before)
3. `kild hide test-branch` — verify window minimizes to Dock (was always failing before)
4. `kild list` — verify status detection shows "running" not "unknown" (was unreliable before)
5. `kild stop test-branch && kild open test-branch` — verify focus/hide work after stop/open cycle
6. Test with multiple Ghostty sessions open simultaneously

---

## Scope Boundaries

**IN SCOPE:**
- Replace 6 AppleScript functions in ghostty.rs with Core Graphics + Accessibility API
- New `terminal/native/` module with types.rs, macos.rs, mod.rs
- Add macOS-only dependencies (xcap, accessibility-sys, core-foundation)
- Add `NativeWindowError` variant to `TerminalError`
- Update existing tests and add new unit tests

**OUT OF SCOPE (do not touch):**
- `execute_spawn` — uses `open -a Ghostty`, works fine
- `close_window` — uses `pkill -f`, works fine
- `find_ghostty_pid_by_session` — keep as-is, useful for PID fallback
- iTerm, Terminal.app, Alacritty backends — no changes needed
- kild-peek-core — remains independent (no cross-crate dependency)
- UI layer (kild-ui) — no changes needed
- Linux/Hyprland support — out of scope for this issue

---

## Metadata

- **Investigated by**: Claude
- **Timestamp**: 2026-02-09T12:00:00Z
- **Artifact**: `.claude/PRPs/issues/issue-269.md`
