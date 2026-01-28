# Feature: Sidebar Layout (Phase 9.9)

## Summary

Replace the project dropdown in the header with a fixed left sidebar (200px) for project navigation. The layout becomes a 3-column design: sidebar (200px) | kild list (flex:1) | detail panel (320px, conditional). The sidebar displays all projects with kild counts, an "All Projects" option, and add/remove project actions.

## User Story

As a Tōryō (developer managing multiple projects)
I want to see all my projects in a persistent sidebar
So that I can quickly switch between projects without opening a dropdown and see kild counts at a glance

## Problem Statement

The current dropdown approach requires clicking to see the project list, doesn't show kild counts per project, and uses valuable header space. The mockup specifies a sidebar layout that provides persistent project visibility and better screen real estate usage.

## Solution Statement

1. Create a new `sidebar.rs` view component for the left sidebar (200px fixed width)
2. Update `main_view.rs` to use 3-column flex layout: sidebar | content | detail panel
3. Remove the project dropdown from the header
4. Remove the `show_project_dropdown` state field (no longer needed)
5. Keep existing handlers (`on_project_select`, `on_project_select_all`, `on_remove_project`, `on_add_project_click`)

## Metadata

| Field            | Value                                                          |
| ---------------- | -------------------------------------------------------------- |
| Type             | ENHANCEMENT                                                    |
| Complexity       | MEDIUM                                                         |
| Systems Affected | kild-ui: views/main_view.rs, views/sidebar.rs, state.rs        |
| Dependencies     | gpui (existing), Phase 9.8 (detail panel) must be complete     |
| Estimated Tasks  | 5                                                              |

---

## UX Design

### Before State

```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                              BEFORE STATE                                      ║
╠═══════════════════════════════════════════════════════════════════════════════╝
║                                                                               ║
║  ┌─────────────────────────────────────────────────────────────────────────┐  ║
║  │ KILD  [kild ▼]              [Open All] [Stop All] [Refresh] [+ Create]  │  ║
║  │         │                                                                │  ║
║  │    ┌────┴─────────┐                                                     │  ║
║  │    │ ● All Proj   │  (dropdown overlay - click to open)                 │  ║
║  │    │ ○ kild       │                                                     │  ║
║  │    │ ○ api        │                                                     │  ║
║  │    │ + Add Proj   │                                                     │  ║
║  │    │ − Remove     │                                                     │  ║
║  │    └──────────────┘                                                     │  ║
║  ├────────────────────────────────────┬────────────────────────────────────┤  ║
║  │ Kild List (full width - dropdown)  │ Detail Panel (320px)               │  ║
║  │                                    │                                     │  ║
║  │  ● feature-auth  claude  23m       │ feature-auth info...               │  ║
║  │  ● feature-pay   claude  1h        │                                     │  ║
║  └────────────────────────────────────┴────────────────────────────────────┘  ║
║                                                                               ║
║   USER_FLOW: Click dropdown → see projects → click project → dropdown closes  ║
║   PAIN_POINT: No kild counts, requires click to see options, uses header space║
║   DATA_FLOW: Click → toggle dropdown → select → close dropdown → filter list  ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

### After State

```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                               AFTER STATE                                      ║
╠═══════════════════════════════════════════════════════════════════════════════╝
║                                                                               ║
║  ┌─────────────────────────────────────────────────────────────────────────┐  ║
║  │ KILD                        [Open All] [Stop All] [Refresh] [+ Create]  │  ║
║  ├───────────┬─────────────────────────┬───────────────────────────────────┤  ║
║  │ SCOPE     │ kild — 8 kilds          │ feature-auth           (320px)    │  ║
║  │ (200px)   │                         │ ● Active                          │  ║
║  │───────────│                         │                                    │  ║
║  │▌● All (18)│  ▌● feature-auth        │ NOTE                              │  ║
║  │           │    +42 -12 claude 23m   │ JWT auth implementation...        │  ║
║  │  [K] kild │    JWT auth...          │                                    │  ║
║  │      (8)  │                         │ DETAILS                            │  ║
║  │  [A] api  │  ● feature-pay          │ Agent: claude                      │  ║
║  │      (4)  │    +156 -34 claude 1h   │ Status: Running                    │  ║
║  │  [W] web  │    Refund...            │                                    │  ║
║  │      (3)  │                         │ GIT STATUS                         │  ║
║  │  [D] data │  ○ fix-login            │ Changes: Uncommitted               │  ║
║  │      (2)  │    kiro 4h ago          │                                    │  ║
║  │  [I] infra│    Issue #234           │ PATH                               │  ║
║  │      (1)  │                         │ ~/.kilds/kild/feature-auth         │  ║
║  │           │                         ├───────────────────────────────────┤  ║
║  │───────────│                         │ [Copy Path] [Open Editor]          │  ║
║  │[+ Add Proj]                         │ [Focus Term] [⏹ Stop]             │  ║
║  │[− Remove] │                         │ [🗑 Destroy Kild]                  │  ║
║  └───────────┴─────────────────────────┴───────────────────────────────────┘  ║
║                                                                               ║
║   USER_FLOW: See all projects always → click to select → list filters         ║
║   VALUE_ADD: Kild counts visible, no dropdown needed, cleaner header          ║
║   DATA_FLOW: Click project → on_project_select() → filter list immediately    ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

