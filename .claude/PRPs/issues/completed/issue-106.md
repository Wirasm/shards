# Investigation: Tests write to real ~/.kild/projects.json instead of test-specific location

**Issue**: #106 (https://github.com/Wirasm/kild/issues/106)
**Type**: BUG
**Investigated**: 2026-01-28T12:00:00Z

### Assessment

| Metric     | Value  | Reasoning                                                                                                    |
| ---------- | ------ | ------------------------------------------------------------------------------------------------------------ |
| Severity   | MEDIUM | Test pollution affects user data but has workaround (manually clean projects.json); no data loss, just noise |
| Complexity | LOW    | Single file change (projects.rs), isolated to path determination function, no architectural changes          |
| Confidence | HIGH   | Root cause is clear (hardcoded path), fix pattern established in kild-core with env vars                     |

---

## Problem Statement

Unit tests in `kild-ui` call `add_project()`, `remove_project()`, and `set_active_project()` which internally use `load_projects()` and `save_projects()`. These functions use `projects_file_path()` which hardcodes the path to `~/.kild/projects.json` with no test override capability. This pollutes the user's real project list with temporary test artifacts.

---

## Analysis

### Root Cause

WHY: Test artifacts appear in real `~/.kild/projects.json`
↓ BECAUSE: `test_add_project_uses_provided_name()` and `test_add_project_derives_name_from_path()` call `add_project()`
Evidence: `crates/kild-ui/src/actions.rs:700` - `super::add_project(path.to_path_buf(), Some("Custom Name".to_string()))`

↓ BECAUSE: `add_project()` calls `save_projects()` which writes to a hardcoded path
Evidence: `crates/kild-ui/src/actions.rs:278` - `save_projects(&data)?;`

↓ BECAUSE: `save_projects()` uses `projects_file_path()` which always returns `~/.kild/projects.json`
Evidence: `crates/kild-ui/src/projects.rs:220` - `let path = projects_file_path();`

↓ ROOT CAUSE: `projects_file_path()` has no test override mechanism
Evidence: `crates/kild-ui/src/projects.rs:311-323`
```rust
fn projects_file_path() -> PathBuf {
    match dirs::home_dir() {
        Some(home) => home.join(".kild").join("projects.json"),
        None => {
            tracing::error!(
                event = "ui.projects.home_dir_not_found",
                fallback = ".",
                "Could not determine home directory - using current directory as fallback"
            );
            PathBuf::from(".").join(".kild").join("projects.json")
        }
    }
}
```

### Evidence Chain

The root cause is the hardcoded path with no test isolation capability.

### Affected Files

| File                            | Lines   | Action | Description                                   |
| ------------------------------- | ------- | ------ | --------------------------------------------- |
| `crates/kild-ui/src/projects.rs` | 311-323 | UPDATE | Add env var override for projects file path   |
| `crates/kild-ui/src/actions.rs`  | 686-733 | UPDATE | Update tests to use env var for test isolation |

### Integration Points

- `crates/kild-ui/src/actions.rs:254` - `load_projects()` called by `add_project()`
- `crates/kild-ui/src/actions.rs:278` - `save_projects()` called by `add_project()`
- `crates/kild-ui/src/actions.rs:296` - `load_projects()` called by `remove_project()`
- `crates/kild-ui/src/actions.rs:310` - `save_projects()` called by `remove_project()`
- `crates/kild-ui/src/actions.rs:327` - `load_projects()` called by `set_active_project()`
- `crates/kild-ui/src/actions.rs:338` - `save_projects()` called by `set_active_project()`
- `crates/kild-ui/src/projects.rs:252` - `load_projects()` + `save_projects()` called by `migrate_projects_to_canonical()`
- `crates/kild-ui/src/state.rs:297` - `load_projects()` called during app initialization

### Git History

