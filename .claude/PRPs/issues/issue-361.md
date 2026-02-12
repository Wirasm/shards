# Investigation: agent-status should print confirmation on success

**Issue**: #361 (https://github.com/Wirasm/kild/issues/361)
**Type**: ENHANCEMENT
**Investigated**: 2026-02-12T12:00:00Z

### Assessment

| Metric     | Value  | Reasoning                                                                                             |
| ---------- | ------ | ----------------------------------------------------------------------------------------------------- |
| Priority   | MEDIUM | Blocks Assistant Agent persona parsing CLI output; not critical for hook integrations which work fine  |
| Complexity | LOW    | 2 files to change (CLI handler + clap definition), isolated to agent-status command with clear pattern |
| Confidence | HIGH   | Exact code paths identified, clear patterns to mirror from other commands, no unknowns                |

---

## Problem Statement

The `kild agent-status <branch> <status>` command silently succeeds with exit code 0 but no output. While this is fine for hook integrations (agents check exit code only), the Assistant Agent persona uses `--json` for all CLI interactions and expects parseable responses. The command needs a `--json` flag returning structured confirmation and optionally a plain-text confirmation line.

---

## Analysis

### Change Rationale

The issue is a missing output path, not a bug. The core handler already writes the status and returns successfully, but the CLI handler discards all context and returns `Ok(())` silently. The core handler returns `Result<(), SessionError>` - it doesn't return the data needed for confirmation output (branch, status, timestamp).

### Evidence Chain

WHY: No output on `kild agent-status my-branch working`
BECAUSE: CLI handler returns `Ok(())` with no println
Evidence: `crates/kild/src/commands/agent_status.rs:44-45` - `info!(event = ...); Ok(())`

BECAUSE: Core handler returns `Result<(), SessionError>` - no data to print
Evidence: `crates/kild-core/src/sessions/agent_status.rs:9-13` - `pub fn update_agent_status(...) -> Result<(), SessionError>`

BECAUSE: No `--json` flag defined in clap args
Evidence: `crates/kild/src/app.rs:349-371` - only `target`, `--self`, `--notify` defined

ROOT CAUSE: Missing output confirmation in CLI handler + no `--json` flag + core returns unit type

### Affected Files

| File                                         | Lines   | Action | Description                                          |
| -------------------------------------------- | ------- | ------ | ---------------------------------------------------- |
| `crates/kild/src/app.rs`                     | 365-370 | UPDATE | Add `--json` flag to agent-status subcommand         |
| `crates/kild/src/commands/agent_status.rs`   | 1-46    | UPDATE | Add JSON/text confirmation output + error handling   |
| `crates/kild/src/commands/json_types.rs`     | EOF     | UPDATE | Add `AgentStatusResponse` struct                     |
| `crates/kild-core/src/sessions/agent_status.rs` | 9-13 | UPDATE | Change return type to include confirmation data      |

### Integration Points

- `crates/kild/src/commands/mod.rs:55-57` dispatches to the handler (no change needed)
- `crates/kild-core/src/sessions/handler.rs` re-exports `update_agent_status` (signature change propagates)
- Agent hooks call this command programmatically and check exit code (must not break)

### Git History

- **Last modified**: `1d160bd` - "feat: add --notify flag to agent-status for desktop notifications (#305)"
- **Implication**: Recent feature addition, well-structured code ready for extension

---

## Implementation Plan

### Step 1: Change core handler return type to provide confirmation data

**File**: `crates/kild-core/src/sessions/agent_status.rs`
**Lines**: 9-13, 62
**Action**: UPDATE

**Current code:**

```rust
pub fn update_agent_status(
    name: &str,
    status: super::types::AgentStatus,
    notify: bool,
) -> Result<(), SessionError> {
    // ... body ...
    Ok(())
}
```

**Required change:**

Return `AgentStatusInfo` (which already contains `status` and `updated_at`) along with the branch name so the CLI can render confirmation:

```rust
/// Result of a successful agent status update.
pub struct AgentStatusResult {
    pub branch: String,
    pub status: super::types::AgentStatus,
    pub updated_at: String,
}

pub fn update_agent_status(
    name: &str,
    status: super::types::AgentStatus,
    notify: bool,
) -> Result<AgentStatusResult, SessionError> {
    // ... existing body unchanged until the return ...

    Ok(AgentStatusResult {
        branch: session.branch.clone(),
        status,
        updated_at: now,
    })
}
```

**Why**: The CLI needs the branch, status, and timestamp to render confirmation output. `AgentStatusResult` is purpose-built for this (not reusing `AgentStatusInfo` which is a persistence type).

### Step 2: Add `--json` flag to clap definition

