# Implementation Plan: GUI Phase 7 - Status Dashboard

**Source PRD**: `.claude/PRPs/prds/gpui-native-terminal-ui.prd.md`
**Phase**: 7
**Status**: READY FOR IMPLEMENTATION

---

## Summary

Implement live status indicators and auto-refresh for the shards-ui dashboard. The current UI requires manual refresh or restart to see status changes. After this phase, the dashboard will auto-update every 5 seconds with clear visual status indicators and additional session metadata.

## User Story

As a power user managing multiple AI agent shards, I want the dashboard to automatically update shard status so I can monitor at a glance whether agents are running, stopped, or crashed without manually refreshing or running CLI commands.

## Problem Statement

The current shards-ui dashboard is static after initial load:
- Status indicators do not update when terminal processes die
- Users must manually click "Refresh" or restart the app to see current state
- No timestamps showing when shards were created or last active
- No worktree path visibility for debugging/navigation

## Solution Statement

Implement a background polling mechanism using GPUI's async executor that:
1. Polls process status every 5 seconds without full session reload
2. Updates status indicators with clear visual differentiation
3. Displays additional metadata (created time, last activity, worktree path)
4. Minimizes UI flicker during updates by only updating changed shards

## Metadata

| Field | Value |
|-------|-------|
| Type | ENHANCEMENT |
| Complexity | MEDIUM |
| Systems Affected | shards-ui |
| Dependencies | Phases 1-6 complete |
| Estimated Tasks | 10 |

---

## Mandatory Reading

**CRITICAL: Implementation agent MUST read these files before starting any task:**

| Priority | File | Lines | Why Read This |
|----------|------|-------|---------------|
| P0 | `crates/shards-ui/src/main.rs` | 1-32 | Application entry point, window setup |
| P0 | `crates/shards-ui/src/state.rs` | 1-55 | ShardDisplay, ProcessStatus, AppState types |
| P0 | `crates/shards-ui/src/views/main_view.rs` | 1-100 | MainView structure, handler patterns |
| P0 | `crates/shards-ui/src/views/shard_list.rs` | 60-100 | Current status rendering code |
| P1 | `crates/shards-core/src/process/operations.rs` | 14-19 | is_process_running() API |

---

## Patterns to Mirror

**State Management in MainView:**
```rust
// SOURCE: crates/shards-ui/src/views/main_view.rs:18-29
pub struct MainView {
    state: AppState,
    focus_handle: FocusHandle,
}
```

**Refresh Sessions Pattern:**
```rust
// SOURCE: crates/shards-ui/src/state.rs:141-146
pub fn refresh_sessions(&mut self) {
    let (displays, load_error) = crate::actions::refresh_sessions();
    self.displays = displays;
    self.load_error = load_error;
}
```

**Process Status Check Pattern:**
```rust
// SOURCE: crates/shards-ui/src/state.rs:34-53
impl ShardDisplay {
    pub fn from_session(session: Session) -> Self {
        let status = match session.process_id {
            None => ProcessStatus::Stopped,
            Some(pid) => match shards_core::process::is_process_running(pid) {
                Ok(true) => ProcessStatus::Running,
                Ok(false) => ProcessStatus::Stopped,
                Err(e) => ProcessStatus::Unknown
            },
        };
        Self { session, status }
    }
}
```

---

## Files to Change

| File | Action | Justification |
|------|--------|---------------|
| `crates/shards-ui/src/refresh.rs` | CREATE | New module for background refresh logic |
| `crates/shards-ui/src/main.rs` | UPDATE | Add `mod refresh;` declaration |
| `crates/shards-ui/src/state.rs` | UPDATE | Add `update_statuses_only()` method, add `last_refresh` timestamp |
| `crates/shards-ui/src/views/main_view.rs` | UPDATE | Add background refresh task startup in `new()` |
| `crates/shards-ui/src/views/shard_list.rs` | UPDATE | Add created_at, last_activity display; update status colors |
| `crates/shards-ui/Cargo.toml` | UPDATE | Add chrono dependency for timestamp formatting |

---

## NOT Building (Scope Limits)

- **Complex real-time streaming** - YAGNI, simple polling is sufficient
- **Notifications (toast/alert)** - Future phase
- **Configurable refresh interval** - YAGNI, 5 seconds is fine
- **Status history/timeline** - YAGNI
- **Process CPU/memory metrics** - Future phase (health metrics)

