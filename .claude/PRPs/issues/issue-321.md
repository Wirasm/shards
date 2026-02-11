# Investigation: sync_daemon_session_status full re-save drops newer fields

**Issue**: #321 (https://github.com/Wirasm/kild/issues/321)
**Type**: BUG
**Investigated**: 2026-02-11T00:00:00Z

### Assessment

| Metric     | Value  | Reasoning                                                                                                          |
| ---------- | ------ | ------------------------------------------------------------------------------------------------------------------ |
| Severity   | MEDIUM | Field loss is silent and only triggers when mixing binary versions; workaround exists (rebuild from source)         |
| Complexity | LOW    | Single function change in one file, plus a new multi-field patch helper and test; mirrors existing pattern exactly  |
| Confidence | HIGH   | Root cause is identical to the agent_status bug fixed in PR #319; code path is clear, fix pattern already proven   |

---

## Problem Statement

`sync_daemon_session_status()` in `handler.rs:1243` reads a session, mutates `status` and `last_activity` in memory, then calls `save_session_to_file()` which serializes the entire `Session` struct. When an older installed binary runs `kild list` or `kild status`, any session fields introduced by a newer version (e.g., `task_list_id`) are silently dropped because the older struct doesn't know about them.

This is the exact same bug that was fixed in `agent_status.rs` by PR #319, just in a different code path.

---

## Analysis

### Root Cause

WHY: Session fields from newer binary versions get dropped during `kild list`/`kild status`
- BECAUSE: `sync_daemon_session_status()` calls `save_session_to_file()` which round-trips through `Session` struct serialization
- Evidence: `handler.rs:1288` - `persistence::save_session_to_file(session, &config.sessions_dir())`

WHY: Round-tripping through `Session` struct drops unknown fields
- BECAUSE: `serde_json::to_string_pretty(session)` only serializes fields known to the current struct definition
- Evidence: `persistence.rs:28` - `serde_json::to_string_pretty(session)`

ROOT CAUSE: `sync_daemon_session_status()` uses full struct serialization instead of targeted JSON field patching, identical to the agent_status bug fixed in PR #319.

### Evidence Chain

1. `kild list` calls `sync_daemon_session_status()` for each active session (`list.rs:18`)
2. `kild status` calls `sync_daemon_session_status()` for the target session (`status.rs:29`)
3. Function only modifies two fields: `status` (line 1284) and `last_activity` (line 1285)
4. But saves the entire struct via `save_session_to_file()` (line 1288)
5. Any field not in the older binary's `Session` struct is lost

### Affected Files

| File                                          | Lines    | Action | Description                                         |
| --------------------------------------------- | -------- | ------ | --------------------------------------------------- |
| `crates/kild-core/src/sessions/persistence.rs` | 161-203  | UPDATE | Add `patch_session_json_fields` for multi-field patches |
| `crates/kild-core/src/sessions/handler.rs`    | 1284-1299 | UPDATE | Replace `save_session_to_file` with field patches   |

### Integration Points

- `crates/kild/src/commands/list.rs:18` - calls `sync_daemon_session_status()` in loop for all sessions
- `crates/kild/src/commands/status.rs:29` - calls `sync_daemon_session_status()` for single session
- `crates/kild-core/src/sessions/persistence.rs:166` - existing `patch_session_json_field` (single field)

### Git History

- **Introduced**: `dc889ca` - "feat: daemon status sync and open command daemon support (#299)"
- **Related fix**: `bd6a034` - "feat: transfer task list across agent sessions via CLAUDE_CODE_TASK_LIST_ID (#319)" (fixed agent_status.rs)
- **Implication**: Known pattern, same class of bug as agent_status

---

## Implementation Plan

### Step 1: Add `patch_session_json_fields` to persistence.rs

**File**: `crates/kild-core/src/sessions/persistence.rs`
**Lines**: After line 203 (after existing `patch_session_json_field`)
**Action**: CREATE new function

**Why**: `sync_daemon_session_status` updates two fields (`status` and `last_activity`). Calling `patch_session_json_field` twice means two file reads/writes and non-atomic updates. A multi-field variant is atomic and efficient.

**Required code:**

```rust
/// Patch multiple fields in a session JSON file without deserializing into Session.
///
/// Same as `patch_session_json_field` but for multiple fields atomically.
/// This avoids multiple file reads/writes when updating several fields at once.
pub fn patch_session_json_fields(
    sessions_dir: &Path,
    session_id: &str,
    fields: &[(&str, serde_json::Value)],
) -> Result<(), SessionError> {
    let session_file = sessions_dir.join(format!("{}.json", session_id.replace('/', "_")));
    let content =
        fs::read_to_string(&session_file).map_err(|e| SessionError::IoError { source: e })?;
    let mut json: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| SessionError::IoError {
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
        })?;

    let obj = json.as_object_mut().ok_or_else(|| SessionError::IoError {
        source: std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "session JSON root is not an object",
        ),
    })?;
    for (field, value) in fields {
        obj.insert((*field).to_string(), value.clone());
    }

    let updated = serde_json::to_string_pretty(&json).map_err(|e| SessionError::IoError {
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
    })?;

    let temp_file = session_file.with_extension("json.tmp");
    if let Err(e) = fs::write(&temp_file, &updated) {
        cleanup_temp_file(&temp_file, &e);
        return Err(SessionError::IoError { source: e });
    }
    if let Err(e) = fs::rename(&temp_file, &session_file) {
        cleanup_temp_file(&temp_file, &e);
        return Err(SessionError::IoError { source: e });
    }

    Ok(())
}
```

---

### Step 2: Replace `save_session_to_file` in `sync_daemon_session_status`

**File**: `crates/kild-core/src/sessions/handler.rs`
**Lines**: 1284-1299
**Action**: UPDATE