**File**: `crates/kild/src/app.rs`
**Lines**: After line 370 (after `--notify` arg)
**Action**: UPDATE

**Current code:**

```rust
                .arg(
                    Arg::new("notify")
                        .long("notify")
                        .help("Send desktop notification when status is 'waiting' or 'error'")
                        .action(ArgAction::SetTrue)
                )
```

**Required change:**

Add `--json` flag after `--notify`:

```rust
                .arg(
                    Arg::new("notify")
                        .long("notify")
                        .help("Send desktop notification when status is 'waiting' or 'error'")
                        .action(ArgAction::SetTrue)
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .help("Output in JSON format")
                        .action(ArgAction::SetTrue)
                )
```

**Why**: Mirrors the pattern used by `list`, `status`, `pr`, `stats`, `overlaps` commands (see `app.rs:100-102`, `322-324`, `343-346`).

### Step 3: Add JSON response struct

**File**: `crates/kild/src/commands/json_types.rs`
**Lines**: EOF
**Action**: UPDATE

**Required change:**

```rust
/// JSON response for agent-status command confirmation.
#[derive(Serialize)]
pub struct AgentStatusResponse {
    pub branch: String,
    pub status: String,
    pub updated_at: String,
}
```

**Why**: Matches the expected output from the issue: `{"branch": "my-branch", "status": "working", "updated_at": "..."}`.

### Step 4: Update CLI handler with confirmation output

**File**: `crates/kild/src/commands/agent_status.rs`
**Lines**: 1-46 (full rewrite of handler)
**Action**: UPDATE

**Current code:**

```rust
use clap::ArgMatches;
use tracing::{error, info};

use kild_core::AgentStatus;
use kild_core::session_ops;

pub(crate) fn handle_agent_status_command(
    matches: &ArgMatches,
) -> Result<(), Box<dyn std::error::Error>> {
    let use_self = matches.get_flag("self");
    let notify = matches.get_flag("notify");
    let targets: Vec<&String> = matches.get_many::<String>("target").unwrap().collect();

    let (branch, status_str) = match (use_self, targets.as_slice()) {
        // ... branch resolution ...
    };

    let status: AgentStatus = status_str.parse().map_err(|_| {
        kild_core::sessions::errors::SessionError::InvalidAgentStatus {
            status: status_str.to_string(),
        }
    })?;

    info!(event = "cli.agent_status_started", branch = %branch, status = %status);

    if let Err(e) = session_ops::update_agent_status(&branch, status, notify) {
        error!(event = "cli.agent_status_failed", error = %e);
        return Err(e.into());
    }

    info!(event = "cli.agent_status_completed", branch = %branch, status = %status);
    Ok(())
}
```

**Required change:**

```rust
use clap::ArgMatches;
use tracing::{error, info};

use kild_core::AgentStatus;
use kild_core::session_ops;

pub(crate) fn handle_agent_status_command(
    matches: &ArgMatches,
) -> Result<(), Box<dyn std::error::Error>> {
    let use_self = matches.get_flag("self");
    let notify = matches.get_flag("notify");
    let json_output = matches.get_flag("json");
    let targets: Vec<&String> = matches.get_many::<String>("target").unwrap().collect();

    let (branch, status_str) = match (use_self, targets.as_slice()) {
        (true, [status]) => {
            let cwd = std::env::current_dir()?;
            let session = session_ops::find_session_by_worktree_path(&cwd)?.ok_or_else(|| {
                format!(
                    "No kild session found for current directory: {}",
                    cwd.display()
                )
            })?;
            (session.branch, status.as_str())
        }
        (false, [branch, status]) => ((*branch).clone(), status.as_str()),
        (true, _) => return Err("Usage: kild agent-status --self <status>".into()),
        (false, _) => return Err("Usage: kild agent-status <branch> <status>".into()),
    };

    let status: AgentStatus = status_str.parse().map_err(|_| {
        kild_core::sessions::errors::SessionError::InvalidAgentStatus {
            status: status_str.to_string(),
        }
    })?;

    info!(event = "cli.agent_status_started", branch = %branch, status = %status);

    match session_ops::update_agent_status(&branch, status, notify) {
        Ok(result) => {
            if json_output {
                let response = super::json_types::AgentStatusResponse {
                    branch: result.branch,
                    status: result.status.to_string(),
                    updated_at: result.updated_at,
                };
                println!("{}", serde_json::to_string_pretty(&response)?);
            }
            info!(event = "cli.agent_status_completed", branch = %branch, status = %status);
            Ok(())
        }
        Err(e) => {
            error!(event = "cli.agent_status_failed", error = %e);
            if json_output {
                return Err(super::helpers::print_json_error(&e, e.error_code()));
            }
            Err(e.into())
        }
    }
}
```

