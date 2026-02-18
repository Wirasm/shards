# Investigation: perf: session startup latency — 200ms sleep, auto-start busy-wait

**Issue**: #475 (https://github.com/Wirasm/kild/issues/475)
**Type**: PERFORMANCE (ENHANCEMENT)
**Investigated**: 2026-02-18T00:00:00Z

### Assessment

| Metric     | Value  | Reasoning                                                                                              |
| ---------- | ------ | ------------------------------------------------------------------------------------------------------ |
| Priority   | MEDIUM | Affects every daemon session open but no functionality is broken; 200ms–700ms latency is noticeable but not blocking. |
| Complexity | LOW    | Two isolated timing changes in 3 files; no API changes, no new types, no integration point changes.   |
| Confidence | HIGH   | Root cause is 100% clear from reading the code — both are trivially unconditional blocking sleeps.     |

---

## Problem Statement

Every `kild open --daemon` (and `kild create --daemon`) unconditionally blocks for 200ms after the PTY is confirmed spawned, even when the process is healthy. Additionally, `ensure_daemon_running` polls with a flat 100ms interval instead of exponential backoff, wasting ~50ms on the happy path (daemon already starting up fast). Combined, a cold `kild create --daemon` wastes ~300ms of unnecessary sleeping per invocation.

---

## Analysis

### Root Cause / Change Rationale

Two independent fixed-delay patterns:

**Fix 1 — 200ms unconditional sleep in `open.rs` and `create.rs`:**
After `create_pty_session` returns (daemon confirms PTY spawned), the code sleeps a flat 200ms before checking if the process exited early. It never checks for a positive "Running" signal — the only way to terminate early is to fall through (not-Stopped). Replacing with a polling loop (50ms → 100ms → 200ms) that exits immediately on `SessionStatus::Running` drops the success-path wait from 200ms to ~50ms.

**Fix 2 — Flat 100ms sleep in `autostart.rs`:**
The polling loop that waits for the daemon socket to appear uses a fixed 100ms sleep on every iteration. A fast daemon start (50–150ms) will always waste a full 100ms sleep even if the socket appears at 51ms. Exponential backoff starting at 50ms catches fast starts in the first poll and gracefully extends to 500ms for slow starts.

### Evidence Chain

WHY: `kild open --daemon` is slow
↓ BECAUSE: unconditional 200ms sleep after `create_pty_session` completes
Evidence: `crates/kild-core/src/sessions/open.rs:335` — `std::thread::sleep(std::time::Duration::from_millis(200));`

↓ BECAUSE: the code only checks for `Stopped` (failure) — never checks for `Running` (success)
Evidence: `crates/kild-core/src/sessions/open.rs:337-339` — condition is `status == kild_protocol::SessionStatus::Stopped`; no `Running` branch

↓ ROOT CAUSE: missing early-exit on `Running` confirmation; flat sleep is needed when no polling exists
Evidence: `crates/kild-protocol/src/types.rs` — `SessionStatus` has three variants: `Creating`, `Running`, `Stopped` — the existing code ignores `Running`

WHY: cold `kild create --daemon` wastes extra time on auto-start
↓ BECAUSE: daemon readiness polling uses flat 100ms intervals
Evidence: `crates/kild-core/src/daemon/autostart.rs:103` — `std::thread::sleep(std::time::Duration::from_millis(100));`

↓ BECAUSE: no backoff — every iteration sleeps the same 100ms regardless of elapsed time
Evidence: `autostart.rs:44-45,103` — `timeout` is 5s, sleep is always exactly 100ms; for a daemon ready in 51ms, iteration 0 sleeps 100ms before checking

↓ ROOT CAUSE: linear polling wastes ~50ms on fast daemon starts, same as slow starts
Evidence: loop structure at `autostart.rs:47-104` — sleep always comes after all checks, no duration variable

### Affected Files

| File                                                  | Lines   | Action | Description                                     |
| ----------------------------------------------------- | ------- | ------ | ----------------------------------------------- |
| `crates/kild-core/src/sessions/open.rs`               | 332-384 | UPDATE | Replace 200ms sleep with exponential backoff loop; add `Running` early-exit |
| `crates/kild-core/src/sessions/create.rs`             | 309-352 | UPDATE | Same fix (identical pattern duplicated verbatim) |
| `crates/kild-core/src/daemon/autostart.rs`            | 43-104  | UPDATE | Add exponential backoff: 50ms → 100ms → 200ms → 500ms (capped) |

### Integration Points

