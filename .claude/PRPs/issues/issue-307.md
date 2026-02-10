# Investigation: Flaky test: test_ping_daemon_returns_false_when_not_running

**Issue**: #307 (https://github.com/Wirasm/kild/issues/307)
**Type**: BUG
**Investigated**: 2026-02-10

### Assessment

| Metric     | Value  | Reasoning                                                                                              |
| ---------- | ------ | ------------------------------------------------------------------------------------------------------ |
| Severity   | MEDIUM | Test passes in CI (no daemon running) but fails on developer machines with active daemon; workaround is stopping daemon before running tests |
| Complexity | LOW    | 1 file to change (`client.rs`), isolated test fix with no integration impact                           |
| Confidence | HIGH   | Root cause is clearly identified in the issue itself and confirmed by code reading — `ping_daemon()` uses hardcoded socket path |

---

## Problem Statement

The test `test_ping_daemon_returns_false_when_not_running` in `crates/kild-core/src/daemon/client.rs:418-427` fails when the kild daemon is actually running on the developer's machine. It calls `ping_daemon()` which connects to the real socket at `~/.kild/daemon.sock`, but assumes no daemon is running. A second test `test_get_session_status_returns_none_when_daemon_not_running` has the same problem.

---

## Analysis

### Root Cause

WHY: Test `test_ping_daemon_returns_false_when_not_running` fails with assertion error
↓ BECAUSE: `ping_daemon()` returns `Ok(true)` instead of `Ok(false)`
Evidence: `client.rs:281` — returns `Ok(true)` when daemon responds to ping

↓ BECAUSE: `connect()` succeeds connecting to a real running daemon
Evidence: `client.rs:269-270` — `connect(&socket_path)` returns `Ok(s)` when socket exists

↓ ROOT CAUSE: `ping_daemon()` hardcodes socket path via `crate::daemon::socket_path()` with no test override
Evidence: `client.rs:260` — `let socket_path = crate::daemon::socket_path();`
Evidence: `daemon/mod.rs:6-8` — `socket_path()` always returns `~/.kild/daemon.sock`

### Evidence Chain

The `connect()` function at `client.rs:41` already accepts a `&Path` parameter, proving the lower layer is testable. The problem is that `ping_daemon()` at `client.rs:259` and `get_session_status()` at `client.rs:296` call `socket_path()` internally, bypassing the parameterized `connect()`.

The existing test `test_connect_returns_not_running_for_missing_socket` at `client.rs:405-416` works correctly because it passes a non-existent path directly to `connect()`:
```rust
let missing = Path::new("/tmp/kild-test-nonexistent-socket.sock");
let result = connect(missing);
```

### Affected Files

| File                                   | Lines   | Action | Description                                                    |
| -------------------------------------- | ------- | ------ | -------------------------------------------------------------- |
| `crates/kild-core/src/daemon/client.rs` | 418-427 | UPDATE | Fix flaky `test_ping_daemon_returns_false_when_not_running`    |
| `crates/kild-core/src/daemon/client.rs` | 389-403 | UPDATE | Fix flaky `test_get_session_status_returns_none_when_daemon_not_running` |

### Integration Points

- `ping_daemon()` is called by CLI commands in `crates/kild/src/commands/helpers.rs:90,118` and `crates/kild/src/commands/daemon.rs:21,59,117`
- `get_session_status()` is called by session lifecycle handlers
- None of these callers are affected — only test code changes

### Git History

- **Introduced**: `6f1cfa7` - "feat: add kild-daemon crate with PTY ownership, IPC server, and session persistence (#294)"
- **Last modified**: `7d5358f` - "fix: daemon PTY commands exit immediately (#302) (#303)"
- **Implication**: Original bug — tests were written assuming CI-only execution where no daemon runs

---

## Implementation Plan

### Step 1: Fix `test_ping_daemon_returns_false_when_not_running`

**File**: `crates/kild-core/src/daemon/client.rs`
**Lines**: 418-427
**Action**: UPDATE

The test should use a non-existent socket path directly via the `connect()` function to verify the "not running" behavior, rather than going through `ping_daemon()` which uses the real socket. Since what we're actually testing is that the daemon client handles the "not running" case correctly (returns `Ok(false)` / no error), we can replicate `ping_daemon()`'s logic with a known-missing socket path.

**Current code:**
```rust
#[test]
fn test_ping_daemon_returns_false_when_not_running() {
    // When the daemon socket doesn't exist, ping should return Ok(false)
    let result = ping_daemon();
    assert!(result.is_ok());
    assert!(
        !result.unwrap(),
        "Ping should return false when daemon is not running"
    );
}
```

**Required change:**

Use a temp directory to guarantee the socket doesn't exist, and test via `connect()` directly (which is the private function that `ping_daemon()` delegates to). The key behavior being tested is that `connect()` returns `NotRunning` for a missing socket, which `ping_daemon()` maps to `Ok(false)`.