---

## Step-by-Step Tasks

### Task 1: Create refresh module

- **ACTION**: Create new module with refresh interval constant
- **FILE**: `crates/shards-ui/src/refresh.rs` (CREATE)
- **IMPLEMENT**:
```rust
//! Background refresh logic for status dashboard.
//!
//! Provides auto-refresh functionality that polls process status
//! every 5 seconds without full session reload.

use std::time::Duration;

/// Refresh interval for auto-update (5 seconds as per PRD)
pub const REFRESH_INTERVAL: Duration = Duration::from_secs(5);
```
- **VALIDATE**: `cargo check -p shards-ui`

### Task 2: Add module declaration to main.rs

- **ACTION**: Add `mod refresh;` to main.rs
- **FILE**: `crates/shards-ui/src/main.rs`
- **LOCATION**: After existing mod declarations (around line 10)
- **IMPLEMENT**:
```rust
mod refresh;
```
- **VALIDATE**: `cargo check -p shards-ui`

### Task 3: Add status-only update method to AppState

- **ACTION**: Add efficient status polling method
- **FILE**: `crates/shards-ui/src/state.rs`
- **LOCATION**: After `refresh_sessions()` method
- **IMPLEMENT**:
```rust
/// Update only the process status of existing shards without reloading from disk.
///
/// This is faster than refresh_sessions() for status polling because it:
/// - Doesn't reload session files from disk
/// - Only checks if tracked processes are still running
/// - Preserves the existing shard list structure
pub fn update_statuses_only(&mut self) {
    for display in &mut self.displays {
        if let Some(pid) = display.session.process_id {
            display.status = match shards_core::process::is_process_running(pid) {
                Ok(true) => ProcessStatus::Running,
                Ok(false) => ProcessStatus::Stopped,
                Err(e) => {
                    tracing::warn!(
                        event = "ui.status_update.process_check_failed",
                        pid = pid,
                        branch = display.session.branch,
                        error = %e
                    );
                    ProcessStatus::Unknown
                }
            };
        }
    }
    self.last_refresh = Some(std::time::Instant::now());
}
```
- **MIRROR**: `ShardDisplay::from_session()` pattern
- **VALIDATE**: `cargo check -p shards-ui`

### Task 4: Add last_refresh timestamp to AppState

- **ACTION**: Track when status was last updated
- **FILE**: `crates/shards-ui/src/state.rs`
- **LOCATION**: In AppState struct
- **IMPLEMENT**:
```rust
pub struct AppState {
    // ... existing fields ...

    /// Timestamp of last successful status refresh
    pub last_refresh: Option<std::time::Instant>,
}
```
- Also update `AppState::new()`:
```rust
last_refresh: Some(std::time::Instant::now()),
```
- **VALIDATE**: `cargo check -p shards-ui`

### Task 5: Implement background refresh timer in MainView

- **ACTION**: Add GPUI timer that polls status every 5 seconds
- **FILE**: `crates/shards-ui/src/views/main_view.rs`
- **IMPLEMENT**:
```rust
use gpui::Task;

pub struct MainView {
    state: AppState,
    focus_handle: FocusHandle,
    _refresh_task: Task<()>,  // Hold to prevent dropping
}

impl MainView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let refresh_task = cx.spawn(async move |this, mut cx| {
            loop {
                cx.background_executor()
                    .timer(crate::refresh::REFRESH_INTERVAL)
                    .await;

                let _ = this.update(&mut cx, |view, cx| {
                    tracing::debug!(event = "ui.auto_refresh.tick");
                    view.state.update_statuses_only();
                    cx.notify();
                });
            }
        });

        Self {
            state: AppState::new(),
            focus_handle: cx.focus_handle(),
            _refresh_task: refresh_task,
        }
    }
}
```
- **GOTCHA**: Task handle must be stored to prevent cancellation
- **VALIDATE**: `cargo check -p shards-ui`

### Task 6: Update status indicator colors

- **ACTION**: Match PRD spec for status colors
- **FILE**: `crates/shards-ui/src/views/shard_list.rs`
- **LOCATION**: Status color assignment (around line 62)
- **IMPLEMENT**:
```rust
let status_color = match display.status {
    ProcessStatus::Running => rgb(0x00ff00), // Green
    ProcessStatus::Stopped => rgb(0xff0000), // Red
    ProcessStatus::Unknown => rgb(0x888888), // Gray (was orange)
};
```
- **VALIDATE**: Visual check via `cargo run -p shards-ui`