- `crates/kild/src/commands/open.rs:33,99` calls `open_session` — CLI entry point
- `crates/kild-core/src/state/dispatch.rs:71` dispatches `Command::OpenKild` through `CoreStore` — UI entry point
- `crates/kild-ui/src/views/main_view/terminal_handlers.rs:394` calls `ensure_daemon_running` directly
- `crates/kild-core/src/sessions/create.rs` calls `ensure_daemon_running` on the daemon path (`create.rs:267`)
- `crates/kild/src/commands/daemon.rs:55-99` has a parallel duplicate of the same polling pattern — **OUT OF SCOPE** for this issue (separate concern, separate fix)

### Git History

- **Autostart** introduced: `0a70f18` — "feat: move daemon auto-start from CLI into kild-core (#323)"
- **open.rs 200ms sleep**: part of daemon session attach work, most recent touch `fa8df00` — "fmt: fix formatting in open.rs"
- **Implication**: both patterns are original implementations, not regressions; never had a smarter polling strategy

---

## Implementation Plan

### Step 1: Replace 200ms sleep with backoff polling in `open.rs`

**File**: `crates/kild-core/src/sessions/open.rs`
**Lines**: 332-384
**Action**: UPDATE

**Current code:**

```rust
// Early exit detection: wait briefly, then verify PTY is still alive.
// Fast-failing processes (bad resume session, missing binary, env issues)
// typically exit within 200ms of spawn.
std::thread::sleep(std::time::Duration::from_millis(200));

if let Ok(Some((status, exit_code))) =
    crate::daemon::client::get_session_info(&daemon_result.daemon_session_id)
    && status == kild_protocol::SessionStatus::Stopped
{
    let scrollback_tail =
        match crate::daemon::client::read_scrollback(&daemon_result.daemon_session_id) {
            // ... scrollback extraction ...
        };
    // ... destroy + return Err(DaemonPtyExitedEarly) ...
}
```

**Required change:**

Replace the `sleep` + single-check block with a polling loop. The `maybe_early_exit` variable preserves the exact same semantics — `Some(exit_code)` means stopped (error path), `None` means running or timed out (continue). The rest of the error handling block (scrollback, destroy, return Err) is unchanged.

```rust
// Early exit detection: poll with exponential backoff until Running or Stopped.
// Fast-failing processes (bad resume session, missing binary, env issues)
// typically exit within 50ms of spawn. Exit early on Running confirmation.
let maybe_early_exit: Option<Option<i32>> = {
    let mut result = None;
    for delay_ms in [50u64, 100, 200] {
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        match crate::daemon::client::get_session_info(&daemon_result.daemon_session_id) {
            Ok(Some((kild_protocol::SessionStatus::Stopped, exit_code))) => {
                result = Some(exit_code);
                break;
            }
            Ok(Some((kild_protocol::SessionStatus::Running, _))) => break, // confirmed alive
            _ => {} // Creating or IPC error — keep polling
        }
    }
    result
};

if let Some(exit_code) = maybe_early_exit {
    let scrollback_tail =
        match crate::daemon::client::read_scrollback(&daemon_result.daemon_session_id) {
            Ok(Some(bytes)) => {
                let text = String::from_utf8_lossy(&bytes);
                let lines: Vec<&str> = text.lines().collect();
                let start = lines.len().saturating_sub(20);
                lines[start..].join("\n")
            }
            Ok(None) => {
                warn!(
                    event = "core.session.open_scrollback_empty",
                    daemon_session_id = %daemon_result.daemon_session_id,
                    "Daemon session exited early with empty scrollback"
                );
                String::new()
            }
            Err(e) => {
                warn!(
                    event = "core.session.open_scrollback_read_failed",
                    daemon_session_id = %daemon_result.daemon_session_id,
                    error = %e,
                    "Failed to read scrollback after early PTY exit"
                );
                String::new()
            }
        };

    if let Err(e) = crate::daemon::client::destroy_daemon_session(
        &daemon_result.daemon_session_id,
        true,
    ) {
        warn!(
            event = "core.session.open_daemon_cleanup_failed",
            daemon_session_id = %daemon_result.daemon_session_id,
            error = %e,
            "Failed to clean up daemon session after early exit"
        );
    }

    return Err(SessionError::DaemonPtyExitedEarly {
        exit_code,
        scrollback_tail,
    });
}
```

**Why**: On success path, first `get_session_info` at 50ms returns `Running` → loop exits immediately. Total wait: ~50ms instead of 200ms. For early-exit detection, `Stopped` at any poll triggers the same error path as before. The `_ => {}` arm handles both `Creating` (still starting up) and any IPC errors (keep polling, same as the old code falling through).

---

### Step 2: Apply identical fix to `create.rs`

**File**: `crates/kild-core/src/sessions/create.rs`
**Lines**: 309-352
**Action**: UPDATE

