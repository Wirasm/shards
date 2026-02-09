# Investigation: iTerm kild create sometimes opens two windows

**Issue**: #271 (https://github.com/Wirasm/kild/issues/271)
**Type**: BUG
**Investigated**: 2026-02-09T12:00:00Z

### Assessment

| Metric     | Value  | Reasoning                                                                                                      |
| ---------- | ------ | -------------------------------------------------------------------------------------------------------------- |
| Severity   | MEDIUM | Cosmetic extra window; the kild session itself works correctly. User can manually close the default window.     |
| Complexity | LOW    | Single file change to one AppleScript constant. No integration points affected, no API changes.                 |
| Confidence | HIGH   | Root cause is clear from code inspection: `ITERM_SCRIPT` lacks `activate` and window-count guard on cold start. |

---

## Problem Statement

When iTerm is not already running, `kild create <branch> --terminal iterm` opens **two** iTerm windows: one default window from iTerm's startup behavior, and one from the AppleScript `create window with default profile` command. The issue is intermittent because it only occurs on cold start (iTerm not running).

---

## Analysis

### Root Cause

The `ITERM_SCRIPT` AppleScript template at `crates/kild-core/src/terminal/backends/iterm.rs:16-23` uses `tell application "iTerm"` which launches iTerm if not running. On cold start, iTerm automatically opens a default window as part of its startup sequence. The script then calls `create window with default profile`, producing a second window.

### Evidence Chain

WHY: Two iTerm windows appear when creating a kild session
↓ BECAUSE: The AppleScript creates a new window via `create window with default profile`
Evidence: `crates/kild-core/src/terminal/backends/iterm.rs:17` - `set newWindow to (create window with default profile)`

↓ BECAUSE: iTerm also opens a default window on cold start (standard iTerm behavior)
Evidence: `tell application "iTerm"` at line 16 launches iTerm if not running; iTerm's default startup opens a window

↓ ROOT CAUSE: The script doesn't account for iTerm's cold-start behavior — it should either reuse the default startup window or close the extra window after creation
Evidence: `crates/kild-core/src/terminal/backends/iterm.rs:16-23` — no window count check, no activate, no startup-window reuse

### Affected Files

| File                                                       | Lines | Action | Description                                    |
| ---------------------------------------------------------- | ----- | ------ | ---------------------------------------------- |
| `crates/kild-core/src/terminal/backends/iterm.rs`          | 16-23 | UPDATE | Fix `ITERM_SCRIPT` to handle cold-start window |

### Integration Points

- `crates/kild-core/src/terminal/common/applescript.rs:9` — `execute_spawn_script` executes the script; no changes needed
- `crates/kild-core/src/terminal/operations.rs:85` — calls `backend.execute_spawn`; no changes needed
- `crates/kild-core/src/sessions/handler.rs:226` — session creation triggers spawn; no changes needed

### Git History

