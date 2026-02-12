# Investigation: Health dashboard truncates column values

**Issue**: #398 (https://github.com/Wirasm/kild/issues/398)
**Type**: BUG
**Investigated**: 2026-02-12T10:00:00Z

### Assessment

| Metric     | Value  | Reasoning                                                                                               |
| ---------- | ------ | ------------------------------------------------------------------------------------------------------- |
| Severity   | MEDIUM | Branch names and timestamps are visually truncated, reducing readability but not losing data or crashing |
| Complexity | LOW    | Single file change (`health.rs`), pattern already established in `table.rs` and `status.rs`             |
| Confidence | HIGH   | Exact truncation mechanism identified at `health.rs:9-18`, fix pattern proven in PR #387                |

---

## Problem Statement

The `kild health` command uses hardcoded column widths and a `truncate()` function that clips branch names and timestamps with "...". PR #387 (commit 2e92b09) fixed this same problem in `list` and `status` commands by switching to dynamic-width tables with padding, but the health dashboard was not included in that fix.

---

## Analysis

### Root Cause

WHY: Branch names and timestamps show "..." in health dashboard
↓ BECAUSE: `print_health_table()` calls `truncate(&kild.branch, 16)` which clips to 16 chars
Evidence: `crates/kild/src/commands/health.rs:185` - `truncate(&kild.branch, 16)`

↓ BECAUSE: The `truncate()` function enforces a fixed max width and appends "..."
Evidence: `crates/kild/src/commands/health.rs:9-18` - hardcoded truncation logic

↓ ROOT CAUSE: The health dashboard still uses the old fixed-width approach while `list` and `status` were migrated to dynamic-width tables in PR #387
Evidence: `crates/kild/src/commands/health.rs:136-207` uses `truncate()` with fixed widths; `crates/kild/src/table.rs:265-282` uses `display_width()` + `pad()` with dynamic widths

### Affected Files

| File                                  | Lines   | Action | Description                                                          |
| ------------------------------------- | ------- | ------ | -------------------------------------------------------------------- |
| `crates/kild/src/commands/health.rs`  | 1-256   | UPDATE | Replace truncation with dynamic-width table (table view + single view) |

### Integration Points

- `crates/kild/src/commands/mod.rs:61` routes "health" subcommand to `handle_health_command()`
- `crates/kild-core/src/health/handler.rs:9` provides `get_health_all_sessions()` returning `HealthOutput`
- `crates/kild-core/src/health/types.rs:23-44` defines `KildHealth` and `HealthOutput` data types
- No callers of `truncate()` outside `health.rs` — safe to remove entirely

### Git History

- **Introduced**: `1027b21` - refactor: split commands.rs into per-command module directory
- **Truncate moved here**: `2dbe56d` - refactor: move truncate() from table.rs to health.rs
- **Implication**: When PR #387 removed truncation from `table.rs`, it was moved to `health.rs` as the last remaining consumer — it should have been removed entirely

---

## Implementation Plan

### Step 1: Remove `truncate()` function and add `display_width()` + `pad()` helpers

**File**: `crates/kild/src/commands/health.rs`
**Lines**: 1-18
**Action**: UPDATE

**Current code:**
```rust
use clap::ArgMatches;
use tracing::{error, info, warn};

use kild_core::events;
use kild_core::health;

use super::helpers::{is_valid_branch_name, load_config_with_warning};

/// Truncate a string to a maximum display width, adding "..." if truncated.
fn truncate(s: &str, max_len: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_len {
        format!("{:<width$}", s, width = max_len)
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(3)).collect();
        format!("{:<width$}", format!("{}...", truncated), width = max_len)
    }
}
```

**Required change:**
```rust
use clap::ArgMatches;
use tracing::{error, info, warn};
use unicode_width::UnicodeWidthStr;

use kild_core::events;
use kild_core::health;

use super::helpers::{is_valid_branch_name, load_config_with_warning};

/// Compute the terminal display width of a string.
fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

/// Pad a string to a minimum display width without truncating.
fn pad(s: &str, min_width: usize) -> String {
    let width = display_width(s);
    if width >= min_width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(min_width - width))
    }
}
```

**Why**: Replace truncation with the same `display_width()` + `pad()` pattern used in `table.rs:265-282`.

---

### Step 2: Rewrite `print_health_table()` with dynamic column widths

**File**: `crates/kild/src/commands/health.rs`
**Lines**: 136-207
**Action**: UPDATE

**Current code:** Hardcoded column widths with `truncate()` calls.

**Required change:** Two-pass approach:
1. First pass: iterate `output.kilds` to compute max width per column (branch, agent, activity, cpu, memory, status, last_activity), starting from header label widths as minimums.
2. Second pass: render table rows using `pad()` with computed widths, and dynamic border generation with `"─".repeat(width + 2)`.

