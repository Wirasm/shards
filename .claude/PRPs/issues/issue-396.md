# Investigation: health dashboard reports daemon sessions as Crashed

**Issue**: #396 (https://github.com/Wirasm/kild/issues/396)
**Type**: BUG
**Investigated**: 2026-02-12T12:00:00Z

### Assessment

| Metric     | Value  | Reasoning                                                                                                  |
| ---------- | ------ | ---------------------------------------------------------------------------------------------------------- |
| Severity   | HIGH   | All daemon sessions show false Crashed status - major feature broken for daemon users, no workaround exists |
| Complexity | MEDIUM | 3 files need changes, daemon IPC infrastructure already exists, clear pattern to follow from list.rs        |
| Confidence | HIGH   | Root cause is obvious from code - PID-only detection skips daemon sessions entirely, fix pattern exists     |

---

## Problem Statement

`kild health` reports daemon-managed sessions as Crashed with a red X because the health check only uses PID-based process detection. Daemon sessions store a `daemon_session_id` instead of a PID, so `process_id()` returns `None` and `process_running` is always `false`. Similarly, `kild list` shows `Run(0/1)` for daemon sessions. The daemon IPC client (`daemon::client::get_session_status`) already exists and is used by `sync_daemon_session_status` in the list command, but neither the health module nor `determine_process_status` nor `format_process_status` use it.

---

## Analysis

### Root Cause

WHY: Daemon sessions show as Crashed in `kild health`
BECAUSE: `calculate_health_status()` returns `HealthStatus::Crashed` when `process_running = false`
Evidence: `crates/kild-core/src/health/operations.rs:25-27`

```rust
if !process_running {
    return HealthStatus::Crashed;
}
```

BECAUSE: `process_running` is always `false` for daemon sessions
Evidence: `crates/kild-core/src/health/handler.rs:67-77`

```rust
let running_pid = session
    .agents()
    .iter()
    .filter_map(|a| a.process_id())  // Returns None for daemon agents
    .find(|&pid| matches!(process::is_process_running(pid), Ok(true)));

let (process_metrics, process_running) = if let Some(pid) = running_pid {
    (get_metrics_for_pid(pid, &session.branch), true)
} else {
    (None, false)  // Always hits this branch for daemon sessions
};
```

ROOT CAUSE: `enrich_session_with_metrics()` has no daemon-aware code path. It only checks `process_id()` which is `None` for daemon agents (they store `daemon_session_id` instead).

The same root cause affects two other locations:
- `determine_process_status()` at `crates/kild-core/src/sessions/info.rs:78-138` - used by `SessionInfo::from_session()` for UI
- `format_process_status()` at `crates/kild/src/table.rs:117-137` - used by `kild list` display

### Evidence Chain

WHY: `kild health` shows Crashed
BECAUSE: `calculate_health_status(false, ...)` returns `Crashed`
Evidence: `crates/kild-core/src/health/operations.rs:25-27`

BECAUSE: `process_running = false` for daemon sessions
Evidence: `crates/kild-core/src/health/handler.rs:67-77` - `filter_map(|a| a.process_id())` yields nothing

BECAUSE: Daemon agents have `process_id: None`, `daemon_session_id: Some(...)`
Evidence: `crates/kild-core/src/sessions/types.rs:230-235` - invariant documented in comments

ROOT CAUSE: No daemon IPC query in the health/process detection path
Evidence: `daemon::client::get_session_status()` exists at `crates/kild-core/src/daemon/client.rs:285-363` but is unused by health

### Affected Files

| File                                          | Lines   | Action | Description                                                           |
| --------------------------------------------- | ------- | ------ | --------------------------------------------------------------------- |
| `crates/kild-core/src/health/handler.rs`      | 65-90   | UPDATE | Add daemon status query in `enrich_session_with_metrics()`            |
| `crates/kild-core/src/sessions/info.rs`       | 78-138  | UPDATE | Add daemon detection path in `determine_process_status()`             |
| `crates/kild/src/table.rs`                    | 117-137 | UPDATE | Add daemon detection path in `format_process_status()`                |

### Integration Points

- `crates/kild-core/src/daemon/client.rs:285` - `get_session_status()` already exists, returns `Option<SessionStatus>`
- `crates/kild-core/src/sessions/list.rs:51-123` - `sync_daemon_session_status()` already uses daemon IPC (pattern to mirror)
- `crates/kild-core/src/sessions/types.rs:380-382` - `AgentProcess::daemon_session_id()` accessor
- `crates/kild-protocol/src/types.rs:4-10` - `SessionStatus::Running` / `Stopped` / `Creating`

### Git History

- **Last modified health/handler.rs**: `fb4a52a` - "fix: use AgentStatus enum instead of String for type safety"
- **Last modified sessions/info.rs**: `10e327f` - "refactor: simplify code clarity in status and health modules"
- **Implication**: Original bug - daemon support was added after health/process detection, and those modules were never updated

---

## Implementation Plan

### Step 1: Add daemon detection to `enrich_session_with_metrics()` in health handler

**File**: `crates/kild-core/src/health/handler.rs`
**Lines**: 65-90
**Action**: UPDATE

