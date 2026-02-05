# Investigation: focus fails after stop/open cycle due to stale window ID

**Issue**: #206 (https://github.com/Wirasm/kild/issues/206)
**Type**: BUG
**Investigated**: 2026-02-05T13:00:00Z

### Assessment

| Metric     | Value  | Reasoning                                                                                                |
| ---------- | ------ | -------------------------------------------------------------------------------------------------------- |
| Severity   | MEDIUM | Focus command fails but kild itself works; user can still interact with the terminal directly as workaround |
| Complexity | LOW    | Fix is isolated to the Ghostty focus_window method and potentially stop's close_window reliability       |
| Confidence | HIGH   | Root cause clearly identified through code tracing; the PID lookup and title search mechanisms are both verifiably fragile |

---

## Problem Statement

`kild focus <branch>` fails after a stop/open cycle because the Ghostty `focus_window` implementation has two fragile lookup mechanisms that can both fail after a terminal window is replaced:

1. **PID-based lookup** filters for processes whose executable name contains "ghostty", but the process with the window title in its command line is `sh` (not `ghostty`), so it's always filtered out.
2. **Title-based lookup** via AppleScript can fail when the running agent overwrites the ANSI terminal title, or when the old (dead) window wasn't closed by `stop` and interferes.

---

## Analysis

### Root Cause / Change Rationale

The window ID is correctly regenerated and persisted after stop/open (confirmed by code trace). The bug is in how `focus_window` finds the window.

### Evidence Chain

WHY: `kild focus` fails with "Ghostty window 'kild-...' not found"
--> BECAUSE: Both PID-based and title-based window lookup fail

WHY: PID-based lookup fails
--> BECAUSE: `find_ghostty_pid_by_session(window_id)` calls `pgrep -f` which finds the `sh -c "printf '\033]2;kild-...\007' && ..."` process, but `is_ghostty_process(pid)` filters it out because the executable is `sh`, not `ghostty`
Evidence: `crates/kild-core/src/terminal/backends/ghostty.rs:62` - `pids.into_iter().find(|&pid| is_ghostty_process(pid))`
Evidence: `crates/kild-core/src/terminal/backends/ghostty.rs:82-101` - `is_ghostty_process` checks `ps -o comm=` for "ghostty"

WHY: Title-based lookup fails
--> BECAUSE: The Ghostty window title can be overwritten by the running agent (e.g., `claude` sets its own terminal title via ANSI escape), AND `stop`'s `close_window` may not reliably close the old Ghostty window
Evidence: `crates/kild-core/src/terminal/backends/ghostty.rs:417-431` - AppleScript checks `name of w contains "window_id"`
Evidence: `crates/kild-core/src/terminal/backends/ghostty.rs:301-304` - `pkill -f "Ghostty.*{escaped_id}"` pattern requires "Ghostty" in the same process command line as the window_id, which doesn't match the `sh -c` process

ROOT CAUSE: The `close_window` pkill pattern `"Ghostty.*{window_id}"` doesn't match any process because:
- The Ghostty.app process doesn't have the window_id in its command line
- The `sh -c` process has the window_id but doesn't have "Ghostty" in its command line

This means `stop` **never actually closes the Ghostty window**. After `open` spawns a new window with the same title, there are potentially two windows. The title-based focus search finds the old dead window first and reports "not found" (since the old window may have a different title after the process exited).

Evidence: `crates/kild-core/src/terminal/backends/ghostty.rs:298-304`:
```rust
let escaped_id = escape_regex(id);
let result = std::process::Command::new("pkill")
    .arg("-f")
    .arg(format!("Ghostty.*{}", escaped_id))
    .output();
```

### Affected Files

| File | Lines | Action | Description |
| --- | --- | --- | --- |
| `crates/kild-core/src/terminal/backends/ghostty.rs` | 282-333 | UPDATE | Fix `close_window` to reliably close the Ghostty terminal window |
| `crates/kild-core/src/terminal/backends/ghostty.rs` | 343-485 | UPDATE | Improve `focus_window` PID lookup to not filter out `sh` processes |
| `crates/kild-core/src/terminal/backends/ghostty.rs` | 17-78 | UPDATE | Fix `find_ghostty_pid_by_session` to find the shell process hosting the window |
| `crates/kild-core/src/terminal/backends/ghostty.rs` | 586+ | UPDATE | Add tests for the fix |

### Integration Points

- `crates/kild-core/src/sessions/handler.rs:1245` calls `close_terminal` during stop
- `crates/kild/src/commands.rs:898` calls `focus_terminal` in the focus command
- `crates/kild-core/src/terminal/handler.rs:363-371` delegates to `operations::focus_terminal_window`
- `crates/kild-core/src/terminal/operations.rs:251-262` resolves backend and calls `backend.focus_window`