### Interaction Changes

| Location | Before | After | User Impact |
|----------|--------|-------|-------------|
| Header | Project dropdown button | No project selector | Cleaner header |
| Left side | None | 200px sidebar | Always-visible project list |
| Project selection | Click dropdown → select | Click sidebar item | Faster, no overlay |
| Kild counts | Not shown | Badge per project | At-a-glance overview |
| Add project | In dropdown | Sidebar footer button | Persistent access |
| Remove project | In dropdown | Sidebar footer (conditional) | Persistent access |

---

## Mandatory Reading

**CRITICAL: Implementation agent MUST read these files before starting any task:**

| Priority | File | Lines | Why Read This |
|----------|------|-------|---------------|
| P0 | `crates/kild-ui/src/views/project_selector.rs` | 1-279 | Current dropdown - migrate logic to sidebar |
| P0 | `crates/kild-ui/src/views/main_view.rs` | 780-930 | Current layout - must update to 3-column |
| P0 | `crates/kild-ui/src/state.rs` | 239-277 | AppState - remove show_project_dropdown |
| P1 | `crates/kild-ui/src/views/detail_panel.rs` | all | Detail panel pattern to mirror for sidebar |
| P1 | `crates/kild-ui/src/theme.rs` | 36-76 | Colors: obsidian, border_subtle, ice, etc. |
| P2 | `mockup-dashboard.html` | 226-298 | CSS specs for sidebar styling |
| P2 | `mockup-dashboard.html` | 797-830 | HTML structure for sidebar |

**External Documentation:**
| Source | Section | Why Needed |
|--------|---------|------------|
| Mockup | `.sidebar` CSS (lines 226-298) | Exact styling specs |
| Mockup | `<aside class="sidebar">` (lines 797-830) | HTML structure |

---

## Patterns to Mirror

**SIDEBAR_STRUCTURE_PATTERN:**
```rust
// SOURCE: crates/kild-ui/src/views/detail_panel.rs (sidebar uses same pattern)
// Fixed width sidebar with header, scrollable content, footer
div()
    .w(px(200.0))        // Fixed sidebar width from mockup
    .h_full()
    .bg(theme::obsidian())
    .border_r_1()        // Right border (detail panel has border_l_1)
    .border_color(theme::border_subtle())
    .flex()
    .flex_col()
    .child(/* header */)
    .child(/* scrollable content */)
    .child(/* footer */)
```

**PROJECT_ITEM_PATTERN:**
```rust
// SOURCE: crates/kild-ui/src/views/project_selector.rs:174-209
// Project item with click handler
div()
    .id(("project-item", idx))
    .px(px(theme::SPACE_4))
    .py(px(theme::SPACE_2))
    .cursor_pointer()
    .hover(|style| style.bg(theme::surface()))
    // Selected state: ice left border + surface bg
    .when(is_selected, |row| {
        row.bg(theme::surface())
            .border_l_2()
            .border_color(theme::ice())
            .pl(px(theme::SPACE_4 - 2.0))  // Compensate for border
    })
    .on_mouse_up(gpui::MouseButton::Left, {
        let path = path.clone();
        cx.listener(move |view, _, _, cx| {
            view.on_project_select(path.clone(), cx);
        })
    })
```