- **Introduced**: `160314d` - Rebrand Shards to KILD (script existed since initial implementation)
- **Last modified**: `6ccfb49` - refactor: deduplicate terminal backend common patterns (#273)
- **Implication**: Long-standing bug since original iTerm backend implementation

---

## Implementation Plan

### Step 1: Update `ITERM_SCRIPT` to reuse cold-start default window

**File**: `crates/kild-core/src/terminal/backends/iterm.rs`
**Lines**: 16-23
**Action**: UPDATE

**Current code:**

```rust
const ITERM_SCRIPT: &str = r#"tell application "iTerm"
        set newWindow to (create window with default profile)
        set windowId to id of newWindow
        tell current session of newWindow
            write text "{command}"
        end tell
        return windowId
    end tell"#;
```

**Required change:**

```rust
const ITERM_SCRIPT: &str = r#"tell application "iTerm"
        activate
        if (count of windows) is 0 then
            set newWindow to (create window with default profile)
        else
            set existingCount to count of windows
            set newWindow to (create window with default profile)
            -- If iTerm just launched and created a default window alongside ours,
            -- close the extra window to ensure exactly one window per kild session.
            if (count of windows) > (existingCount + 1) then
                -- Our window is newWindow; close any other window that appeared
                repeat with w in windows
                    if id of w is not equal to id of newWindow then
                        close w
                        exit repeat
                    end if
                end repeat
            end if
        end if
        set windowId to id of newWindow
        tell current session of newWindow
            write text "{command}"
        end tell
        return windowId
    end tell"#;
```

**Wait — simpler approach.** The above is over-engineered. The real fix is simpler: when iTerm is launched cold, `tell application "iTerm"` causes it to open a default window. Instead of creating a new window, we should detect that a fresh default window exists and reuse it:

```rust
const ITERM_SCRIPT: &str = r#"tell application "iTerm"
        activate
        -- After activate, if iTerm just launched it will have one default window.
        -- Check if we can reuse it (single window, single session, default session).
        set shouldReuse to false
        if (count of windows) is 1 then
            tell current session of window 1
                if (is at shell prompt) then
                    set shouldReuse to true
                end if
            end tell
        end if

        if shouldReuse then
            set newWindow to window 1
        else
            set newWindow to (create window with default profile)
        end if
        set windowId to id of newWindow
        tell current session of newWindow
            write text "{command}"
        end tell
        return windowId
    end tell"#;
```

**Problem:** `is at shell prompt` is an iTerm-specific AppleScript property that may not be reliable across all iTerm versions. Let me reconsider.

**Simplest correct approach:** The cleanest fix is to **count windows before and after** `create window`, and close the extra:

```rust
const ITERM_SCRIPT: &str = r#"tell application "iTerm"
        activate
        set windowCountBefore to count of windows
        set newWindow to (create window with default profile)
        set windowId to id of newWindow
        -- If iTerm launched fresh, it may have opened a default window alongside ours.
        -- Close any extra windows that appeared (not our new window).
        if (count of windows) > (windowCountBefore + 1) then
            repeat with w in windows
                if id of w is not equal to windowId then
                    close w
                    exit repeat
                end if
            end repeat
        end if
        tell current session of newWindow
            write text "{command}"
        end tell
        return windowId
    end tell"#;
```

**Why**: This handles both cold-start (iTerm not running) and warm-start (iTerm already running) correctly:
- **Cold start**: `activate` launches iTerm, which may open a default window. `windowCountBefore` is 0 (captured after launch but before `create window`). If after `create window` there are 2 windows instead of 1, the extra is closed.
- **Warm start**: `windowCountBefore` reflects existing windows. `create window` adds exactly 1. Count matches `windowCountBefore + 1`, no cleanup needed.

**Actually, there's a subtlety:** `count of windows` after `activate` but before `create window` — when iTerm cold-starts, `activate` triggers the launch and the default window opens. So `windowCountBefore` would be 1 (the default window), then `create window` adds another, making 2. The condition `2 > 1 + 1` = `2 > 2` is false, so nothing is cleaned up. The default window persists.

The issue is that `activate` is asynchronous — the default window may not have appeared yet when we check `count of windows`. The timing is unpredictable.

**Best approach — reuse the startup window directly:**

```rust
const ITERM_SCRIPT: &str = r#"tell application "iTerm"
        if not (application "iTerm" is running) then
            activate
            -- iTerm just launched and will create a default window.
            -- Reuse that window instead of creating a new one.
            set newWindow to current window
        else
            set newWindow to (create window with default profile)
        end if
        set windowId to id of newWindow
        tell current session of newWindow
            write text "{command}"
        end tell
        return windowId
    end tell"#;
```

**Problem:** The `if not (application "iTerm" is running)` check happens *inside* `tell application "iTerm"` which itself launches the app, so by the time we check, it IS running. The check must happen *before* the `tell` block.

**Final correct approach:**

```rust
const ITERM_SCRIPT: &str = r#"set iTermWasRunning to application "iTerm" is running
    tell application "iTerm"
        activate
        if not iTermWasRunning then
            -- iTerm just launched and created a default window.
            -- Reuse it instead of creating a second window.
            set newWindow to current window
        else
            set newWindow to (create window with default profile)
        end if
        set windowId to id of newWindow
        tell current session of newWindow
            write text "{command}"
        end tell
        return windowId
    end tell"#;
```

**Why this is correct:**
1. Check `application "iTerm" is running` **before** the `tell` block — this accurately captures whether iTerm was already running
2. If iTerm was NOT running: `tell application "iTerm"` + `activate` launches it, creating a default window. We reuse `current window` instead of creating a second one.
3. If iTerm WAS running: `activate` brings it to foreground, then we `create window with default profile` as before.
4. `activate` is added for both paths — it brings iTerm to foreground which is consistent with the focus script pattern (`ITERM_FOCUS_SCRIPT` at line 36-39 also uses `activate`).

### Step 2: Update test to verify script structure

**File**: `crates/kild-core/src/terminal/backends/iterm.rs`
**Lines**: 128-149 (test module)
**Action**: UPDATE

**Test cases to update/add:**

```rust
#[cfg(target_os = "macos")]
#[test]
fn test_iterm_script_checks_running_state() {
    assert!(ITERM_SCRIPT.contains("application \"iTerm\" is running"));
}

#[cfg(target_os = "macos")]
#[test]
fn test_iterm_script_reuses_window_on_cold_start() {
    assert!(ITERM_SCRIPT.contains("current window"));
}

#[cfg(target_os = "macos")]
#[test]
fn test_iterm_script_creates_window_when_running() {
    assert!(ITERM_SCRIPT.contains("create window with default profile"));
}
```

---

## Patterns to Follow

**From codebase - iTerm focus script uses `activate`:**

```rust
// SOURCE: crates/kild-core/src/terminal/backends/iterm.rs:36-39
const ITERM_FOCUS_SCRIPT: &str = r#"tell application "iTerm"
        activate
        set frontmost of window id {window_id} to true
    end tell"#;
```

**From codebase - check running state before `tell` is a standard AppleScript pattern:**

The `application "X" is running` check outside a `tell` block is idiomatic AppleScript for detecting cold vs warm start.

---

## Edge Cases & Risks

| Risk/Edge Case                                    | Mitigation                                                                                   |
| ------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| iTerm launched but no windows open (all closed)   | Falls into `iTermWasRunning = true` path, creates new window — correct behavior              |
| iTerm in "Restore Previous Session" mode           | `current window` captures the restored window; command is written to it — acceptable          |
| `current window` errors if iTerm starts minimized | Unlikely for cold start; `activate` ensures app is in foreground before accessing `current window` |
| Multiple rapid `kild create` calls                | Each checks running state independently; second call sees iTerm running and creates new window |

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

1. Quit iTerm completely, run `cargo run -p kild -- create test-branch --terminal iterm` — verify exactly one window opens
2. With iTerm already running, run `cargo run -p kild -- create test-branch2 --terminal iterm` — verify a new window opens alongside existing ones
3. Destroy test sessions: `cargo run -p kild -- destroy test-branch --force && cargo run -p kild -- destroy test-branch2 --force`

---

## Scope Boundaries

**IN SCOPE:**
- Fix `ITERM_SCRIPT` constant in `iterm.rs` to handle cold-start default window
- Add `activate` call for consistency with focus script pattern
- Update/add tests for new script structure

**OUT OF SCOPE (do not touch):**
- Terminal.app backend (different mechanism — `do script` doesn't have this issue)
- Ghostty backend (uses `open -na`, not AppleScript `tell application`)
- Other iTerm scripts (close, focus, hide — already work correctly)
- Session handler or terminal operations layer

---

## Metadata

- **Investigated by**: Claude
- **Timestamp**: 2026-02-09T12:00:00Z
- **Artifact**: `.claude/PRPs/issues/issue-271.md`
