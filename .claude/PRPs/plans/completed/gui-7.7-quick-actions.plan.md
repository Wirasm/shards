# Feature: GUI Quick Actions (Per-Row)

## Summary

Add per-row action buttons: Copy Path, Open in Editor, Focus Terminal. These enable quick access to common operations without leaving the UI or using CLI.

## User Story

As a power user viewing my shards, I want quick access to common actions per shard so that I can copy paths, open editors, and focus terminals without CLI.

## Problem Statement

From PRD Phase 7.7: Quick access to common operations without leaving the UI or using CLI. Users managing shards need to:
- Copy worktree paths for shell commands
- Open code in their editor
- Focus terminal windows that may be behind other windows

Currently these require switching to terminal and running CLI commands.

## Solution Statement

Add three action buttons to each shard row:
- **Copy** - Copy worktree path to clipboard
- **Edit** - Open worktree in user's preferred editor ($EDITOR or zed)
- **Focus** - Bring shard's terminal window to foreground (only when running)

## Metadata

| Field | Value |
|-------|-------|
| Type | NEW_CAPABILITY |
| Complexity | MEDIUM |
| Systems Affected | shards-ui |
| Dependencies | CLI cd (done), code (done), focus (done) |
| Estimated Tasks | 5 |

---

## UX Design

### Before State
```
|  Running status  branch         agent    project   time   note   [Stop] [Destroy] |
```

### After State
```
|  Running status  branch         agent    project   time   note   [Copy] [Edit] [Focus] [Stop] [Destroy] |
                                                                     |      |       |
                                                                     |      |       +-- Focus Terminal
                                                                     |      +-- Open in Editor
                                                                     +-- Copy Path
```

Note: Focus button only appears when shard is running (has active terminal).

---

## Mandatory Reading

| Priority | File | Lines | Why Read This |
|----------|------|-------|---------------|
| P0 | `crates/shards-ui/src/views/shard_list.rs` | 46-272 | Row rendering pattern, button patterns |
| P0 | `crates/shards-ui/src/actions.rs` | 1-143 | Action handler patterns |
| P0 | `crates/shards-ui/src/views/main_view.rs` | 168-213 | Click handler patterns |
| P1 | `crates/shards-core/src/terminal/handler.rs` | 350-358 | Focus terminal implementation |
| P1 | `crates/shards/src/commands.rs` | 518-589 | CLI code command pattern |

---

## Patterns to Mirror

**ROW_BUTTON_PATTERN:**
```rust
// SOURCE: crates/shards-ui/src/views/shard_list.rs:188-211
// Open button pattern - shows conditional button with click handler
.when(!is_running, |row| {
    row.child(
        div()
            .id(("open-btn", ix))
            .px_2()
            .py_1()
            .bg(rgb(0x444444))
            .hover(|style| style.bg(rgb(0x555555)))
            .rounded_md()
            .cursor_pointer()
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(move |view, _, _, cx| {
                    view.on_open_click(&branch_for_open, cx);
                }),
            )
            .child(div().text_color(rgb(0xffffff)).child("icon")),
    )
})
```

**MAINVIEW_HANDLER_PATTERN:**
```rust
// SOURCE: crates/shards-ui/src/views/main_view.rs:168-189
pub fn on_open_click(&mut self, branch: &str, cx: &mut Context<Self>) {
    tracing::info!(event = "ui.open_clicked", branch = branch);
    self.state.clear_open_error();

    match actions::open_shard(branch, None) {
        Ok(_session) => {
            self.state.refresh_sessions();
        }
        Err(e) => {
            // error handling...
        }
    }
    cx.notify();
}
```

**GPUI_CLIPBOARD_PATTERN:**
```rust
// Import: use gpui::ClipboardItem;
cx.write_to_clipboard(ClipboardItem::new_string(text));
```

---

## Files to Change

| File | Action | Justification |
|------|--------|---------------|
| `crates/shards-ui/src/views/shard_list.rs` | UPDATE | Add ClipboardItem import, add 3 quick action buttons to each row |
| `crates/shards-ui/src/actions.rs` | UPDATE | Add open_in_editor function |
| `crates/shards-ui/src/views/main_view.rs` | UPDATE | Add click handlers for copy_path, open_editor, focus_terminal |

---

## Step-by-Step Tasks

### Task 1: ADD ClipboardItem import to shard_list.rs