**PROJECT_ICON_PATTERN:**
```rust
// SOURCE: mockup-dashboard.html:268-278 → Rust translation
// 16x16 icon with first letter, border bg, rounded
div()
    .size(px(16.0))
    .bg(theme::border())
    .rounded(px(theme::RADIUS_SM))
    .flex()
    .items_center()
    .justify_center()
    .text_size(px(10.0))
    .text_color(theme::text_muted())
    .child(name.chars().next().unwrap_or('?').to_uppercase().to_string())
```

**KILD_COUNT_BADGE_PATTERN:**
```rust
// SOURCE: mockup-dashboard.html:286-292 → Rust translation
// Small pill badge with count
div()
    .text_size(px(theme::TEXT_XS))
    .text_color(theme::text_muted())
    .bg(theme::border_subtle())
    .px(px(6.0))
    .py(px(2.0))
    .rounded(px(10.0))
    .child(count.to_string())
```

**SECTION_HEADER_PATTERN:**
```rust
// SOURCE: crates/kild-ui/src/views/detail_panel.rs render_section pattern
// "SCOPE" header styling
div()
    .px(px(theme::SPACE_4))
    .py(px(theme::SPACE_3))
    .border_b_1()
    .border_color(theme::border_subtle())
    .text_size(px(theme::TEXT_XS))
    .font_weight(gpui::FontWeight::SEMIBOLD)
    .text_color(theme::text_muted())
    .child("SCOPE")
```

**CLICK_HANDLER_PATTERN:**
```rust
// SOURCE: crates/kild-ui/src/views/project_selector.rs:180-185
// Click handler with cloned path
.on_mouse_up(gpui::MouseButton::Left, {
    let path = path.clone();
    cx.listener(move |view, _, _, cx| {
        view.on_project_select(path.clone(), cx);
    })
})
```

---

## Files to Change

| File | Action | Justification |
|------|--------|---------------|
| `crates/kild-ui/src/views/sidebar.rs` | CREATE | New sidebar component |
| `crates/kild-ui/src/views/mod.rs` | UPDATE | Export sidebar module |
| `crates/kild-ui/src/views/main_view.rs` | UPDATE | Remove dropdown, add 3-column layout |
| `crates/kild-ui/src/state.rs` | UPDATE | Remove `show_project_dropdown` field |
| `crates/kild-ui/src/views/project_selector.rs` | DELETE | No longer needed (logic moves to sidebar) |

---

## NOT Building (Scope Limits)

Explicit exclusions to prevent scope creep:

- **Collapsible sidebar** - Fixed 200px, not collapsible
- **Sidebar resize** - No drag-to-resize
- **Project reordering** - Projects shown in order added, no drag reorder
- **Keyboard navigation in sidebar** - Phase 10 handles keyboard shortcuts
- **Project search/filter** - Show all projects, no search box
- **Multi-select projects** - Single project filter only

---

## Step-by-Step Tasks

Execute in order. Each task is atomic and independently verifiable.

### Task 1: UPDATE `crates/kild-ui/src/state.rs` - Remove dropdown state

- **ACTION**: Remove the `show_project_dropdown` field
- **IMPLEMENT**:
  1. Remove this line from `AppState` struct (around line 276):
     ```rust
     // REMOVE: pub show_project_dropdown: bool,
     ```
  2. Remove from `Default` impl or `new()` initialization if present
  3. Add helper method to count kilds per project:
     ```rust
     impl AppState {
         /// Count kilds for a specific project (by project_id)
         pub fn kild_count_for_project(&self, project_path: &Path) -> usize {
             let project_id = crate::projects::derive_project_id(project_path);
             self.displays
                 .iter()
                 .filter(|d| d.session.project_id == project_id)
                 .count()
         }

         /// Count total kilds across all projects
         pub fn total_kild_count(&self) -> usize {
             self.displays.len()
         }
     }
     ```