**Current code:**

```rust
fn enrich_session_with_metrics(session: &sessions::types::Session) -> KildHealth {
    // Find first running agent for metrics (multi-agent path)
    let running_pid = session
        .agents()
        .iter()
        .filter_map(|a| a.process_id())
        .find(|&pid| matches!(process::is_process_running(pid), Ok(true)));

    let (process_metrics, process_running) = if let Some(pid) = running_pid {
        (get_metrics_for_pid(pid, &session.branch), true)
    } else {
        (None, false)
    };
    // ...
}
```

**Required change:**

After the PID-based check fails to find a running PID, check if any agent has a `daemon_session_id`. If so, query the daemon for status. If daemon reports Running, set `process_running = true`.

```rust
fn enrich_session_with_metrics(session: &sessions::types::Session) -> KildHealth {
    // Find first running agent for metrics (multi-agent path)
    let running_pid = session
        .agents()
        .iter()
        .filter_map(|a| a.process_id())
        .find(|&pid| matches!(process::is_process_running(pid), Ok(true)));

    let (process_metrics, process_running) = if let Some(pid) = running_pid {
        (get_metrics_for_pid(pid, &session.branch), true)
    } else {
        // Check daemon-managed agents if no PID-based process is running
        let daemon_running = session.agents().iter().any(|a| {
            a.daemon_session_id()
                .is_some_and(|id| is_daemon_session_running(id))
        });
        (None, daemon_running)
    };
    // ... rest unchanged
}
```

Add a helper function in the same file:

```rust
/// Check if a daemon-managed session is still running via IPC.
fn is_daemon_session_running(daemon_session_id: &str) -> bool {
    match crate::daemon::client::get_session_status(daemon_session_id) {
        Ok(Some(kild_protocol::SessionStatus::Running)) => true,
        Ok(_) => false,
        Err(e) => {
            warn!(
                event = "core.health.daemon_status_check_failed",
                daemon_session_id = daemon_session_id,
                error = %e,
            );
            false
        }
    }
}
```

**Why**: Health check needs to know if daemon PTY is running to avoid false Crashed status. Process metrics (CPU/memory) are unavailable for daemon sessions (no local PID), so `process_metrics` stays `None` but `process_running` becomes `true`.

---

### Step 2: Add daemon detection to `determine_process_status()` in sessions/info.rs

**File**: `crates/kild-core/src/sessions/info.rs`
**Lines**: 78-138
**Action**: UPDATE

**Current code:**

```rust
pub fn determine_process_status(session: &Session) -> ProcessStatus {
    let mut any_running = false;
    let mut any_unknown = false;

    for agent_proc in session.agents() {
        // Try PID-based detection first
        if let Some(pid) = agent_proc.process_id() {
            // ... PID check
        }

        // Fallback to window-based detection
        if let (Some(terminal_type), Some(window_id)) = (...) {
            // ... window check
        }
    }
    // ...
}
```

**Required change:**

Add a daemon detection path after the window-based fallback:

```rust
pub fn determine_process_status(session: &Session) -> ProcessStatus {
    let mut any_running = false;
    let mut any_unknown = false;

    for agent_proc in session.agents() {
        // Try PID-based detection first
        if let Some(pid) = agent_proc.process_id() {
            // ... existing PID check (unchanged)
        }

        // Fallback to window-based detection
        if let (Some(terminal_type), Some(window_id)) =
            (agent_proc.terminal_type(), agent_proc.terminal_window_id())
        {
            // ... existing window check (unchanged)
        }

        // Fallback to daemon-based detection
        if let Some(daemon_sid) = agent_proc.daemon_session_id() {
            match crate::daemon::client::get_session_status(daemon_sid) {
                Ok(Some(kild_protocol::SessionStatus::Running)) => {
                    any_running = true;
                    continue;
                }
                Ok(_) => continue,
                Err(e) => {
                    tracing::warn!(
                        event = "core.session.daemon_check_failed",
                        daemon_session_id = daemon_sid,
                        agent = agent_proc.agent(),
                        branch = &session.branch,
                        error = %e
                    );
                    any_unknown = true;
                    continue;
                }
            }
        }
    }
    // ... rest unchanged
}
```

**Why**: `determine_process_status()` is used by `SessionInfo::from_session()` which powers the UI. Without daemon detection, the UI also shows daemon sessions as Stopped.

---

### Step 3: Add daemon detection to `format_process_status()` in table.rs

**File**: `crates/kild/src/table.rs`
**Lines**: 117-137
**Action**: UPDATE

**Current code:**

```rust
fn format_process_status(session: &Session) -> String {
    let mut running = 0;
    let mut errored = 0;
    for agent_proc in session.agents() {
        if let Some(pid) = agent_proc.process_id() {
            match kild_core::process::is_process_running(pid) {
                Ok(true) => running += 1,
                Ok(false) => {}
                Err(_) => errored += 1,
            }
        }
    }
    // ...
}
```

**Required change:**

Add daemon status check when no PID is available:

