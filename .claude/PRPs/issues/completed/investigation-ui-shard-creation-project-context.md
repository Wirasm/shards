# Investigation: UI Shard Creation Missing Project Context + All Projects Filter

**Issue**: Free-form investigation (no GitHub issue)
**Type**: BUG
**Investigated**: 2026-01-26T13:05:00Z

### Assessment

| Metric     | Value  | Reasoning                                                                                                           |
| ---------- | ------ | ------------------------------------------------------------------------------------------------------------------- |
| Severity   | HIGH   | Core feature broken - shards created in UI are not associated with the selected project, making the UI unusable     |
| Complexity | MEDIUM | 4 files affected in shards-ui, 2 files in shards-core; changes are straightforward but span multiple layers         |
| Confidence | HIGH   | Root cause clearly identified through code tracing; `detect_project()` uses `current_dir()` instead of passed path  |

---

## Problem Statement

Two related bugs in the recently-merged multi-project support (Phase 8):

1. **Shard Creation Bug**: When creating a shard in the UI after selecting a project, the shard gets created with the wrong `project_id` because `detect_project()` uses `std::env::current_dir()` (the UI app's working directory) instead of the selected project's path.

2. **All Projects Filter Missing**: Users cannot view "all shards across all projects" in the UI. If a shard was created via CLI in a project that isn't saved in the UI's project list, it won't appear. Need an "All Projects" option in the dropdown.

---

## Analysis

### Root Cause: Shard Creation Bug

**WHY 1**: Why does the shard get created in the wrong project?
↓ BECAUSE: `CreateSessionRequest` doesn't include the project path
Evidence: `crates/shards-ui/src/actions.rs:46`
```rust
let request = CreateSessionRequest::new(branch.to_string(), Some(agent.to_string()), note);
```

**WHY 2**: Why doesn't `CreateSessionRequest` include the project path?
↓ BECAUSE: The struct has no field for it
Evidence: `crates/shards-core/src/sessions/types.rs:104-109`
```rust
pub struct CreateSessionRequest {
    pub branch: String,
    pub agent: Option<String>,
    pub note: Option<String>,
    // NO project_path field!
}
```

**WHY 3**: Why does `create_session` detect the wrong project?
↓ BECAUSE: It calls `detect_project()` which uses `std::env::current_dir()`
Evidence: `crates/shards-core/src/sessions/handler.rs:76-77`
```rust
let project = git::handler::detect_project().map_err(|e| SessionError::GitError { source: e })?;
```

**WHY 4**: Why does `detect_project()` use current directory?
↓ ROOT CAUSE: It was designed for CLI usage where `cwd` IS the project directory
Evidence: `crates/shards-core/src/git/handler.rs:18-23`
```rust
pub fn detect_project() -> Result<ProjectInfo, GitError> {
    let current_dir = std::env::current_dir().map_err(io_error)?;
    let repo = Repository::discover(&current_dir).map_err(|_| GitError::NotInRepository)?;
    // ...
}
```

### Root Cause: All Projects Filter Missing

The UI filters shards by active project in `filtered_displays()`:
Evidence: `crates/shards-ui/src/state.rs:382-392`
```rust
pub fn filtered_displays(&self) -> Vec<&ShardDisplay> {
    let Some(active_id) = self.active_project_id() else {
        // No active project - show all shards
        return self.displays.iter().collect();
    };
    self.displays.iter().filter(|d| d.session.project_id == active_id).collect()
}
```

This works when `active_project` is `None`, but there's no UI way to SET it to `None` after a project is selected. The dropdown only allows selecting specific projects.

### Affected Files

| File                                           | Lines     | Action | Description                                        |
| ---------------------------------------------- | --------- | ------ | -------------------------------------------------- |
| `crates/shards-core/src/sessions/types.rs`     | 104-127   | UPDATE | Add `project_path: Option<PathBuf>` to request     |
| `crates/shards-core/src/sessions/handler.rs`   | 43-77     | UPDATE | Use request's project_path if provided             |
| `crates/shards-core/src/git/handler.rs`        | 18-58     | UPDATE | Add `detect_project_at()` that takes explicit path |
| `crates/shards-ui/src/actions.rs`              | 19-67     | UPDATE | Pass active project path to CreateSessionRequest   |
| `crates/shards-ui/src/views/main_view.rs`      | 108-135   | UPDATE | Pass active_project to actions::create_shard       |
| `crates/shards-ui/src/views/project_selector.rs` | 118-163 | UPDATE | Add "All Projects" option to dropdown              |
| `crates/shards-ui/src/state.rs`                | 251       | N/A    | No change - already handles `None` as "show all"   |

### Integration Points

- `main_view.rs:117` calls `actions::create_shard()`
- `actions.rs:46` calls `CreateSessionRequest::new()`
- `actions.rs:48` calls `session_ops::create_session()`
- `handler.rs:76` calls `git::handler::detect_project()`
- `state.rs:382` `filtered_displays()` already handles `active_project: None` correctly

### Git History

- **Introduced**: `13a3b16` - "Add multi-project support to shards-ui (Phase 8)"
- **Implication**: This is a bug in the new feature, not a regression

---

## Implementation Plan

### Step 1: Add `project_path` field to `CreateSessionRequest`

**File**: `crates/shards-core/src/sessions/types.rs`
**Lines**: 104-127
**Action**: UPDATE

**Current code:**
```rust
#[derive(Debug, Clone)]
pub struct CreateSessionRequest {
    pub branch: String,
    pub agent: Option<String>,
    pub note: Option<String>,
}

impl CreateSessionRequest {
    pub fn new(branch: String, agent: Option<String>, note: Option<String>) -> Self {
        Self {
            branch,
            agent,
            note,
        }
    }
    // ...
}
```

**Required change:**
```rust
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CreateSessionRequest {
    pub branch: String,
    pub agent: Option<String>,
    pub note: Option<String>,
    /// Optional project path for UI context. When provided, this path is used
    /// instead of current working directory for project detection.
    pub project_path: Option<PathBuf>,
}

impl CreateSessionRequest {
    pub fn new(branch: String, agent: Option<String>, note: Option<String>) -> Self {
        Self {
            branch,
            agent,
            note,
            project_path: None,
        }
    }

    /// Create a request with explicit project path (for UI usage)
    pub fn with_project_path(
        branch: String,
        agent: Option<String>,
        note: Option<String>,
        project_path: PathBuf,
    ) -> Self {
        Self {
            branch,
            agent,
            note,
            project_path: Some(project_path),
        }
    }
    // ...existing methods...
}
```

**Why**: Allows UI to specify which project directory to use for shard creation.

---

### Step 2: Add `detect_project_at()` function

**File**: `crates/shards-core/src/git/handler.rs`
**Lines**: After line 58
**Action**: UPDATE (add new function)

**Required change:**
```rust
/// Detect project from a specific path (for UI usage).
///
/// Unlike `detect_project()` which uses current directory, this function
/// uses the provided path to discover the git repository.
pub fn detect_project_at(path: &Path) -> Result<ProjectInfo, GitError> {
    info!(event = "core.git.project.detect_at_started", path = %path.display());

    let repo = Repository::discover(path).map_err(|_| GitError::NotInRepository)?;

    let repo_path = repo.workdir().ok_or_else(|| GitError::OperationFailed {
        message: "Repository has no working directory".to_string(),
    })?;

    let remote_url = repo
        .find_remote("origin")
        .ok()
        .and_then(|remote| remote.url().map(|s| s.to_string()));

    let project_name = if let Some(ref url) = remote_url {
        operations::derive_project_name_from_remote(url)
    } else {
        operations::derive_project_name_from_path(repo_path)
    };

    let project_id = operations::generate_project_id(repo_path);

    let project = ProjectInfo::new(
        project_id.clone(),
        project_name.clone(),
        repo_path.to_path_buf(),
        remote_url.clone(),
    );

    info!(
        event = "core.git.project.detect_at_completed",
        project_id = project_id,
        project_name = project_name,
        repo_path = %repo_path.display(),
        remote_url = remote_url.as_deref().unwrap_or("none")
    );

    Ok(project)
}
```

**Why**: Allows specifying explicit path for project detection instead of relying on cwd.

---

### Step 3: Update `create_session` to use `project_path` if provided

**File**: `crates/shards-core/src/sessions/handler.rs`
**Lines**: 72-77
**Action**: UPDATE

**Current code:**
```rust
// 1. Validate input (pure)
let validated = operations::validate_session_request(&request.branch, &agent_command, &agent)?;

// 2. Detect git project (I/O)
let project = git::handler::detect_project().map_err(|e| SessionError::GitError { source: e })?;
```

**Required change:**
```rust
// 1. Validate input (pure)
let validated = operations::validate_session_request(&request.branch, &agent_command, &agent)?;

// 2. Detect git project (I/O)
// Use explicit project path if provided (UI context), otherwise use cwd (CLI context)
let project = match &request.project_path {
    Some(path) => {
        info!(
            event = "core.session.using_explicit_project_path",
            path = %path.display()
        );
        git::handler::detect_project_at(path)
    }
    None => git::handler::detect_project(),
}
.map_err(|e| SessionError::GitError { source: e })?;
```

**Why**: When UI provides a project path, use that instead of cwd.

---

### Step 4: Update UI `actions::create_shard` to accept project path

**File**: `crates/shards-ui/src/actions.rs`
**Lines**: 19-67
**Action**: UPDATE

**Current code:**
```rust
pub fn create_shard(branch: &str, agent: &str, note: Option<String>) -> Result<Session, String> {
    // ...
    let request = CreateSessionRequest::new(branch.to_string(), Some(agent.to_string()), note);
    // ...
}
```

**Required change:**
```rust
use std::path::PathBuf;

/// Create a new shard with the given branch name, agent, optional note, and optional project path.
///
/// When `project_path` is provided (UI context), creates the shard in that project.
/// When `None` (shouldn't happen in UI), falls back to current directory detection.
pub fn create_shard(
    branch: &str,
    agent: &str,
    note: Option<String>,
    project_path: Option<PathBuf>,
) -> Result<Session, String> {
    tracing::info!(
        event = "ui.create_shard.started",
        branch = branch,
        agent = agent,
        note = ?note,
        project_path = ?project_path
    );

    if branch.trim().is_empty() {
        tracing::warn!(
            event = "ui.create_dialog.validation_failed",
            reason = "empty branch name"
        );
        return Err("Branch name cannot be empty".to_string());
    }

    let config = match ShardsConfig::load_hierarchy() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(
                event = "ui.create_shard.config_load_failed",
                error = %e
            );
            return Err(format!("Failed to load config: {e}"));
        }
    };

    let request = match project_path {
        Some(path) => CreateSessionRequest::with_project_path(
            branch.to_string(),
            Some(agent.to_string()),
            note,
            path,
        ),
        None => CreateSessionRequest::new(branch.to_string(), Some(agent.to_string()), note),
    };

    match session_ops::create_session(request, &config) {
        Ok(session) => {
            tracing::info!(
                event = "ui.create_shard.completed",
                session_id = session.id,
                branch = session.branch
            );
            Ok(session)
        }
        Err(e) => {
            tracing::error!(
                event = "ui.create_shard.failed",
                branch = branch,
                agent = agent,
                error = %e
            );
            Err(e.to_string())
        }
    }
}
```

**Why**: Propagate project path from UI state to core.

---

### Step 5: Update `on_dialog_submit` to pass active project

**File**: `crates/shards-ui/src/views/main_view.rs`
**Lines**: 108-135
**Action**: UPDATE

**Current code:**
```rust
pub fn on_dialog_submit(&mut self, cx: &mut Context<Self>) {
    let branch = self.state.create_form.branch_name.trim().to_string();
    let agent = self.state.create_form.selected_agent();
    let note = if self.state.create_form.note.trim().is_empty() {
        None
    } else {
        Some(self.state.create_form.note.trim().to_string())
    };

    match actions::create_shard(&branch, &agent, note) {
        // ...
    }
}
```

**Required change:**
```rust
pub fn on_dialog_submit(&mut self, cx: &mut Context<Self>) {
    let branch = self.state.create_form.branch_name.trim().to_string();
    let agent = self.state.create_form.selected_agent();
    let note = if self.state.create_form.note.trim().is_empty() {
        None
    } else {
        Some(self.state.create_form.note.trim().to_string())
    };

    // Get active project path for shard creation context
    let project_path = self.state.active_project.clone();

    // Warn if no project selected (shouldn't happen with current UI flow)
    if project_path.is_none() {
        tracing::warn!(
            event = "ui.dialog_submit.no_active_project",
            message = "Creating shard without active project - will use cwd detection"
        );
    }

    match actions::create_shard(&branch, &agent, note, project_path) {
        Ok(_session) => {
            // Success - close dialog and refresh list
            self.state.show_create_dialog = false;
            self.state.reset_create_form();
            self.state.refresh_sessions();
        }
        Err(e) => {
            tracing::warn!(
                event = "ui.dialog_submit.error_displayed",
                branch = %branch,
                agent = %agent,
                error = %e
            );
            self.state.create_error = Some(e);
        }
    }
    cx.notify();
}
```

**Why**: Pass the selected project path to shard creation.

---

### Step 6: Add "All Projects" option to project dropdown

**File**: `crates/shards-ui/src/views/project_selector.rs`
**Lines**: 117-163 (children section)
**Action**: UPDATE

**Current code:**
```rust
// Project list
.children(
    projects_for_dropdown
        .iter()
        .enumerate()
        .map(|(idx, project)| {
            // ... project items
        }),
)
```

**Required change:**
Add an "All Projects" option at the top of the dropdown:
```rust
// "All Projects" option
.child(
    div()
        .id("project-all")
        .px_3()
        .py_2()
        .hover(|style| style.bg(rgb(0x3d3d3d)))
        .cursor_pointer()
        .on_mouse_up(
            gpui::MouseButton::Left,
            cx.listener(|view, _, _, cx| {
                view.on_project_select_all(cx);
            }),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .w(px(16.0))
                        .text_color(if active_for_dropdown.is_none() {
                            rgb(0x4a9eff)
                        } else {
                            rgb(0x444444)
                        })
                        .child(if active_for_dropdown.is_none() { "●" } else { "○" }),
                )
                .child(
                    div()
                        .text_color(rgb(0xffffff))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child("All Projects"),
                ),
        ),
)
// Divider after "All Projects"
.child(div().h(px(1.0)).bg(rgb(0x444444)).mx_2().my_1())
// Project list (existing code)
.children(
    projects_for_dropdown
        .iter()
        .enumerate()
        .map(|(idx, project)| {
            // ... existing project items
        }),
)
```

**Why**: Allows users to clear the active project filter and see all shards.

---

### Step 7: Add handler for "All Projects" selection

**File**: `crates/shards-ui/src/views/main_view.rs`
**Lines**: After `on_project_select` (~line 480)
**Action**: UPDATE (add new method)

**Required change:**
```rust
/// Handle "All Projects" selection from dropdown.
pub fn on_project_select_all(&mut self, cx: &mut Context<Self>) {
    tracing::info!(event = "ui.project_selected_all");

    if let Err(e) = actions::set_active_project(None) {
        tracing::error!(event = "ui.project_select_all.failed", error = %e);
        self.state.add_project_error = Some(format!("Failed to clear project selection: {}", e));
        cx.notify();
        return;
    }

    self.state.active_project = None;
    self.state.show_project_dropdown = false;
    cx.notify();
}
```

**Why**: Handles the "All Projects" dropdown option to clear filtering.

---

### Step 8: Update empty state message when "All Projects" selected

**File**: `crates/shards-ui/src/views/shard_list.rs`
**Lines**: 118-125
**Action**: UPDATE

**Current code:**
```rust
if filtered.is_empty() {
    // Empty state - no shards for the current project
    div()
        .flex()
        .flex_1()
        .justify_center()
        .items_center()
        .text_color(rgb(0x888888))
        .child("No active shards for this project")
}
```

**Required change:**
```rust
if filtered.is_empty() {
    // Empty state - message depends on whether filtering is active
    let message = if state.active_project.is_some() {
        "No active shards for this project"
    } else {
        "No active shards"
    };
    div()
        .flex()
        .flex_1()
        .justify_center()
        .items_center()
        .text_color(rgb(0x888888))
        .child(message)
}
```

**Why**: Different message when showing all projects vs. filtered by project.

---

### Step 9: Add/Update Tests

**File**: `crates/shards-core/src/sessions/types.rs`
**Action**: ADD tests

```rust
#[test]
fn test_create_session_request_with_project_path() {
    let request = CreateSessionRequest::with_project_path(
        "test-branch".to_string(),
        Some("claude".to_string()),
        None,
        PathBuf::from("/path/to/project"),
    );
    assert_eq!(request.branch, "test-branch");
    assert_eq!(request.project_path, Some(PathBuf::from("/path/to/project")));
}

#[test]
fn test_create_session_request_new_has_no_project_path() {
    let request = CreateSessionRequest::new("test-branch".to_string(), None, None);
    assert!(request.project_path.is_none());
}
```

**File**: `crates/shards-ui/src/state.rs`
**Action**: ADD test

```rust
#[test]
fn test_filtered_displays_shows_all_when_active_project_none() {
    // Already exists - verify it passes
}
```

---

## Patterns to Follow

**From codebase - project ID derivation must match:**

```rust
// SOURCE: crates/shards-ui/src/projects.rs:62-66
// Pattern for deriving project ID from path
pub fn derive_project_id(path: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

// SOURCE: crates/shards-core/src/git/operations.rs
// Must use the same algorithm!
pub fn generate_project_id(repo_path: &Path) -> String {
    // Same hash algorithm
}
```

**From codebase - logging pattern:**

```rust
// SOURCE: crates/shards-core/src/sessions/handler.rs
// Event naming: core.session.{action}_{state}
info!(event = "core.session.create_started", branch = request.branch);
info!(event = "core.session.create_completed", session_id = session.id);
```

---

## Edge Cases & Risks

| Risk/Edge Case                    | Mitigation                                                                |
| --------------------------------- | ------------------------------------------------------------------------- |
| UI has no active project selected | Log warning, fall back to cwd detection (existing behavior)               |
| Project path doesn't exist        | `detect_project_at()` will return `NotInRepository` error                 |
| Path normalization mismatch       | Already addressed in separate PR (path normalization investigation)       |
| CLI-created shards with new UI    | "All Projects" option allows seeing all shards regardless of UI projects  |

---

## Validation

### Automated Checks

```bash
cargo fmt --check              # Formatting
cargo clippy --all -- -D warnings  # Linting
cargo test --all               # All tests pass
cargo build --all              # Clean build
```

### Manual Verification

1. **Create shard in UI with project selected**:
   - Add a project (e.g., `/path/to/my-project`)
   - Select that project in dropdown
   - Create a shard
   - Verify shard's `project_id` matches the selected project's hash

2. **All Projects filter**:
   - Create shards via CLI in different directories
   - In UI, select "All Projects" from dropdown
   - Verify all shards appear regardless of UI project list

3. **Create shard via CLI still works**:
   - `cd /path/to/project && shards create test-branch`
   - Verify shard is created with correct project_id

---

## Scope Boundaries

**IN SCOPE:**

- Passing project path from UI to core during shard creation
- Adding "All Projects" dropdown option
- Updating empty state messages

**OUT OF SCOPE (do not touch):**

- CLI behavior (must continue using cwd)
- Automatic project discovery from CLI
- Project path normalization (separate PR in progress)
- Bulk operations (Open All, Stop All) - already work correctly

---

## Metadata

- **Investigated by**: Claude
- **Timestamp**: 2026-01-26T13:05:00Z
- **Artifact**: `.claude/PRPs/issues/investigation-ui-shard-creation-project-context.md`