- **MIRROR**: Other helper methods in `impl AppState`
- **GOTCHA**: Search for any `show_project_dropdown` references and remove
- **VALIDATE**: `cargo build -p kild-ui` - should fail with missing field errors (expected, fix in later tasks)

### Task 2: CREATE `crates/kild-ui/src/views/sidebar.rs` - New sidebar component

- **ACTION**: Create the sidebar render function
- **IMPLEMENT**: Full sidebar matching mockup structure:
  ```rust
  //! Project sidebar component.
  //!
  //! Fixed left sidebar (200px) for project navigation.

  use std::path::PathBuf;

  use gpui::{div, prelude::*, px, Context, FontWeight, IntoElement, ParentElement, Styled};

  use crate::components::{Button, ButtonVariant};
  use crate::projects::Project;
  use crate::state::AppState;
  use crate::theme;
  use crate::views::main_view::MainView;

  /// Width of the sidebar in pixels (from mockup)
  pub const SIDEBAR_WIDTH: f32 = 200.0;

  pub fn render_sidebar(state: &AppState, cx: &mut Context<MainView>) -> impl IntoElement {
      let projects = &state.projects;
      let active_project = &state.active_project;
      let total_count = state.total_kild_count();

      div()
          .w(px(SIDEBAR_WIDTH))
          .h_full()
          .bg(theme::obsidian())
          .border_r_1()
          .border_color(theme::border_subtle())
          .flex()
          .flex_col()
          // Header: "SCOPE"
          .child(
              div()
                  .px(px(theme::SPACE_4))
                  .py(px(theme::SPACE_3))
                  .border_b_1()
                  .border_color(theme::border_subtle())
                  .text_size(px(theme::TEXT_XS))
                  .font_weight(FontWeight::SEMIBOLD)
                  .text_color(theme::text_muted())
                  .child("SCOPE"),
          )
          // Scrollable content
          .child(
              div()
                  .flex_1()
                  .overflow_y_scroll()
                  // "All Projects" option
                  .child(render_all_projects_item(active_project.is_none(), total_count, cx))
                  // Project list
                  .children(
                      projects.iter().enumerate().map(|(idx, project)| {
                          let is_selected = active_project.as_ref() == Some(&project.path().to_path_buf());
                          let count = state.kild_count_for_project(project.path());
                          render_project_item(project, idx, is_selected, count, cx)
                      })
                  )
          )
          // Footer: Add Project button (and Remove if project selected)
          .child(render_sidebar_footer(active_project, cx))
  }

  fn render_all_projects_item(
      is_selected: bool,
      count: usize,
      cx: &mut Context<MainView>,
  ) -> impl IntoElement {
      div()
          .id("sidebar-all-projects")
          .flex()
          .items_center()
          .gap(px(theme::SPACE_2))
          .px(px(theme::SPACE_4))
          .py(px(theme::SPACE_2))
          .cursor_pointer()
          .hover(|style| style.bg(theme::surface()))
          .when(is_selected, |this| {
              this.bg(theme::surface())
                  .border_l_2()
                  .border_color(theme::ice())
                  .pl(px(theme::SPACE_4 - 2.0))
          })
          .on_mouse_up(
              gpui::MouseButton::Left,
              cx.listener(|view, _, _, cx| {
                  view.on_project_select_all(cx);
              }),
          )
          // Radio indicator
          .child(
              div()
                  .w(px(16.0))
                  .text_color(if is_selected { theme::ice() } else { theme::border() })
                  .child(if is_selected { "●" } else { "○" }),
          )
          // "All" text
          .child(
              div()
                  .flex_1()
                  .text_size(px(theme::TEXT_SM))
                  .text_color(theme::text())
                  .child("All"),
          )
          // Count badge
          .child(render_count_badge(count))
  }

  fn render_project_item(
      project: &Project,
      idx: usize,
      is_selected: bool,
      count: usize,
      cx: &mut Context<MainView>,
  ) -> impl IntoElement {
      let path = project.path().to_path_buf();
      let name = project.name().to_string();
      let first_char = name
          .chars()
          .next()
          .unwrap_or('?')
          .to_uppercase()
          .to_string();

      div()
          .id(("sidebar-project", idx))
          .flex()
          .items_center()
          .gap(px(theme::SPACE_2))
          .px(px(theme::SPACE_4))
          .py(px(theme::SPACE_2))
          .cursor_pointer()
          .hover(|style| style.bg(theme::surface()))
          .when(is_selected, |this| {
              this.bg(theme::surface())
                  .border_l_2()
                  .border_color(theme::ice())
                  .pl(px(theme::SPACE_4 - 2.0))
          })
          .on_mouse_up(gpui::MouseButton::Left, {
              let path = path.clone();
              cx.listener(move |view, _, _, cx| {
                  view.on_project_select(path.clone(), cx);
              })
          })
          // Project icon (first letter)
          .child(
              div()
                  .size(px(16.0))
                  .bg(theme::border())
                  .rounded(px(theme::RADIUS_SM))
                  .flex()
                  .items_center()
                  .justify_center()
                  .text_size(px(10.0))
                  .text_color(theme::text_muted())
                  .child(first_char),
          )
          // Project name
          .child(
              div()
                  .flex_1()
                  .text_size(px(theme::TEXT_SM))
                  .text_color(theme::text())
                  .overflow_hidden()
                  .text_ellipsis()
                  .child(name),
          )
          // Count badge
          .child(render_count_badge(count))
  }

  fn render_count_badge(count: usize) -> impl IntoElement {
      div()
          .text_size(px(theme::TEXT_XS))
          .text_color(theme::text_muted())
          .bg(theme::border_subtle())
          .px(px(6.0))
          .py(px(2.0))
          .rounded(px(10.0))
          .child(count.to_string())
  }

  fn render_sidebar_footer(
      active_project: &Option<PathBuf>,
      cx: &mut Context<MainView>,
  ) -> impl IntoElement {
      div()
          .px(px(theme::SPACE_4))
          .py(px(theme::SPACE_3))
          .border_t_1()
          .border_color(theme::border_subtle())
          .flex()
          .flex_col()
          .gap(px(theme::SPACE_2))
          // Add Project button
          .child(
              Button::new("sidebar-add-project", "+ Add Project")
                  .variant(ButtonVariant::Ghost)
                  .on_click(cx.listener(|view, _, _, cx| {
                      view.on_add_project_click(cx);
                  })),
          )
          // Remove current (only if project selected)
          .when_some(active_project.clone(), |this, path| {
              this.child(
                  div()
                      .id("sidebar-remove-project")
                      .w_full()
                      .px(px(theme::SPACE_3))
                      .py(px(theme::SPACE_2))
                      .rounded(px(theme::RADIUS_MD))
                      .cursor_pointer()
                      .hover(|style| style.bg(theme::surface()))
                      .on_mouse_up(gpui::MouseButton::Left, {
                          cx.listener(move |view, _, _, cx| {
                              view.on_remove_project(path.clone(), cx);
                          })
                      })
                      .child(
                          div()
                              .flex()
                              .items_center()
                              .justify_center()
                              .gap(px(theme::SPACE_1))
                              .text_size(px(theme::TEXT_SM))
                              .text_color(theme::ember())
                              .child("−")
                              .child("Remove current"),
                      ),
              )
          })
  }
  ```