```rust
#[test]
fn test_ping_daemon_returns_false_when_not_running() {
    // Use a temp directory to guarantee no socket exists, isolating from a running daemon.
    let dir = tempfile::tempdir().unwrap();
    let socket_path = dir.path().join("daemon.sock");

    // connect() should return NotRunning for a missing socket,
    // which is what ping_daemon() maps to Ok(false).
    let result = connect(&socket_path);
    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), DaemonClientError::NotRunning { .. }),
        "Should return NotRunning when daemon socket doesn't exist"
    );
}
```

**Why**: The test was testing `ping_daemon()` → `connect()` → `NotRunning` → `Ok(false)`. The `connect()` → `NotRunning` part is the actual behavior worth testing in isolation. The mapping from `NotRunning` to `Ok(false)` is trivially visible in `ping_daemon()`'s match arm at line 271.

### Step 2: Fix `test_get_session_status_returns_none_when_daemon_not_running`

**File**: `crates/kild-core/src/daemon/client.rs`
**Lines**: 389-403
**Action**: UPDATE

Same problem — uses `get_session_status()` which calls `socket_path()` internally.

**Current code:**
```rust
#[test]
fn test_get_session_status_returns_none_when_daemon_not_running() {
    // The daemon socket won't exist in test environments, so get_session_status
    // should return Ok(None) rather than an error.
    let result = get_session_status("nonexistent-session-id");
    assert!(
        result.is_ok(),
        "Should not error when daemon is not running"
    );
    assert_eq!(
        result.unwrap(),
        None,
        "Should return None when daemon socket doesn't exist"
    );
}
```

**Required change:**

Same approach — use `connect()` with a temp path to test the "daemon not running" behavior:

```rust
#[test]
fn test_get_session_status_returns_none_when_daemon_not_running() {
    // Use a temp directory to guarantee no socket exists, isolating from a running daemon.
    let dir = tempfile::tempdir().unwrap();
    let socket_path = dir.path().join("daemon.sock");

    // connect() should return NotRunning for a missing socket,
    // which is what get_session_status() maps to Ok(None).
    let result = connect(&socket_path);
    assert!(result.is_err());
    assert!(
        matches!(result.unwrap_err(), DaemonClientError::NotRunning { .. }),
        "Should return NotRunning when daemon socket doesn't exist"
    );
}
```

**Why**: Same reasoning as Step 1. The `get_session_status()` function maps `NotRunning` to `Ok(None)` at line 312-318. Testing `connect()` directly with a known-missing path validates the core behavior without depending on system state.

---

## Patterns to Follow

**From codebase — mirror the existing `connect()` test:**

```rust
// SOURCE: crates/kild-core/src/daemon/client.rs:405-416
// Pattern for testing connect with explicit path
#[test]
fn test_connect_returns_not_running_for_missing_socket() {
    let missing = Path::new("/tmp/kild-test-nonexistent-socket.sock");
    let result = connect(missing);
    assert!(result.is_err());
    match result.unwrap_err() {
        DaemonClientError::NotRunning { path } => {
            assert!(path.contains("nonexistent"));
        }
        other => panic!("Expected NotRunning, got: {:?}", other),
    }
}
```

**From codebase — tempdir pattern used in daemon PID tests:**

```rust
// SOURCE: crates/kild-daemon/src/pid.rs:114-124
let dir = tempfile::tempdir().unwrap();
let pid_path = dir.path().join("daemon.pid");
```

---

## Edge Cases & Risks

| Risk/Edge Case                  | Mitigation                                                    |
| ------------------------------- | ------------------------------------------------------------- |
| Tests now test `connect()` not the full public API | The mapping from `NotRunning` → `Ok(false)`/`Ok(None)` is a trivial match arm, not worth coupling tests to system state for |
| `tempfile` dependency missing   | Already in `Cargo.toml` as dev-dependency (used by other tests in kild-core) |

---

## Validation

### Automated Checks

```bash
cargo test -p kild-core test_ping_daemon_returns_false_when_not_running
cargo test -p kild-core test_get_session_status_returns_none_when_daemon_not_running
cargo test -p kild-core test_connect_returns_not_running_for_missing_socket
cargo clippy --all -- -D warnings
cargo fmt --check
```

### Manual Verification

1. Start the daemon with `kild daemon start`
2. Run `cargo test -p kild-core -- daemon::client` — all tests should pass
3. Stop the daemon with `kild daemon stop`
4. Run `cargo test -p kild-core -- daemon::client` — all tests should still pass

---

## Scope Boundaries

**IN SCOPE:**
- Fix the two flaky tests in `crates/kild-core/src/daemon/client.rs`

**OUT OF SCOPE (do not touch):**
- Making `ping_daemon()` or `get_session_status()` accept a socket path parameter (not needed for this fix, would change public API)
- Fixing similar patterns in `kild-tmux-shim` (separate issue if desired)
- Adding new test infrastructure or test helpers

---

## Metadata

- **Investigated by**: Claude
- **Timestamp**: 2026-02-10
- **Artifact**: `.claude/PRPs/issues/issue-307.md`