```rust
fn format_process_status(session: &Session) -> String {
    let mut running = 0;
    let mut errored = 0;
    for agent_proc in session.agents() {
        if let Some(pid) = agent_proc.process_id() {
            match kild_core::process::is_process_running(pid) {
                Ok(true) => running += 1,
                Ok(false) => {}
                Err(_) => errored += 1,
            }
        } else if let Some(daemon_sid) = agent_proc.daemon_session_id() {
            match kild_core::daemon::client::get_session_status(daemon_sid) {
                Ok(Some(kild_protocol::SessionStatus::Running)) => running += 1,
                Ok(_) => {}
                Err(_) => errored += 1,
            }
        }
    }
    // ... rest unchanged
}
```

**Why**: `format_process_status()` produces the "Run(X/Y)" display in `kild list`. Without daemon detection, daemon agents are never counted as running, showing "Run(0/1)".

---

### Step 4: Add tests for daemon detection paths

**File**: `crates/kild-core/src/sessions/info.rs`
**Action**: UPDATE (add tests)

**Test cases to add:**

```rust
#[test]
fn test_determine_process_status_daemon_agent_no_pid() {
    // Daemon agent with no PID should not crash - daemon IPC will fail gracefully
    let mut session = make_session(PathBuf::from("/tmp/nonexistent"));
    let agent = make_daemon_agent("claude", "test_daemon_0");
    session.set_agents(vec![agent]);
    // Without a running daemon, should return Stopped (not crash)
    let status = determine_process_status(&session);
    assert!(matches!(status, ProcessStatus::Stopped | ProcessStatus::Unknown));
}
```

Add helper for daemon agent creation:

```rust
fn make_daemon_agent(agent: &str, daemon_session_id: &str) -> crate::sessions::types::AgentProcess {
    crate::sessions::types::AgentProcess::new(
        agent.to_string(),
        String::new(),
        None,  // No PID for daemon agents
        None,
        None,
        None,
        None,
        String::new(),
        "2024-01-01T00:00:00Z".to_string(),
        Some(daemon_session_id.to_string()),
    )
    .unwrap()
}
```

**File**: `crates/kild-core/src/health/operations.rs`
**Action**: No change needed - `calculate_health_status` tests already cover `process_running = true/false` scenarios. The fix is in the caller (`handler.rs`), not the calculation logic.

---

## Patterns to Follow

**From codebase - mirror `sync_daemon_session_status` pattern exactly:**

```rust
// SOURCE: crates/kild-core/src/sessions/list.rs:57-80
// Pattern for daemon IPC status check with graceful error handling
let daemon_sid = match session.latest_agent().and_then(|a| a.daemon_session_id()) {
    Some(id) => id.to_string(),
    None => return false,
};

let status = match crate::daemon::client::get_session_status(&daemon_sid) {
    Ok(s) => s,
    Err(e) => {
        warn!(
            event = "core.session.daemon_status_sync_failed",
            // ...
        );
        return false;
    }
};

if status == Some(kild_protocol::SessionStatus::Running) {
    return false;
}
```

**Key conventions:**
- Check `daemon_session_id()` on each agent, not just `latest_agent()`
- Daemon unreachable → treat as stopped (graceful degradation)
- IPC errors → `warn!` log, not `error!`

---

## Edge Cases & Risks

| Risk/Edge Case                    | Mitigation                                                                 |
| --------------------------------- | -------------------------------------------------------------------------- |
| Daemon not running                | `get_session_status` returns `Ok(None)` → treat as stopped (existing behavior) |
| Daemon IPC timeout                | 2-second timeout built into `get_session_status` → won't block health check    |
| Session not found in daemon       | `get_session_status` returns `Ok(None)` → treat as stopped                     |
| No process metrics for daemon     | `process_metrics` stays `None` → CPU/memory show as "-" (acceptable)           |
| Multiple daemon agents per session | Loop checks each agent, `any_running` pattern handles correctly               |
| Mixed PID + daemon agents         | PID check runs first, daemon check only if no PID found (performance)          |

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

1. Create a daemon session: `kild create test --no-agent --daemon`
2. Run `kild health` - should show status based on daemon state, not Crashed
3. Run `kild list` - should show `Run(1/1)` for daemon sessions, not `Run(0/1)`
4. Stop the daemon session and verify health shows Stopped/Crashed appropriately

---

## Scope Boundaries

**IN SCOPE:**

- Adding daemon IPC status checks to health handler (`enrich_session_with_metrics`)
- Adding daemon detection to `determine_process_status`
- Adding daemon detection to `format_process_status`
- Tests for daemon detection paths

**OUT OF SCOPE (do not touch):**

- `calculate_health_status()` logic - works correctly, the bug is in the caller
- `sync_daemon_session_status()` - already works correctly for its purpose (file sync)
- Daemon client implementation - already exists and works
- Process metrics (CPU/memory) for daemon sessions - requires daemon protocol extension (future work)
- Health storage/snapshots - not affected

---

## Metadata

- **Investigated by**: Claude
- **Timestamp**: 2026-02-12T12:00:00Z
- **Artifact**: `.claude/PRPs/issues/issue-396.md`