### Git History

- **Introduced**: `1aa6d9d9` (2026-01-26) - original Ghostty backend implementation
- **PID focus added**: `c31229d3` (2026-01-28) - "Fix: Ghostty focus uses PID-based lookup (#95)"
- **Per-agent isolation**: `ad0bf70` (2026-02-05) - "fix: per-agent PID file and window title isolation (#232)"
- **Implication**: Original bug in `close_window` pkill pattern; PID-based focus has never worked for `sh`-hosted windows

---

## Implementation Plan

### Step 1: Fix `close_window` to reliably close the Ghostty terminal

**File**: `crates/kild-core/src/terminal/backends/ghostty.rs`
**Lines**: 282-333
**Action**: UPDATE

**Current code:**
```rust
let escaped_id = escape_regex(id);
let result = std::process::Command::new("pkill")
    .arg("-f")
    .arg(format!("Ghostty.*{}", escaped_id))
    .output();
```

**Required change:**
Change the `pkill` pattern to match the actual process that contains the window_id. The shell process command line contains the window_id (in the ANSI escape sequence), so match that directly without requiring "Ghostty" prefix:

```rust
let escaped_id = escape_regex(id);
// Kill the shell process that hosts our window (has window_id in its command line
// from the ANSI title escape sequence)
let result = std::process::Command::new("pkill")
    .arg("-f")
    .arg(&escaped_id)
    .output();
```

**Why**: The current pattern `"Ghostty.*{id}"` never matches any process because the Ghostty app process doesn't contain the window_id, and the shell process doesn't contain "Ghostty". Removing the "Ghostty" prefix allows `pkill` to find and kill the `sh -c "printf '\033]2;{window_id}\007' && ..."` process, which closes the Ghostty window.

**Risk**: The simpler pattern could match unrelated processes that happen to contain the window_id string. The window_id format `kild-{project_hash}-{branch}_{index}` is specific enough to avoid false matches, but we should document this assumption.

---

### Step 2: Improve `find_ghostty_pid_by_session` to find shell-hosted windows

**File**: `crates/kild-core/src/terminal/backends/ghostty.rs`
**Lines**: 17-78
**Action**: UPDATE

**Current code:**
```rust
// Find the first Ghostty process among candidates by checking each process's
// executable name (via ps -o comm=) for "ghostty"
let found_pid = pids.into_iter().find(|&pid| is_ghostty_process(pid));
```

**Required change:**
Instead of filtering for Ghostty processes only, find the shell process that hosts our window. The correct process is the one whose parent is a Ghostty process, or simply the first process whose command line contains the window_id (since the window_id is unique enough):

```rust
// The window_id is embedded in the shell command line via ANSI escape.
// Find any process with this ID - it will be the sh process hosting our window.
// Don't filter for Ghostty executable since the hosting process is sh, not ghostty.
let found_pid = pids.first().copied();
```