**Pattern to follow:** Same approach as `TableFormatter::new()` in `table.rs:26-100`.

**Columns and their minimum widths (header labels):**
- St: 2 (fixed — status icons are all ~2 display width)
- Branch: 6 (`"Branch".len()`)
- Agent: 5 (`"Agent".len()`)
- Activity: 8 (`"Activity".len()`)
- CPU %: 5 (`"CPU %".len()`)
- Memory: 6 (`"Memory".len()`)
- Status: 7 (`"Status".len()` — "Working" is 7 chars)
- Last Activity: 13 (`"Last Activity".len()`)

**Width computation for each kild:**
- `branch_w = branch_w.max(display_width(&kild.branch))`
- `agent_w = agent_w.max(display_width(&kild.agent))`
- `activity_w = activity_w.max(display_width(&agent_activity_string))`
- `cpu_w = cpu_w.max(display_width(&cpu_string))`
- `mem_w = mem_w.max(display_width(&mem_string))`
- `status_w = status_w.max(display_width(&status_string))`
- `last_activity_w = last_activity_w.max(display_width(&last_activity_string))`

**Row rendering:** Replace `truncate()` calls with `pad()` calls using computed widths.

**Border rendering:** Use `"─".repeat(width + 2)` for each column (matching `table.rs` pattern).

---

### Step 3: Rewrite `print_single_kild_health()` with dynamic box width

**File**: `crates/kild/src/commands/health.rs`
**Lines**: 209-255
**Action**: UPDATE

**Current code:** Fixed box width of 47 chars for values, uses `truncate()` for worktree_path and last_activity.

**Required change:** Follow the `status.rs:87-224` pattern:
1. Collect all label-value pairs into a `Vec<(&str, String)>`.
2. Compute max value width using `display_width()`.
3. Use a fixed label width (13 — "Last Active:" is 12 + space).
4. Generate dynamic border with `"─".repeat(label_width + value_width + 2)`.
5. Render rows with computed widths — no truncation.

**Why**: Same dynamic approach as status command, ensures worktree paths and timestamps display in full.

---

## Patterns to Follow

**From codebase — mirror these exactly:**

```rust
// SOURCE: crates/kild/src/table.rs:265-282
// Pattern for display width and padding
fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

fn pad(s: &str, min_width: usize) -> String {
    let width = display_width(s);
    if width >= min_width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(min_width - width))
    }
}
```

```rust
// SOURCE: crates/kild/src/table.rs:200-262
// Pattern for dynamic border generation
fn top_border(&self) -> String {
    format!(
        "┌{}┬{}┬...┐",
        "─".repeat(self.branch_width + 2),
        "─".repeat(self.agent_width + 2),
    )
}
```

```rust
// SOURCE: crates/kild/src/commands/status.rs:184-198
// Pattern for dynamic box width in single-item view
let value_width = rows
    .iter()
    .map(|(_, v)| UnicodeWidthStr::width(v.as_str()))
    .max()
    .unwrap_or(0);

let inner_width = label_width + value_width;
let border = "─".repeat(inner_width + 2);
```

---

## Edge Cases & Risks

| Risk/Edge Case                         | Mitigation                                                         |
| -------------------------------------- | ------------------------------------------------------------------ |
| Empty kilds list                       | Already handled at line 137-139 (early return)                     |
| Very long branch names                 | Table columns grow — acceptable since terminal wraps naturally      |
| Unicode/emoji in branch names          | `display_width()` uses `UnicodeWidthStr` for correct width calc    |
| Status icons have variable width       | St column stays fixed at 2 — icons already rendered at ~2 width    |
| N/A values for CPU/Memory              | Still included in width computation as "N/A" (3 chars)             |

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

1. Run `kild health` with kilds that have long branch names — verify no truncation
2. Run `kild health <branch>` for single kild — verify box adapts to content width
3. Run `kild health --json` — verify JSON output unchanged (not affected by this change)

---

## Scope Boundaries

**IN SCOPE:**
- `crates/kild/src/commands/health.rs` — table and single-kild display rendering

**OUT OF SCOPE (do not touch):**
- `crates/kild-core/src/health/` — data layer is fine, no changes needed
- `crates/kild/src/table.rs` — already uses dynamic widths
- `crates/kild/src/commands/stats.rs` — has its own truncation issue but separate scope
- JSON output path — unaffected by display changes

---

## Metadata

- **Investigated by**: Claude
- **Timestamp**: 2026-02-12T10:00:00Z
- **Artifact**: `.claude/PRPs/issues/issue-398.md`