- **ACTION**: Import ClipboardItem for clipboard operations
- **FILE**: `crates/shards-ui/src/views/shard_list.rs`
- **LOCATION**: Line 6
- **IMPLEMENT**:
  ```rust
  // Change from:
  use gpui::{Context, IntoElement, div, prelude::*, rgb, uniform_list};
  // To:
  use gpui::{ClipboardItem, Context, IntoElement, div, prelude::*, rgb, uniform_list};
  ```
- **VALIDATE**: `cargo check -p shards-ui`

### Task 2: ADD open_in_editor function to actions.rs

- **ACTION**: Add function to open worktree in editor
- **FILE**: `crates/shards-ui/src/actions.rs`
- **LOCATION**: After stop_shard() function (after line 142)
- **IMPLEMENT**:
  ```rust
  /// Open a worktree path in the user's preferred editor.
  ///
  /// Editor selection priority:
  /// 1. $EDITOR environment variable
  /// 2. Default: "zed"
  ///
  /// This is a fire-and-forget operation - we spawn the editor and don't wait.
  pub fn open_in_editor(worktree_path: &std::path::Path) {
      let editor = std::env::var("EDITOR").unwrap_or_else(|_| "zed".to_string());

      tracing::info!(
          event = "ui.open_in_editor.started",
          path = %worktree_path.display(),
          editor = %editor
      );

      match std::process::Command::new(&editor)
          .arg(worktree_path)
          .spawn()
      {
          Ok(_) => {
              tracing::info!(
                  event = "ui.open_in_editor.completed",
                  path = %worktree_path.display(),
                  editor = %editor
              );
          }
          Err(e) => {
              tracing::error!(
                  event = "ui.open_in_editor.failed",
                  path = %worktree_path.display(),
                  editor = %editor,
                  error = %e
              );
          }
      }
  }
  ```
- **VALIDATE**: `cargo check -p shards-ui`

### Task 3: ADD quick action handlers to MainView

- **ACTION**: Add three handler methods for copy_path, open_editor, focus_terminal
- **FILE**: `crates/shards-ui/src/views/main_view.rs`
- **LOCATION**: After on_stop_click (around line 213)
- **IMPLEMENT**:
  ```rust
  /// Handle click on the Copy Path button in a shard row.
  pub fn on_copy_path_click(&mut self, worktree_path: &std::path::Path, cx: &mut Context<Self>) {
      tracing::info!(
          event = "ui.copy_path_clicked",
          path = %worktree_path.display()
      );
      let path_str = worktree_path.display().to_string();
      cx.write_to_clipboard(gpui::ClipboardItem::new_string(path_str));
      cx.notify();
  }

  /// Handle click on the Open Editor button in a shard row.
  pub fn on_open_editor_click(&mut self, worktree_path: &std::path::Path, cx: &mut Context<Self>) {
      tracing::info!(
          event = "ui.open_editor_clicked",
          path = %worktree_path.display()
      );
      actions::open_in_editor(worktree_path);
      cx.notify();
  }

  /// Handle click on the Focus Terminal button in a shard row.
  pub fn on_focus_terminal_click(
      &mut self,
      terminal_type: Option<&shards_core::terminal::types::TerminalType>,
      window_id: Option<&str>,
      branch: &str,
      cx: &mut Context<Self>,
  ) {
      tracing::info!(
          event = "ui.focus_terminal_clicked",
          branch = branch,
          terminal_type = ?terminal_type,
          window_id = ?window_id
      );

      match (terminal_type, window_id) {
          (Some(tt), Some(wid)) => {
              if let Err(e) = shards_core::terminal_ops::focus_terminal(tt, wid) {
                  tracing::warn!(
                      event = "ui.focus_terminal_failed",
                      branch = branch,
                      error = %e
                  );
              }
          }
          _ => {
              tracing::warn!(
                  event = "ui.focus_terminal_no_window_info",
                  branch = branch,
                  message = "No terminal type or window ID recorded"
              );
          }
      }
      cx.notify();
  }
  ```
- **VALIDATE**: `cargo check -p shards-ui`

### Task 4: ADD quick action buttons to shard_list.rs row rendering

- **ACTION**: Add Copy, Edit, Focus buttons to each row before Stop/Destroy
- **FILE**: `crates/shards-ui/src/views/shard_list.rs`
- **LOCATION**: Inside the row rendering, after note column (around line 186)