- **MIRROR**: `detail_panel.rs` structure and `project_selector.rs` click handlers
- **GOTCHA**: Clone `path` before moving into click listeners
- **GOTCHA**: Use `pl(px(theme::SPACE_4 - 2.0))` to compensate for 2px left border
- **VALIDATE**: `cargo build -p kild-ui`

### Task 3: UPDATE `crates/kild-ui/src/views/mod.rs` - Export sidebar module

- **ACTION**: Add module export
- **IMPLEMENT**:
  ```rust
  pub mod sidebar;
  ```
- **ALSO**: Remove `project_selector` export if present (will delete file in Task 5)
- **MIRROR**: Other module exports in the file
- **VALIDATE**: `cargo build -p kild-ui`

### Task 4: UPDATE `crates/kild-ui/src/views/main_view.rs` - 3-column layout

- **ACTION**: Update render to use sidebar, remove dropdown from header
- **IMPLEMENT**:

  1. Add import at top:
     ```rust
     use super::sidebar;
     ```

  2. In the `Render` impl, modify the layout structure:
     - Remove `project_selector::render_project_selector(&self.state, cx)` from header
     - Change the main content area to 3-column flex layout

  The new render structure should be:
  ```rust
  impl Render for MainView {
      fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
          let stopped_count = self.state.stopped_count();
          let running_count = self.state.running_count();

          div()
              .track_focus(&self.focus_handle)
              .on_key_down(cx.listener(Self::on_key_down))
              .size_full()
              .flex()
              .flex_col()
              .bg(theme::void())
              // Header (NO project dropdown)
              .child(
                  div()
                      .px(px(theme::SPACE_4))
                      .py(px(theme::SPACE_3))
                      .flex()
                      .items_center()
                      .justify_between()
                      .child(
                          div()
                              .text_size(px(theme::TEXT_XL))
                              .text_color(theme::text_white())
                              .font_weight(FontWeight::BOLD)
                              .child("KILD"),
                      )
                      .child(
                          div()
                              .flex()
                              .items_center()
                              .gap(px(theme::SPACE_2))
                              // Open All, Stop All, Refresh, Create buttons
                              // ... (keep existing button code)
                      ),
              )
              // Bulk errors banner (keep as-is)
              .when(!self.state.bulk_errors.is_empty(), |this| {
                  // ... existing bulk errors code
              })
              // Main content: 3-column layout
              .child(
                  div()
                      .flex_1()
                      .flex()
                      .overflow_hidden()
                      // Sidebar (200px fixed)
                      .child(sidebar::render_sidebar(&self.state, cx))
                      // Kild list (flex:1)
                      .child(
                          div()
                              .flex_1()
                              .overflow_hidden()
                              .child(kild_list::render_kild_list(&self.state, cx))
                      )
                      // Detail panel (320px, conditional)
                      .when(self.state.selected_kild_id.is_some(), |this| {
                          this.child(detail_panel::render_detail_panel(&self.state, cx))
                      })
              )
              // Dialogs (keep as-is)
              .when(self.state.show_create_dialog, |this| {
                  this.child(create_dialog::render_create_dialog(&self.state, cx))
              })
              .when(self.state.show_confirm_dialog, |this| {
                  this.child(confirm_dialog::render_confirm_dialog(&self.state, cx))
              })
              .when(self.state.show_add_project_dialog, |this| {
                  this.child(add_project_dialog::render_add_project_dialog(
                      &self.state,
                      cx,
                  ))
              })
      }
  }
  ```

  3. Remove `on_toggle_project_dropdown` handler (no longer needed):
     - Delete the handler function from `impl MainView`
     - Remove any references to it

  4. Update existing handlers to NOT set `show_project_dropdown`:
     - In `on_project_select()`: Remove `self.state.show_project_dropdown = false;`
     - In `on_project_select_all()`: Remove `self.state.show_project_dropdown = false;`
     - In `on_remove_project()`: Remove `self.state.show_project_dropdown = false;`