### Task 7: Add chrono dependency

- **ACTION**: Add chrono for timestamp formatting
- **FILE**: `crates/shards-ui/Cargo.toml`
- **IMPLEMENT**:
```toml
[dependencies]
chrono.workspace = true
```
- **VALIDATE**: `cargo check -p shards-ui`

### Task 8: Add created_at timestamp display

- **ACTION**: Show relative time for when shard was created
- **FILE**: `crates/shards-ui/src/views/shard_list.rs`
- **IMPLEMENT**:
```rust
/// Format RFC3339 timestamp as relative time (e.g., "5m ago", "2h ago")
fn format_relative_time(timestamp: &str) -> String {
    use chrono::{DateTime, Utc};

    let Ok(created) = DateTime::parse_from_rfc3339(timestamp) else {
        return timestamp.to_string();
    };

    let now = Utc::now();
    let duration = now.signed_duration_since(created.with_timezone(&Utc));

    let minutes = duration.num_minutes();
    let hours = duration.num_hours();
    let days = duration.num_days();

    if days > 0 { format!("{}d ago", days) }
    else if hours > 0 { format!("{}h ago", hours) }
    else if minutes > 0 { format!("{}m ago", minutes) }
    else { "just now".to_string() }
}
```
- Add to row rendering after project_id
- **VALIDATE**: `cargo run -p shards-ui`

### Task 9: Add last_activity display

- **ACTION**: Show last activity time if available
- **FILE**: `crates/shards-ui/src/views/shard_list.rs`
- **IMPLEMENT**:
```rust
.when_some(display.session.last_activity.clone(), |row, activity| {
    row.child(
        div()
            .text_color(rgb(0x666666))
            .text_sm()
            .child(format_relative_time(&activity)),
    )
})
```
- **VALIDATE**: `cargo run -p shards-ui`

### Task 10: Add worktree path tooltip

- **ACTION**: Show worktree path on hover
- **FILE**: `crates/shards-ui/src/views/shard_list.rs`
- **IMPLEMENT**:
```rust
.child(
    div()
        .flex_1()
        .text_color(rgb(0xffffff))
        .child(branch.clone())
        .tooltip({
            let path = display.session.worktree_path.display().to_string();
            move |cx| gpui::Tooltip::text(format!("Path: {}", path), cx)
        }),
)
```
- **VALIDATE**: `cargo run -p shards-ui` and hover over branch name

---

## Validation Commands

### Level 1: STATIC_ANALYSIS
```bash
cargo fmt --check && cargo clippy --all -- -D warnings
```

### Level 2: TYPE_CHECK
```bash
cargo check --all
```

### Level 3: BUILD
```bash
cargo build --all
```

### Level 4: TESTS
```bash
cargo test --all
```

### Level 5: MANUAL_VALIDATION
```bash
# Create a shard
shards create test-dashboard --agent claude

# Open UI
cargo run -p shards-ui
# Verify green status indicator

# Close the terminal window manually
# Wait 5 seconds
# Verify status changes to red without clicking Refresh

# Restart shard via UI
# Verify status changes to green within 5 seconds

# Clean up
shards destroy --force test-dashboard
```

---

## Acceptance Criteria

- [ ] Status indicators show: Green (Running), Red (Stopped), Gray (Unknown)
- [ ] Status auto-updates every 5 seconds without manual refresh
- [ ] Created time displayed for each shard
- [ ] Last activity time displayed (when available)
- [ ] Worktree path accessible via tooltip
- [ ] No UI flicker during status updates
- [ ] Closing terminal window causes status to change within 5 seconds
- [ ] Restarting shard causes status to change within 5 seconds
- [ ] All validation commands pass

---

## Completion Checklist

- [ ] Task 1: Created refresh.rs module
- [ ] Task 2: Added mod declaration to main.rs
- [ ] Task 3: Added update_statuses_only() method
- [ ] Task 4: Added last_refresh timestamp
- [ ] Task 5: Implemented background refresh timer
- [ ] Task 6: Updated status indicator colors
- [ ] Task 7: Added chrono dependency
- [ ] Task 8: Added created_at display
- [ ] Task 9: Added last_activity display
- [ ] Task 10: Added worktree path tooltip
- [ ] Level 1-4 validation commands pass
- [ ] Manual testing completed
