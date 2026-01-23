# Feature: Phase 3 - Shard List View

## Summary

Display existing shards from `~/.shards/sessions/` in a GPUI list view. This is the core value proposition of the UI - seeing all shards in one place with their status. Read-only for Phase 3; no create/destroy buttons yet.

## User Story

As a power user managing multiple AI agent shards
I want to see all my shards in a visual list with their status
So that I can quickly understand which shards are running across my projects

## Problem Statement

The CLI requires repeated `shards list` commands to see shard status. There's no persistent visual overview. Users must remember and type commands repeatedly.

## Solution Statement

Build a GPUI list view that loads sessions from `~/.shards/sessions/` on startup, checks process status for each, and displays them in a scrollable list. Uses shards-core's existing `session_ops::list_sessions()` API.

## Metadata

| Field            | Value                                        |
| ---------------- | -------------------------------------------- |
| Type             | NEW_CAPABILITY                               |
| Complexity       | MEDIUM                                       |
| Systems Affected | shards-ui                                    |
| Dependencies     | gpui 0.2, shards-core                        |
| Estimated Tasks  | 6                                            |

---

## UX Design

### Before State

```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                              BEFORE STATE                                      ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║   ┌─────────────┐         ┌─────────────┐         ┌─────────────┐            ║
║   │   Terminal  │ ──────► │ shards list │ ──────► │  Text table │            ║
║   │   (CLI)     │         │   command   │         │   output    │            ║
║   └─────────────┘         └─────────────┘         └─────────────┘            ║
║                                                                               ║
║   USER_FLOW: Open terminal → type "shards list" → read text output           ║
║   PAIN_POINT: Must repeat command to see updates, no persistent view         ║
║   DATA_FLOW: CLI → session_ops::list_sessions() → text table → stdout        ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

### After State

```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                               AFTER STATE                                      ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║   ┌─────────────────────────────────────────────────────────────┐            ║
║   │                    Shards UI Window                          │            ║
║   │  ┌─────────────────────────────────────────────────────────┐│            ║
║   │  │  "Shards" title                                          ││            ║
║   │  ├─────────────────────────────────────────────────────────┤│            ║
║   │  │  ┌─────────────────────────────────────────────────┐    ││            ║
║   │  │  │ 🟢 feature-auth   │ claude │ my-project         │    ││            ║
║   │  │  ├─────────────────────────────────────────────────┤    ││            ║
║   │  │  │ 🔴 fix-bug        │ kiro   │ other-project      │    ││            ║
║   │  │  ├─────────────────────────────────────────────────┤    ││            ║
║   │  │  │ 🟢 refactor-api   │ claude │ my-project         │    ││            ║
║   │  │  └─────────────────────────────────────────────────┘    ││            ║
║   │  │                                                          ││            ║
║   │  │  (empty state: "No active shards. Create one via CLI")  ││            ║
║   │  └─────────────────────────────────────────────────────────┘│            ║
║   └─────────────────────────────────────────────────────────────┘            ║
║                                                                               ║
║   USER_FLOW: Launch shards-ui → see all shards immediately                   ║
║   VALUE_ADD: Persistent view, visual status indicators, no typing            ║
║   DATA_FLOW: App startup → session_ops::list_sessions() → GPUI render        ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

### Interaction Changes

| Location      | Before          | After                        | User Impact                  |
| ------------- | --------------- | ---------------------------- | ---------------------------- |
| UI Window     | "Shards" text   | Shard list with status       | See all shards at a glance   |
| Each shard    | N/A             | Branch, agent, project, icon | Understand shard state       |
| Empty state   | N/A             | Helpful message              | Know what to do next         |

---

## Mandatory Reading

**CRITICAL: Implementation agent MUST read these files before starting any task:**

| Priority | File | Lines | Why Read This |
|----------|------|-------|---------------|
| P0 | `crates/shards-ui/src/main.rs` | all | Current GPUI setup to EXTEND |
| P0 | `crates/shards-core/src/sessions/handler.rs` | 152-172 | `list_sessions()` API to CALL |
| P0 | `crates/shards-core/src/sessions/types.rs` | 21-94 | `Session` type to DISPLAY |
| P1 | `crates/shards-core/src/process/operations.rs` | 14-19 | `is_process_running()` for status |
| P1 | `crates/shards-core/src/lib.rs` | 27-38 | Public API exports to IMPORT |
| P2 | `crates/shards/src/table.rs` | 51-86 | How CLI renders session info |