- **Last modified**: 160314d - Rebrand Shards to KILD (#110)
- **Implication**: Original bug from initial implementation, not a regression

---

## Implementation Plan

### Step 1: Add KILD_PROJECTS_FILE environment variable override

**File**: `crates/kild-ui/src/projects.rs`
**Lines**: 311-323
**Action**: UPDATE

**Current code:**
```rust
fn projects_file_path() -> PathBuf {
    match dirs::home_dir() {
        Some(home) => home.join(".kild").join("projects.json"),
        None => {
            tracing::error!(
                event = "ui.projects.home_dir_not_found",
                fallback = ".",
                "Could not determine home directory - using current directory as fallback"
            );
            PathBuf::from(".").join(".kild").join("projects.json")
        }
    }
}
```

**Required change:**
```rust
fn projects_file_path() -> PathBuf {
    // Allow override via env var for testing
    if let Ok(path) = std::env::var("KILD_PROJECTS_FILE") {
        return PathBuf::from(path);
    }

    match dirs::home_dir() {
        Some(home) => home.join(".kild").join("projects.json"),
        None => {
            tracing::error!(
                event = "ui.projects.home_dir_not_found",
                fallback = ".",
                "Could not determine home directory - using current directory as fallback"
            );
            PathBuf::from(".").join(".kild").join("projects.json")
        }
    }
}
```

**Why**: This follows the established pattern in kild-core (KILD_LOG_LEVEL, KILD_DEFAULT_PORT_COUNT, KILD_BASE_PORT_RANGE) and provides a clean override mechanism for tests without modifying the default behavior.

---

### Step 2: Update test_add_project_uses_provided_name to use isolated file

**File**: `crates/kild-ui/src/actions.rs`
**Lines**: 686-708
**Action**: UPDATE

**Current code:**
```rust
#[test]
fn test_add_project_uses_provided_name() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .expect("git init failed");

    let result = super::add_project(path.to_path_buf(), Some("Custom Name".to_string()));

    // This will actually save to the real projects file, so we need to check the returned project
    // If it succeeds, it should have the custom name
    if let Ok(project) = result {
        assert_eq!(project.name(), "Custom Name");
    }
    // If it fails due to file system issues, that's acceptable for this test
}
```

**Required change:**
```rust
#[test]
fn test_add_project_uses_provided_name() {
    use tempfile::TempDir;

    // Use isolated projects file for test
    let projects_dir = TempDir::new().unwrap();
    let projects_file = projects_dir.path().join("projects.json");
    std::env::set_var("KILD_PROJECTS_FILE", &projects_file);

    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .expect("git init failed");

    let result = super::add_project(path.to_path_buf(), Some("Custom Name".to_string()));

    // Clean up env var
    std::env::remove_var("KILD_PROJECTS_FILE");

    let project = result.expect("add_project should succeed");
    assert_eq!(project.name(), "Custom Name");
}
```

**Why**: Test now writes to an isolated temp file that gets cleaned up automatically when `projects_dir` goes out of scope.

---

### Step 3: Update test_add_project_derives_name_from_path to use isolated file

**File**: `crates/kild-ui/src/actions.rs`
**Lines**: 710-733
**Action**: UPDATE

**Current code:**
```rust
#[test]
fn test_add_project_derives_name_from_path() {
    use tempfile::TempDir;

    // Create a temp dir with a specific name
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .expect("git init failed");

    let result = super::add_project(path.to_path_buf(), None);

    // If it succeeds, the name should be derived from the path
    if let Ok(project) = result {
        // Name should be the directory name (temp dir names are random)
        assert!(!project.name().is_empty());
        assert_ne!(project.name(), "unknown");
    }
}
```

**Required change:**
```rust
#[test]
fn test_add_project_derives_name_from_path() {
    use tempfile::TempDir;

    // Use isolated projects file for test
    let projects_dir = TempDir::new().unwrap();
    let projects_file = projects_dir.path().join("projects.json");
    std::env::set_var("KILD_PROJECTS_FILE", &projects_file);

    // Create a temp dir with a specific name
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .expect("git init failed");

    let result = super::add_project(path.to_path_buf(), None);

    // Clean up env var
    std::env::remove_var("KILD_PROJECTS_FILE");

    let project = result.expect("add_project should succeed");
    // Name should be the directory name (temp dir names are random)
    assert!(!project.name().is_empty());
    assert_ne!(project.name(), "unknown");
}
```

**Why**: Same reasoning as Step 2 - test isolation using temp file.

---

### Step 4: Add test for projects_file_path env var override

**File**: `crates/kild-ui/src/projects.rs`
**Lines**: After line 641 (end of tests module)
**Action**: UPDATE (add new test)

**Test to add:**
```rust
#[test]
fn test_projects_file_path_env_override() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let custom_path = temp_dir.path().join("custom_projects.json");

    // Set env var
    std::env::set_var("KILD_PROJECTS_FILE", &custom_path);

    // Verify override works
    let path = super::projects_file_path();
    assert_eq!(path, custom_path);

    // Clean up
    std::env::remove_var("KILD_PROJECTS_FILE");

    // Verify default works after cleanup
    let default_path = super::projects_file_path();
    assert!(default_path.ends_with("projects.json"));
    assert!(default_path.to_string_lossy().contains(".kild"));
}
```

**Why**: Verify the new env var override mechanism works correctly.

---

## Patterns to Follow

**From kild-core - env var override pattern:**

```rust
// SOURCE: crates/kild-core/src/config/defaults.rs:74
// Pattern for env var with default fallback
log_level: std::env::var("KILD_LOG_LEVEL").unwrap_or("info".to_string()),
```

**From kild-core - test isolation with temp directories:**

```rust
// SOURCE: crates/kild-core/src/config/loading.rs:256
// Pattern for test isolation using temp directories
let temp_dir = env::temp_dir().join("kild_config_test");
let user_config_dir = temp_dir.join("user");
// ... use isolated directories ...
let _ = fs::remove_dir_all(&temp_dir);  // cleanup
```

---

## Edge Cases & Risks

| Risk/Edge Case                     | Mitigation                                                              |
| ---------------------------------- | ----------------------------------------------------------------------- |
| Parallel tests modifying same file | Each test uses unique TempDir, no shared state                          |
| Env var not cleaned up after test  | Add explicit `remove_var()` call at end of each test                    |
| Invalid path in env var            | Existing error handling in save_projects will surface any write errors  |
| Non-test code uses env var         | Document env var is for testing only; unlikely to be set in production  |

---

## Validation

### Automated Checks

```bash
# Type check
cargo check -p kild-ui

# Run affected tests
cargo test -p kild-ui -- projects::tests::test_projects_file_path_env_override
cargo test -p kild-ui -- tests::test_add_project_uses_provided_name
cargo test -p kild-ui -- tests::test_add_project_derives_name_from_path

# Run all kild-ui tests to ensure no regressions
cargo test -p kild-ui

# Lint check
cargo clippy -p kild-ui -- -D warnings

# Format check
cargo fmt --check -p kild-ui
```

### Manual Verification

1. Before fix: Run `cargo test -p kild-ui` and check `~/.kild/projects.json` - should see temp paths
2. After fix: Run `cargo test -p kild-ui` and check `~/.kild/projects.json` - should NOT see new temp paths
3. Verify normal GUI usage still works (projects saved/loaded correctly)

---

## Scope Boundaries

**IN SCOPE:**

- Add `KILD_PROJECTS_FILE` env var override to `projects_file_path()`
- Update two tests in actions.rs to use isolated temp file
- Add test for env var override mechanism

**OUT OF SCOPE (do not touch):**

- Dependency injection pattern (over-engineering for this simple case)
- Thread-safe test isolation (env vars work fine for serial test execution)
- Other load/save functions that don't have tests calling them
- The `migrate_projects_to_canonical()` function (not called by tests)

---

## Metadata

- **Investigated by**: Claude
- **Timestamp**: 2026-01-28T12:00:00Z
- **Artifact**: `.claude/PRPs/issues/issue-106.md`