- **MIRROR**: Existing flex layout patterns in the file
- **GOTCHA**: Remove ALL references to `show_project_dropdown`
- **GOTCHA**: Keep the project dropdown import removed (will delete file in Task 5)
- **VALIDATE**: `cargo build -p kild-ui`

### Task 5: DELETE `crates/kild-ui/src/views/project_selector.rs` - Remove old dropdown

- **ACTION**: Delete the file entirely
- **IMPLEMENT**: `rm crates/kild-ui/src/views/project_selector.rs`
- **ALSO**: Ensure `mod.rs` no longer exports it (done in Task 3)
- **GOTCHA**: Ensure all handlers were migrated to sidebar (they use the same MainView handlers)
- **VALIDATE**: `cargo build -p kild-ui && cargo run -p kild-ui`

---

## Testing Strategy

### Manual Test Cases

| Test Case | Steps | Expected Result |
|-----------|-------|-----------------|
| Sidebar visible | Open app | 200px sidebar on left with "SCOPE" header |
| All Projects works | Click "All" in sidebar | All kilds shown, "All" has ice border |
| Project select works | Click a project | Only that project's kilds shown, ice border on project |
| Kild counts accurate | Check badges | Each project badge shows correct kild count |
| Add project works | Click "+ Add Project" | Add project dialog opens |
| Remove project works | Select project, click "Remove current" | Project removed from list |
| Layout responsive | Resize window | Sidebar stays 200px, list takes remaining space |