**External Documentation:**

| Source | Section | Why Needed |
|--------|---------|------------|
| [GPUI docs.rs](https://docs.rs/gpui/0.2.0/gpui/) | Render trait, div | Core rendering pattern |
| [Zed uniform_list example](https://github.com/zed-industries/zed/blob/main/crates/gpui/examples/uniform_list.rs) | uniform_list usage | List rendering pattern |

---

## Patterns to Mirror

**GPUI_RENDER_PATTERN:**
```rust
// SOURCE: crates/shards-ui/src/main.rs:13-24
// COPY THIS PATTERN for view rendering:
impl Render for MainView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .justify_center()
            .items_center()
            .bg(rgb(0x1e1e1e))
            .text_3xl()
            .text_color(rgb(0xffffff))
            .child("Shards")
    }
}
```

**SESSION_LOADING_PATTERN:**
```rust
// SOURCE: crates/shards-core/src/sessions/handler.rs:152-172
// USE THIS API to load sessions:
pub fn list_sessions() -> Result<Vec<Session>, SessionError> {
    info!(event = "core.session.list_started");
    let config = Config::new();
    let (sessions, skipped_count) = operations::load_sessions_from_files(&config.sessions_dir())?;
    // ... logging ...
    Ok(sessions)
}
```

**PROCESS_STATUS_PATTERN:**
```rust
// SOURCE: crates/shards/src/table.rs:53-67
// MIRROR THIS for determining Running/Stopped status:
let process_status = session.process_id.map_or("No PID".to_string(), |pid| {
    match shards_core::process::is_process_running(pid) {
        Ok(true) => format!("Run({})", pid),
        Ok(false) => format!("Stop({})", pid),
        Err(e) => {
            tracing::warn!(
                event = "cli.list_process_check_failed",
                pid = pid,
                session_branch = &session.branch,
                error = %e
            );
            format!("Err({})", pid)
        }
    }
});
```

**UNIFORM_LIST_PATTERN:**
```rust
// SOURCE: Zed gpui/examples/uniform_list.rs
// ADAPT THIS for shard list:
uniform_list(
    "shard-list",
    session_count,
    cx.processor(|this, range, _window, _cx| {
        let mut items = Vec::new();
        for ix in range {
            let session = &this.sessions[ix];
            items.push(
                div()
                    .id(ix)
                    .px_2()
                    .child(format!("{} - {}", session.branch, session.agent))
            );
        }
        items
    }),
)
.h_full()
```

---

## Files to Change

| File | Action | Justification |
|------|--------|---------------|
| `crates/shards-ui/src/main.rs` | UPDATE | Add session loading and list rendering |

**Note:** PRD suggested separate files for state and shard_list, but YAGNI applies. For Phase 3's read-only list, a single file is simpler. State management and component separation can come in later phases if needed.

---

## NOT Building (Scope Limits)

Explicit exclusions to prevent scope creep:

- **Create/destroy buttons** - Phase 4-5
- **Click interactions** - Phase 4+
- **Refresh button** - Phase 6 (manual reopen is fine for now)
- **Auto-refresh/polling** - Phase 6
- **Favorites** - Phase 7
- **Separate view/state files** - YAGNI for read-only list

---

## Step-by-Step Tasks

Execute in order. Each task is atomic and independently verifiable.

### Task 1: Add shards-core process module re-export

- **ACTION**: Ensure `process::is_process_running` is accessible from shards-core public API
- **FILE**: `crates/shards-core/src/lib.rs`
- **IMPLEMENT**: Add `pub use process::is_process_running;` if not already exported
- **RATIONALE**: UI needs to check process status for Running/Stopped display
- **VALIDATE**: `cargo check -p shards-core`

### Task 2: Define ShardListView struct with session state

- **ACTION**: UPDATE `crates/shards-ui/src/main.rs`
- **IMPLEMENT**:
  - Rename `MainView` to `ShardListView`
  - Add `sessions: Vec<Session>` field
  - Add `ProcessStatus` enum for display (Running/Stopped/Unknown)
  - Add helper struct `ShardDisplay` that combines Session with computed status
- **IMPORTS**:
  ```rust
  use shards_core::{Session, session_ops};
  use shards_core::process::is_process_running;
  ```
- **MIRROR**: Current MainView struct pattern
- **VALIDATE**: `cargo check -p shards-ui`

### Task 3: Load sessions on view creation

- **ACTION**: UPDATE `crates/shards-ui/src/main.rs`
- **IMPLEMENT**:
  - Create `ShardListView::new()` constructor
  - Call `session_ops::list_sessions()` to load sessions
  - For each session, check `is_process_running(pid)` to determine status
  - Store results in view state
  - Handle errors gracefully (empty list on error, log warning)
- **PATTERN**:
  ```rust
  impl ShardListView {
      fn new() -> Self {
          let sessions = match session_ops::list_sessions() {
              Ok(s) => s,
              Err(e) => {
                  tracing::warn!(event = "ui.shard_list.load_failed", error = %e);
                  Vec::new()
              }
          };
          // ... compute display data with process status
          Self { sessions, displays }
      }
  }
  ```
- **GOTCHA**: `list_sessions()` returns `Result` - handle the error case
- **VALIDATE**: `cargo check -p shards-ui`

### Task 4: Implement empty state rendering

- **ACTION**: UPDATE render method in `crates/shards-ui/src/main.rs`
- **IMPLEMENT**:
  - Check if `self.sessions.is_empty()`
  - If empty: show centered text "No active shards"
  - If has sessions: render list (Task 5)
- **PATTERN**:
  ```rust
  fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
      let content = if self.displays.is_empty() {
          div()
              .flex()
              .justify_center()
              .items_center()
              .size_full()
              .text_color(rgb(0x888888))
              .child("No active shards")
      } else {
          // Task 5: list rendering
      };

      div()
          .size_full()
          .bg(rgb(0x1e1e1e))
          .child(header)
          .child(content)
  }
  ```
- **VALIDATE**: `cargo run -p shards-ui` (should show empty state if no shards exist)

### Task 5: Implement shard list rendering with uniform_list

- **ACTION**: UPDATE render method in `crates/shards-ui/src/main.rs`
- **IMPLEMENT**:
  - Use `uniform_list` to render sessions efficiently
  - Each row shows: status icon, branch name, agent, project
  - Status icons: "●" green (0x00ff00) for running, "●" red (0xff0000) for stopped
  - Row layout: horizontal flex with gap
- **PATTERN**:
  ```rust
  uniform_list(
      "shard-list",
      self.displays.len(),
      cx.processor(|this, range, _window, _cx| {
          range.map(|ix| {
              let display = &this.displays[ix];
              let status_color = if display.is_running {
                  rgb(0x00ff00)
              } else {
                  rgb(0xff0000)
              };

              div()
                  .id(ix)
                  .w_full()
                  .px_4()
                  .py_2()
                  .flex()
                  .gap_3()
                  .child(
                      div()
                          .text_color(status_color)
                          .child("●")
                  )
                  .child(
                      div()
                          .flex_1()
                          .text_color(rgb(0xffffff))
                          .child(display.session.branch.clone())
                  )
                  .child(
                      div()
                          .text_color(rgb(0x888888))
                          .child(display.session.agent.clone())
                  )
                  .child(
                      div()
                          .text_color(rgb(0x666666))
                          .child(display.session.project_id.clone())
                  )
          }).collect()
      }),
  )
  .h_full()
  ```
- **GOTCHA**: `uniform_list` requires importing from gpui, check exact import path
- **VALIDATE**:
  1. Create test shards via CLI: `cargo run -p shards -- create test-1`
  2. Run UI: `cargo run -p shards-ui`
  3. Verify shards appear in list with correct status

### Task 6: Add header with title

- **ACTION**: UPDATE render method
- **IMPLEMENT**:
  - Add "Shards" title at top of window
  - Use consistent styling with Phase 2
  - Layout: vertical flex with header + list content
- **PATTERN**:
  ```rust
  div()
      .size_full()
      .flex()
      .flex_col()
      .bg(rgb(0x1e1e1e))
      .child(
          // Header
          div()
              .px_4()
              .py_3()
              .flex()
              .items_center()
              .child(
                  div()
                      .text_xl()
                      .text_color(rgb(0xffffff))
                      .font_weight(FontWeight::BOLD)
                      .child("Shards")
              )
      )
      .child(
          // List content (from Task 4-5)
          div().flex_1().child(list_or_empty)
      )
  ```
- **VALIDATE**: `cargo run -p shards-ui` - title visible at top

---

## Testing Strategy

### Manual Testing (Primary for Phase 3)

| Test Case | Steps | Expected |
|-----------|-------|----------|
| Empty state | 1. Ensure no shards exist (`shards destroy` all) 2. Run `cargo run -p shards-ui` | Shows "No active shards" message |
| Single shard | 1. Create shard: `cargo run -p shards -- create test-1` 2. Run UI | Shows test-1 with green status |
| Multiple shards | 1. Create 2-3 shards 2. Run UI | All shards visible in list |
| Stopped shard | 1. Create shard 2. Manually kill terminal 3. Reopen UI | Shows red status for killed shard |
| Refresh by reopen | 1. UI open 2. Create shard via CLI 3. Close and reopen UI | New shard appears |

### Edge Cases Checklist

- [ ] No sessions directory exists (~/.shards/sessions/ missing) - should show empty
- [ ] Session JSON files corrupt - should skip gracefully (shards-core handles this)
- [ ] Session with no process_id - should show as stopped
- [ ] Many sessions (10+) - should scroll properly
- [ ] Long branch names - should not break layout

---

## Validation Commands

### Level 1: STATIC_ANALYSIS

```bash
cargo fmt --check -p shards-ui && cargo clippy -p shards-ui -- -D warnings
```

**EXPECT**: Exit 0, no errors or warnings

### Level 2: BUILD

```bash
cargo build -p shards-ui
```

**EXPECT**: Clean build with no errors

### Level 3: FULL_SUITE

```bash
cargo test --all && cargo build --all
```

**EXPECT**: All tests pass, full workspace builds

### Level 4: MANUAL_VALIDATION

```bash
# 1. Start fresh
cargo run -p shards -- destroy test-shard-1 2>/dev/null || true
cargo run -p shards -- destroy test-shard-2 2>/dev/null || true

# 2. Test empty state
cargo run -p shards-ui
# VERIFY: Shows "No active shards"

# 3. Create shards (in another terminal, with a git repo)
cd /path/to/any/git/repo
cargo run -p shards -- create test-shard-1
cargo run -p shards -- create test-shard-2

# 4. Reopen UI
cargo run -p shards-ui
# VERIFY: Shows both shards with green running status

# 5. Kill one terminal manually, reopen UI
# VERIFY: Killed shard shows red stopped status

# 6. Cleanup
cargo run -p shards -- destroy test-shard-1
cargo run -p shards -- destroy test-shard-2
```

---

## Acceptance Criteria

- [ ] Window shows "Shards" title at top
- [ ] Empty state shows helpful message when no shards exist
- [ ] All existing shards from `~/.shards/sessions/` are displayed
- [ ] Each shard shows: branch name, agent type, project name
- [ ] Running shards show green status indicator
- [ ] Stopped shards show red status indicator
- [ ] List scrolls if many shards exist
- [ ] `cargo clippy -p shards-ui -- -D warnings` passes
- [ ] `cargo build -p shards-ui` succeeds
- [ ] Manual validation steps pass

---

## Completion Checklist

- [ ] Task 1: process module export verified
- [ ] Task 2: ShardListView struct defined with state
- [ ] Task 3: Session loading implemented
- [ ] Task 4: Empty state renders correctly
- [ ] Task 5: Shard list renders with uniform_list
- [ ] Task 6: Header with title added
- [ ] Level 1: Static analysis passes
- [ ] Level 2: Build succeeds
- [ ] Level 3: Full test suite passes
- [ ] Level 4: Manual validation complete
- [ ] All acceptance criteria met

---

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| uniform_list API differs from example | LOW | MED | Fall back to simple div().children() if needed |
| Process check slow for many shards | LOW | LOW | Current impl is fine, optimize in Phase 6 if needed |
| Session loading errors | LOW | LOW | Already handled gracefully in shards-core |

---

## Notes

**Design Decision: Single File**
The PRD suggested separate `state.rs` and `shard_list.rs` files. For Phase 3's simple read-only list, this is over-engineering. All logic fits cleanly in `main.rs`. If Phase 4+ needs more complexity, we can refactor then.

**Design Decision: No Logging Initialization**
The UI doesn't need `init_logging()` for Phase 3. We're not logging structured events from the UI yet. Can add in Phase 6 when we add status monitoring.

**Design Decision: Eager Loading**
Sessions are loaded once on startup. No auto-refresh. Users reopen the window to see updates. This matches the PRD's "What NOT to do" for Phase 3.

**GPUI uniform_list Import**
Based on research, `uniform_list` is available in GPUI's prelude. If not, check `gpui::elements::uniform_list` or similar. The Zed example shows it's a top-level function.