Then in `focus_by_pid`, instead of focusing by the shell PID (which wouldn't raise the Ghostty window), we should find the Ghostty **parent** process or use the AppleScript title-based approach. Actually, a better approach: since we found the PID of the `sh` process, we can get its parent PID (which should be the Ghostty process) and focus that.

Alternative simpler approach: Remove the `is_ghostty_process` filter and instead verify the found PID's parent is Ghostty before attempting `focus_by_pid` on the parent:

```rust
// Find a candidate process and check if its parent is Ghostty
let found_pid = pids.into_iter().find_map(|pid| {
    get_parent_pid(pid).filter(|&ppid| is_ghostty_process(ppid))
});
```

**Why**: The current logic assumes the process with the window_id in its command line IS a Ghostty process. But Ghostty spawns `sh -c "..."` to run the command, so the window_id lives in `sh`'s command line, not Ghostty's.

---

### Step 3: Add helper to get parent PID

**File**: `crates/kild-core/src/terminal/backends/ghostty.rs`
**Action**: UPDATE (add function)

**Required change:**
```rust
/// Get the parent PID of a process.
#[cfg(target_os = "macos")]
fn get_parent_pid(pid: u32) -> Option<u32> {
    let output = std::process::Command::new("ps")
        .args(["-o", "ppid=", "-p", &pid.to_string()])
        .output()
        .ok()?;

    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .ok()
}
```

**Why**: Needed to traverse from the `sh` process (which has the window_id in its command line) to its parent Ghostty process (which owns the window and can be focused).

---

### Step 4: Update `focus_by_pid` call to use Ghostty parent PID

**File**: `crates/kild-core/src/terminal/backends/ghostty.rs`
**Lines**: 385-412
**Action**: UPDATE

**Current code:**
```rust
if let Some(pid) = find_ghostty_pid_by_session(window_id) {
    match focus_by_pid(pid) {
        Ok(()) => return Ok(()),
        // ...
    }
}
```

**Required change:**
After `find_ghostty_pid_by_session` is updated to return the Ghostty parent PID (from Step 2), this code works unchanged. The Ghostty PID can be focused via `focus_by_pid` which uses System Events `unix id` to raise the process's window.

**Why**: `focus_by_pid` raises the window of the process identified by the unix PID. When passed the Ghostty process PID (not the `sh` child), it correctly raises the Ghostty window.

---

### Step 5: Add tests

**File**: `crates/kild-core/src/terminal/backends/ghostty.rs`
**Lines**: 586+
**Action**: UPDATE

**Test cases to add:**

```rust
#[cfg(target_os = "macos")]
#[test]
fn test_get_parent_pid_returns_none_for_nonexistent_process() {
    let result = get_parent_pid(99999999);
    // Either returns None or returns a PID - shouldn't panic
    // Can't assert specific result without a running process
}

#[cfg(target_os = "macos")]
#[test]
fn test_get_parent_pid_for_current_process() {
    let current_pid = std::process::id();
    let parent = get_parent_pid(current_pid);
    // Current process should have a parent
    assert!(parent.is_some(), "Current process should have a parent PID");
}

#[test]
fn test_close_window_pkill_pattern_no_ghostty_prefix() {
    // Verify the pkill pattern matches the window_id without requiring "Ghostty" prefix
    let window_id = "kild-project123-my-branch_0";
    let escaped = escape_regex(window_id);
    // Pattern should be just the escaped window_id, not "Ghostty.*{id}"
    assert_eq!(escaped, "kild-project123-my-branch_0");
    assert!(!escaped.contains("Ghostty"));
}
```

---

## Patterns to Follow

**From codebase - mirror these exactly:**

```rust
// SOURCE: crates/kild-core/src/terminal/backends/ghostty.rs:82-101
// Pattern for checking process attributes via ps
fn is_ghostty_process(pid: u32) -> bool {
    match std::process::Command::new("ps")
        .args(["-o", "comm=", "-p", &pid.to_string()])
        .output()
    {
        Ok(output) => {
            let comm = String::from_utf8_lossy(&output.stdout);
            comm.to_lowercase().contains("ghostty")
        }
        Err(e) => {
            debug!(/* ... */);
            false
        }
    }
}
```

---

## Edge Cases & Risks

| Risk/Edge Case | Mitigation |
| --- | --- |
| `pkill` without "Ghostty" prefix could match unrelated processes | Window IDs are highly specific (`kild-{hash}-{branch}_{index}`), making false matches extremely unlikely |
| `get_parent_pid` returns wrong parent after process tree changes | This is a best-effort lookup; title-based fallback still exists |
| Old session files with top-level `terminal_window_id` (pre-f1a4972) | Serde ignores unknown fields; `agents` vec is source of truth |
| Multiple Ghostty windows with same title after incomplete stop | Fix to `close_window` ensures old window is closed; PID-based focus targets correct parent |
| Agent that rewrites terminal title (e.g., `claude`) | PID-based focus works regardless of title since it uses command line matching, not window title |

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

1. `kild create test-branch` - verify session created with window_id
2. `kild focus test-branch` - verify focus works
3. `kild stop test-branch` - verify terminal window closes
4. `kild open test-branch` - verify new terminal opens
5. `kild focus test-branch` - verify focus works on the new window (THIS IS THE BUG SCENARIO)
6. Wait for agent to change terminal title, then `kild focus test-branch` - verify PID-based focus still works

---

## Scope Boundaries

**IN SCOPE:**
- Fix `close_window` pkill pattern to actually kill the correct process
- Fix `find_ghostty_pid_by_session` to traverse from `sh` to Ghostty parent process
- Add `get_parent_pid` helper
- Add tests for the new behavior

**OUT OF SCOPE (do not touch):**
- Session persistence logic (works correctly)
- `open_session` window_id capture (works correctly)
- Other terminal backends (iTerm, Terminal.app, Alacritty)
- UI-layer code
- Agent command building

---

## Metadata

- **Investigated by**: Claude
- **Timestamp**: 2026-02-05T13:00:00Z
- **Artifact**: `.claude/PRPs/issues/issue-206.md`