### Edge Cases Checklist

- [ ] No projects: Sidebar shows "All" only, footer has "+ Add Project"
- [ ] Single project: Selecting it shows ice border
- [ ] Many projects: Sidebar content scrolls
- [ ] Long project name: Text truncates with ellipsis
- [ ] Zero kilds for project: Badge shows "0"
- [ ] Remove last project: Falls back to "All Projects"

---

## Validation Commands

### Level 1: STATIC_ANALYSIS

```bash
cargo fmt --check && cargo clippy -p kild-ui -- -D warnings
```

**EXPECT**: Exit 0, no errors or warnings

### Level 2: BUILD

```bash
cargo build -p kild-ui
```

**EXPECT**: Clean build, no errors

### Level 3: FULL_SUITE

```bash
cargo test --all && cargo build --all
```

**EXPECT**: All tests pass, full workspace builds

### Level 4: VISUAL_VALIDATION

```bash
cargo run -p kild-ui
```

Manual steps:
1. Verify sidebar appears on left (200px width)
2. Verify "SCOPE" header is uppercase, muted text
3. Verify "All" option at top with kild count badge
4. Verify projects show icon (first letter), name, count
5. Click "All" - verify ice left border appears
6. Click a project - verify selection changes, list filters
7. Verify "+ Add Project" button in footer
8. Select a project - verify "Remove current" appears in footer
9. Click "Remove current" - verify project is removed

---

## Acceptance Criteria

- [ ] Sidebar is 200px fixed width on left
- [ ] Header shows "SCOPE" (uppercase, muted, semibold)
- [ ] "All" option shows total kild count and is selectable
- [ ] Each project shows: icon (first letter), name, kild count
- [ ] Selected item has ice left border and surface background
- [ ] "+ Add Project" button in footer opens dialog
- [ ] "Remove current" appears when project selected
- [ ] Dropdown is completely removed from header
- [ ] Layout is 3-column when detail panel visible
- [ ] Level 1-3 validation passes

---

## Completion Checklist

- [ ] Task 1: `show_project_dropdown` removed, kild count helpers added
- [ ] Task 2: `sidebar.rs` created
- [ ] Task 3: Module exported, `project_selector` removed from exports
- [ ] Task 4: Main view updated to 3-column layout
- [ ] Task 5: `project_selector.rs` deleted
- [ ] Level 1: cargo fmt + clippy passes
- [ ] Level 2: cargo build -p kild-ui succeeds
- [ ] Level 3: cargo test --all passes
- [ ] Level 4: Visual validation complete
- [ ] All acceptance criteria met

---

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Handler references break | LOW | MED | Handlers stay in main_view.rs, just called from sidebar |
| Layout breaks with detail panel | LOW | MED | Use same flex pattern as Phase 9.8 |
| Kild counts stale | LOW | LOW | Counts recalculated on every render from displays |
| Long project names overflow | MED | LOW | Use text_ellipsis and overflow_hidden |

---

## Notes

- Sidebar width (200px) and detail panel width (320px) are from mockup specs
- All handlers (`on_project_select`, `on_project_select_all`, `on_remove_project`, `on_add_project_click`) already exist in `main_view.rs` - just remove dropdown-specific code
- The `show_project_dropdown` state is completely removed, not replaced
- Project icon uses first character of name, uppercase, in a styled 16x16 box
- Kild count badges use the same styling as in the mockup
- This phase depends on Phase 9.8 being complete for the detail panel integration