**Why**:
- JSON mode returns structured `{"branch", "status", "updated_at"}` confirmation
- Plain-text mode stays silent for hook compatibility (as requested in the issue)
- Error path uses `print_json_error` helper for consistent JSON error output
- Mirrors patterns from `status.rs:41-86` and `list.rs:54-117`

### Step 5: Re-export `AgentStatusResult` from kild-core

**File**: `crates/kild-core/src/sessions/handler.rs`
**Action**: UPDATE

Ensure `AgentStatusResult` is re-exported so the CLI can use it (verify the existing re-export pattern covers it).

### Step 6: Add tests

**File**: `crates/kild/tests/cli_json_output.rs`
**Action**: UPDATE

**Test cases to add:**

```rust
#[test]
fn test_agent_status_json_nonexistent_returns_json_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_kild"))
        .args(["agent-status", "nonexistent-branch-xyz-12345", "working", "--json"])
        .output()
        .expect("Failed to execute 'kild agent-status --json'");

    // Command should fail (non-zero exit code)
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let error_obj: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON even on error");

    assert!(error_obj.is_object());
    assert_eq!(
        error_obj.get("code").and_then(|v| v.as_str()),
        Some("SESSION_NOT_FOUND")
    );
}

#[test]
fn test_agent_status_json_invalid_status_returns_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_kild"))
        .args(["agent-status", "some-branch", "bogus", "--json"])
        .output()
        .expect("Failed to execute 'kild agent-status --json'");

    assert!(!output.status.success());
}
```

Note: Testing the success path requires a running session (integration test). Error paths can be tested in isolation.

---

## Patterns to Follow

**From codebase - mirror these exactly:**

```rust
// SOURCE: crates/kild/src/commands/status.rs:41-79
// Pattern for JSON output path
if json_output {
    // ... collect data ...
    println!("{}", serde_json::to_string_pretty(&enriched)?);
    info!(event = "cli.status_completed", branch = branch);
    return Ok(());
}
```

```rust
// SOURCE: crates/kild/src/commands/status.rs:238-248
// Pattern for JSON error handling
if json_output {
    return Err(super::helpers::print_json_error(&e, e.error_code()));
}
```

```rust
// SOURCE: crates/kild/src/commands/json_types.rs:92-97
// Pattern for JSON response struct
#[derive(Serialize)]
pub struct JsonError {
    pub error: String,
    pub code: String,
}
```

---

## Edge Cases & Risks

| Risk/Edge Case                          | Mitigation                                                                  |
| --------------------------------------- | --------------------------------------------------------------------------- |
| Breaking hook integrations (silent mode) | Plain-text mode stays silent; only `--json` adds output                    |
| Core return type change breaks callers  | Only CLI handler calls `update_agent_status`; change is contained           |
| `--self` mode with `--json`             | Works naturally since branch is resolved before JSON output                 |
| Invalid status with `--json`            | Status parsing error happens before core call; need JSON error for this too |

Note on invalid status: The `parse()` error at line 31-35 currently returns a `SessionError` but doesn't go through the JSON error path. The updated handler should handle this:

```rust
let status: AgentStatus = status_str.parse().map_err(|_| {
    let e = kild_core::sessions::errors::SessionError::InvalidAgentStatus {
        status: status_str.to_string(),
    };
    if json_output {
        // print_json_error returns Box<dyn Error>, use it directly
        return super::helpers::print_json_error(&e, e.error_code());
    }
    Box::new(e) as Box<dyn std::error::Error>
})?;
```

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

1. `kild agent-status <branch> working --json` returns `{"branch": "...", "status": "working", "updated_at": "..."}`
2. `kild agent-status <branch> working` stays silent (hook compatibility)
3. `kild agent-status nonexistent working --json` returns JSON error with code `SESSION_NOT_FOUND`
4. `kild agent-status <branch> bogus --json` returns JSON error with code `INVALID_AGENT_STATUS`

---

## Scope Boundaries

**IN SCOPE:**

- Add `--json` flag to `agent-status` subcommand
- Add JSON confirmation output for success
- Add JSON error output for failures
- Change core handler return type to provide confirmation data
- Keep plain-text mode silent (hook compatibility)

**OUT OF SCOPE (do not touch):**

- Adding plain-text confirmation (e.g., `Updated: my-branch -> working`) - the issue says current silent behavior is "fine for hooks", defer this
- Notification system changes
- `--self` flag behavior changes
- Other commands' JSON output

---

## Metadata

- **Investigated by**: Claude
- **Timestamp**: 2026-02-12T12:00:00Z
- **Artifact**: `.claude/PRPs/issues/issue-361.md`