**Current code:**

```rust
// Line 1284-1299
session.status = SessionStatus::Stopped;
session.last_activity = Some(chrono::Utc::now().to_rfc3339());

let config = Config::new();
if let Err(e) = persistence::save_session_to_file(session, &config.sessions_dir()) {
    error!(
        event = "core.session.daemon_status_sync_save_failed",
        session_id = session.id,
        error = %e,
        "Failed to persist synced status"
    );
    eprintln!(
        "Warning: kild '{}' status is stale (daemon stopped but save failed: {}). Check disk space/permissions in ~/.kild/sessions/",
        session.branch, e
    );
}
```

**Required change:**

```rust
let now = chrono::Utc::now().to_rfc3339();

// Patch status and last_activity via targeted JSON update to preserve unknown fields.
// Using patch instead of full save prevents older binaries from dropping new fields
// (e.g., installed kild binary dropping task_list_id added by a newer version).
let config = Config::new();
if let Err(e) = persistence::patch_session_json_fields(
    &config.sessions_dir(),
    &session.id,
    &[
        ("status", serde_json::json!("Stopped")),
        ("last_activity", serde_json::Value::String(now.clone())),
    ],
) {
    error!(
        event = "core.session.daemon_status_sync_save_failed",
        session_id = session.id,
        error = %e,
        "Failed to persist synced status"
    );
    eprintln!(
        "Warning: kild '{}' status is stale (daemon stopped but save failed: {}). Check disk space/permissions in ~/.kild/sessions/",
        session.branch, e
    );
}

// Update in-memory session for callers (list/status display)
session.status = SessionStatus::Stopped;
session.last_activity = Some(now);
```

**Why**: Mirrors the agent_status fix pattern. Updates only `status` and `last_activity` while preserving all other fields including those from newer binary versions.

---

### Step 3: Add test for `patch_session_json_fields`

**File**: `crates/kild-core/src/sessions/persistence.rs`
**Action**: UPDATE (add test to existing `#[cfg(test)]` module)

**Test cases to add:**

```rust
#[test]
fn test_patch_session_json_fields_preserves_unknown_fields() {
    use std::env;

    let temp_dir = env::temp_dir().join("kild_test_patch_multi_preserves_fields");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let json = serde_json::json!({
        "id": "proj/my-branch",
        "project_id": "proj",
        "branch": "my-branch",
        "worktree_path": "/tmp/test",
        "agent": "claude",
        "status": "Active",
        "created_at": "2024-01-01T00:00:00Z",
        "port_range_start": 3000,
        "port_range_end": 3009,
        "port_count": 10,
        "last_activity": "2024-01-01T00:00:00Z",
        "agents": [],
        "future_field": "must_survive"
    });
    let session_file = temp_dir.join("proj_my-branch.json");
    std::fs::write(&session_file, serde_json::to_string_pretty(&json).unwrap()).unwrap();

    // Patch both status and last_activity atomically
    patch_session_json_fields(
        &temp_dir,
        "proj/my-branch",
        &[
            ("status", serde_json::json!("Stopped")),
            ("last_activity", serde_json::Value::String("2024-06-15T12:00:00Z".to_string())),
        ],
    )
    .unwrap();

    let content = std::fs::read_to_string(&session_file).unwrap();
    let patched: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(patched["status"], "Stopped", "Status should be updated");
    assert_eq!(patched["last_activity"], "2024-06-15T12:00:00Z", "last_activity should be updated");
    assert_eq!(patched["future_field"], "must_survive", "Unknown fields must be preserved");
    assert_eq!(patched["branch"], "my-branch", "Existing fields must be preserved");

    let _ = std::fs::remove_dir_all(&temp_dir);
}
```

---

## Patterns to Follow

**From codebase - mirror these exactly:**

```rust
// SOURCE: agent_status.rs:35-43
// Pattern for targeted field update preserving unknown fields
persistence::patch_session_json_field(
    &config.sessions_dir(),
    &session.id,
    "last_activity",
    serde_json::Value::String(now),
)?;
```

```rust
// SOURCE: persistence.rs:161-203
// Pattern for atomic JSON field patching
pub fn patch_session_json_field(
    sessions_dir: &Path,
    session_id: &str,
    field: &str,
    value: serde_json::Value,
) -> Result<(), SessionError> { ... }
```

---

## Edge Cases & Risks

| Risk/Edge Case                         | Mitigation                                                                  |
| -------------------------------------- | --------------------------------------------------------------------------- |
| Two-field patch is not atomic at OS level | temp file + rename is as atomic as the single-field version                 |
| `SessionStatus::Stopped` serialized form | Uses `"Stopped"` string directly in JSON value, matching serde serialization |
| In-memory session out of sync with disk | Update in-memory fields after successful disk write                          |
| Empty fields slice passed              | No-op on the JSON object, still writes file (acceptable, caller controls)    |

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

1. Build new binary, create a daemon session, verify `kild list` works
2. Add a fake `"future_field"` to a session JSON, run `kild list` when daemon is stopped, verify field survives

---

## Scope Boundaries

**IN SCOPE:**
- `sync_daemon_session_status` in `handler.rs` - replace `save_session_to_file` with field patches
- `persistence.rs` - add `patch_session_json_fields` multi-field variant
- Test for the new function

**OUT OF SCOPE (do not touch):**
- Other `save_session_to_file` call sites (create, open, stop, complete) - these are safe per issue analysis
- `agent_status.rs` - already fixed in PR #319
- Refactoring `patch_session_json_field` to call `patch_session_json_fields` internally (YAGNI)

---

## Metadata

- **Investigated by**: Claude
- **Timestamp**: 2026-02-11
- **Artifact**: `.claude/PRPs/issues/issue-321.md`