**PART A - Add clones for button closures** (after line 115):
```rust
let worktree_path_for_copy = display.session.worktree_path.clone();
let worktree_path_for_edit = display.session.worktree_path.clone();
let terminal_type_for_focus = display.session.terminal_type.clone();
let window_id_for_focus = display.session.terminal_window_id.clone();
let branch_for_focus = branch.clone();
```

**PART B - Add buttons** (insert before the Open/Stop button, around line 187):
```rust
// Copy Path button [Copy]
.child(
    div()
        .id(("copy-btn", ix))
        .px_2()
        .py_1()
        .bg(rgb(0x444444))
        .hover(|style| style.bg(rgb(0x555555)))
        .rounded_md()
        .cursor_pointer()
        .on_mouse_up(
            gpui::MouseButton::Left,
            cx.listener(move |view, _, _, cx| {
                view.on_copy_path_click(&worktree_path_for_copy, cx);
            }),
        )
        .child(div().text_color(rgb(0xaaaaaa)).text_sm().child("Copy")),
)
// Open in Editor button [Edit]
.child(
    div()
        .id(("edit-btn", ix))
        .px_2()
        .py_1()
        .bg(rgb(0x444444))
        .hover(|style| style.bg(rgb(0x555555)))
        .rounded_md()
        .cursor_pointer()
        .on_mouse_up(
            gpui::MouseButton::Left,
            cx.listener(move |view, _, _, cx| {
                view.on_open_editor_click(&worktree_path_for_edit, cx);
            }),
        )
        .child(div().text_color(rgb(0xaaaaaa)).text_sm().child("Edit")),
)
// Focus Terminal button [Focus] - only show when running
.when(is_running, |row| {
    let tt = terminal_type_for_focus.clone();
    let wid = window_id_for_focus.clone();
    let br = branch_for_focus.clone();
    row.child(
        div()
            .id(("focus-btn", ix))
            .px_2()
            .py_1()
            .bg(rgb(0x444488))
            .hover(|style| style.bg(rgb(0x555599)))
            .rounded_md()
            .cursor_pointer()
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(move |view, _, _, cx| {
                    view.on_focus_terminal_click(
                        tt.as_ref(),
                        wid.as_deref(),
                        &br,
                        cx,
                    );
                }),
            )
            .child(div().text_color(rgb(0xffffff)).child("Focus")),
    )
})
```
- **VALIDATE**: `cargo check -p shards-ui`

### Task 5: Build and lint check

- **ACTION**: Full build and lint check
- **VALIDATE**:
  ```bash
  cargo fmt --check
  cargo clippy --all -- -D warnings
  cargo build -p shards-ui
  ```

---

## Validation Commands

### Level 1: STATIC_ANALYSIS
```bash
cargo fmt --check && cargo clippy --all -- -D warnings
```

### Level 2: BUILD
```bash
cargo build -p shards-ui
```

### Level 3: MANUAL_TEST
```bash
# Create a test shard
cargo run -p shards -- create test-actions --note "Testing quick actions"

# Open UI
cargo run -p shards-ui

# Test each button:
# 1. Click "Copy" button -> paste in terminal, verify path is correct
# 2. Click "Edit" button -> editor opens with worktree
# 3. Click "Focus" button (when running) -> terminal window comes to front

# Cleanup
cargo run -p shards -- destroy test-actions --force
```

---

## Acceptance Criteria

- [ ] Copy button copies worktree path to clipboard
- [ ] Edit button launches $EDITOR (or zed) with worktree path
- [ ] Focus button brings terminal window to foreground (only visible when running)
- [ ] Buttons appear on all shard rows in correct order: Copy, Edit, Focus, Stop/Open, Destroy
- [ ] Consistent styling with existing buttons (gray bg, hover effect)
- [ ] All validation passes (fmt, clippy, build)
- [ ] No new warnings introduced

---

## Completion Checklist

- [ ] ClipboardItem import added to shard_list.rs
- [ ] open_in_editor function added to actions.rs
- [ ] on_copy_path_click handler added to MainView
- [ ] on_open_editor_click handler added to MainView
- [ ] on_focus_terminal_click handler added to MainView
- [ ] Copy button added to row rendering
- [ ] Edit button added to row rendering
- [ ] Focus button added to row rendering (conditional on running)
- [ ] All validation commands pass