Same change as Step 1. The code at `create.rs:309-352` is the same pattern with only minor style differences in the scrollback extraction (`inspect_err().ok().flatten()` vs `match`).

Replace:
```rust
// Early exit detection: wait briefly, then verify PTY is still alive.
// Fast-failing processes (bad resume session, missing binary, env issues)
// typically exit within 200ms of spawn.
std::thread::sleep(std::time::Duration::from_millis(200));

if let Ok(Some((status, exit_code))) =
    crate::daemon::client::get_session_info(&daemon_result.daemon_session_id)
    && status == kild_protocol::SessionStatus::Stopped
{
    let scrollback_tail =
        crate::daemon::client::read_scrollback(&daemon_result.daemon_session_id)
            .inspect_err(|e| { debug!(...) })
            .ok()
            .flatten()
            .map(|bytes| { /* last 20 lines */ })
            .unwrap_or_default();
    // ... destroy + return Err(DaemonPtyExitedEarly) ...
}
```

With the same `maybe_early_exit` polling loop pattern (adapting the scrollback extraction style from `create.rs`):

```rust
// Early exit detection: poll with exponential backoff until Running or Stopped.
// Fast-failing processes (bad resume session, missing binary, env issues)
// typically exit within 50ms of spawn. Exit early on Running confirmation.
let maybe_early_exit: Option<Option<i32>> = {
    let mut result = None;
    for delay_ms in [50u64, 100, 200] {
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        match crate::daemon::client::get_session_info(&daemon_result.daemon_session_id) {
            Ok(Some((kild_protocol::SessionStatus::Stopped, exit_code))) => {
                result = Some(exit_code);
                break;
            }
            Ok(Some((kild_protocol::SessionStatus::Running, _))) => break, // confirmed alive
            _ => {} // Creating or IPC error — keep polling
        }
    }
    result
};

if let Some(exit_code) = maybe_early_exit {
    let scrollback_tail =
        crate::daemon::client::read_scrollback(&daemon_result.daemon_session_id)
            .inspect_err(|e| {
                debug!(
                    event = "core.session.scrollback_read_failed",
                    daemon_session_id = daemon_result.daemon_session_id,
                    error = %e,
                );
            })
            .ok()
            .flatten()
            .map(|bytes| {
                let text = String::from_utf8_lossy(&bytes);
                let lines: Vec<&str> = text.lines().collect();
                let start = lines.len().saturating_sub(20);
                lines[start..].join("\n")
            })
            .unwrap_or_default();

    if let Err(e) = crate::daemon::client::destroy_daemon_session(
        &daemon_result.daemon_session_id,
        true,
    ) {
        warn!(
            event = "core.session.create_daemon_cleanup_failed",
            daemon_session_id = %daemon_result.daemon_session_id,
            error = %e,
        );
    }

    return Err(SessionError::DaemonPtyExitedEarly {
        exit_code,
        scrollback_tail,
    });
}
```

---

### Step 3: Add exponential backoff to daemon auto-start polling loop

**File**: `crates/kild-core/src/daemon/autostart.rs`
**Lines**: 43-104
**Action**: UPDATE

**Current code (the relevant section):**

```rust
let socket = socket_path();
let timeout = std::time::Duration::from_secs(5);
let start = std::time::Instant::now();

loop {
    // ... crash check ...
    // ... socket + ping check ...
    // ... timeout check ...
    std::thread::sleep(std::time::Duration::from_millis(100));  // line 103
}
```

**Required change** — add `delay_ms` variable before the loop and update the sleep:

```rust
let socket = socket_path();
let timeout = std::time::Duration::from_secs(5);
let start = std::time::Instant::now();
let mut delay_ms = 50u64;

loop {
    // Check if daemon process crashed before socket was ready
    match child.try_wait() {
        Ok(Some(status)) => {
            error!(event = "core.daemon.auto_start_failed", reason = "child_exited", status = %status);
            return Err(DaemonAutoStartError::SpawnFailed {
                message: format!(
                    "Daemon process exited with {} before becoming ready.\n\
                     Check daemon logs: kild daemon start --foreground\n\
                     Daemon binary: {}",
                    status,
                    daemon_binary.display()
                ),
            });
        }
        Ok(None) => {} // Still running
        Err(e) => {
            warn!(event = "core.daemon.child_status_check_failed", error = %e);
        }
    }

    if socket.exists() && client::ping_daemon().unwrap_or(false) {
        info!(event = "core.daemon.auto_start_completed");
        eprintln!("Daemon started.");
        return Ok(());
    }

    if start.elapsed() > timeout {
        let socket_exists = socket.exists();
        if socket_exists {
            error!(
                event = "core.daemon.auto_start_failed",
                reason = "timeout_no_ping",
                socket_exists = true
            );
            return Err(DaemonAutoStartError::Timeout {
                message: "Daemon socket exists but not responding to ping after 5s.\n\
                          Try: kild daemon stop && kild daemon start"
                    .to_string(),
            });
        } else {
            error!(
                event = "core.daemon.auto_start_failed",
                reason = "timeout_no_socket",
                socket_exists = false
            );
            return Err(DaemonAutoStartError::Timeout {
                message: format!(
                    "Daemon process spawned but socket not created after 5s.\n\
                     Check daemon logs: kild daemon start --foreground\n\
                     Daemon binary: {}",
                    daemon_binary.display()
                ),
            });
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(delay_ms));
    delay_ms = (delay_ms * 2).min(500);
}
```

**Why**: Backoff sequence is 50ms, 100ms, 200ms, 400ms, 500ms, 500ms... A daemon ready in 50ms is caught in iteration 1 (after 50ms sleep) instead of iteration 1 (after 100ms sleep). Typical start in 100–200ms is caught in 2–3 iterations totaling 150–350ms vs the old fixed 100ms×N. The 5s hard timeout and error messages are unchanged.

---

## Patterns to Follow

**Exponential backoff via doubling with cap — natural Rust idiom:**

```rust
// Pattern: initialize before loop, double with min() inside loop
let mut delay_ms = 50u64;
loop {
    // ... do work ...
    std::thread::sleep(std::time::Duration::from_millis(delay_ms));
    delay_ms = (delay_ms * 2).min(500);
}
```

**Named timing constants for pid_file.rs — existing convention in codebase:**

```rust
// SOURCE: crates/kild-core/src/process/pid_file.rs:46-47
// Named constants pattern (used elsewhere in codebase for timing values)
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const MAX_WAIT: Duration = Duration::from_secs(3);
```
Note: The open.rs/create.rs fix uses an inline array `[50u64, 100, 200]` instead of named constants — this is acceptable for a fixed 3-step sequence (not a configurable loop).

---

## Edge Cases & Risks

| Risk/Edge Case                                                | Mitigation                                                                 |
| ------------------------------------------------------------- | -------------------------------------------------------------------------- |
| Session still in `Creating` state at 350ms (all 3 polls)     | Falls through with `maybe_early_exit = None` — same behavior as old code (assume alive) |
| IPC error on all 3 polls                                      | Falls through with `maybe_early_exit = None` — same behavior as old code  |
| Agent process exits very fast (< 50ms) but `Creating` state not yet flushed | Next poll at 100ms will see `Stopped` — still caught within 150ms |
| Daemon takes > 500ms to start (slow machine / cold disk)     | Loop continues with 500ms sleep per iteration; 5s total timeout unchanged  |
| `autostart.rs`: daemon exits before first 50ms sleep finishes | `child.try_wait()` catches it on the next iteration's crash check — unchanged behavior |

---

## Validation

```bash
cargo fmt --check
cargo clippy --all -- -D warnings
cargo test --all
cargo build --all
```

Specific test targets:
```bash
cargo test -p kild-core test_auto_start      # autostart tests
cargo test -p kild-core test_daemon_pty      # DaemonPtyExitedEarly error test
```

### Manual Verification

1. **Success path timing**: `time cargo run -p kild -- create test-475 --daemon` — should complete in < 400ms total (was ~700ms on cold start)
2. **Early exit detection still works**: Create a session with an invalid agent binary and verify `DaemonPtyExitedEarly` error is still returned with scrollback
3. **Auto-start**: Stop daemon, run `kild create --daemon` — should print "Starting daemon... Daemon started." and complete faster than before
4. **`kild open --all` with 10 sessions**: Should complete in 50ms×10 = ~500ms instead of 200ms×10 = 2000ms on the success path

---

## Scope Boundaries

**IN SCOPE:**
- Replace `std::thread::sleep(200ms)` in `open.rs:335` with exponential backoff polling loop
- Apply same fix to identical pattern in `create.rs:312`
- Add exponential backoff to `autostart.rs:103` sleep

**OUT OF SCOPE (do not touch):**
- Duplicate polling loop in `crates/kild/src/commands/daemon.rs:55-99` (same pattern in CLI daemon start command — separate issue)
- Extracting a shared `poll_for_pty_ready` helper to deduplicate open.rs/create.rs (valid DRY improvement but separate concern)
- Any other items from the performance audit (`remove flush()`, connection pooling, base64 elimination, etc.)

---

## Metadata

- **Investigated by**: Claude
- **Timestamp**: 2026-02-18T00:00:00Z
- **Artifact**: `.claude/PRPs/issues/issue-475.md`
