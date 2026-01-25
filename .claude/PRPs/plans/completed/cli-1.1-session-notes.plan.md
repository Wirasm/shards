# Implementation Plan: CLI Phase 1.1 - Session Notes (`--note`)

**Source PRD**: `.claude/PRPs/prds/cli-core-features.prd.md`
**Phase**: 1.1
**Status**: READY FOR IMPLEMENTATION

---

## Summary

Add optional `note` field to Session struct enabling users to document what each shard is for. The note is set via `--note` flag during `shards create`, shown truncated in `shards list` table, and displayed in full in `shards status` output. JSON serialization is handled automatically by serde.

## User Story

As a user managing multiple shards, I want to add a description when creating a shard so that I can remember what each shard is for when viewing the list later.

## Problem Statement

When managing multiple shards, users lose track of what each shard is for. The branch name alone is often insufficient context (e.g., "feature-auth" could be implementing auth in many ways). Users need a way to add and view quick notes.

## Solution Statement

Add an optional `note: Option<String>` field to the Session struct, a `--note` CLI flag to the create command, a "Note" column in the list table (truncated to 30 chars), and full note display in the status output. Use serde defaults for backward compatibility with existing session files.

## Metadata

| Field | Value |
|-------|-------|
| Type | ENHANCEMENT |
| Complexity | LOW |
| Systems Affected | shards-core (types), shards (CLI) |
| Dependencies | None |
| Estimated Tasks | 10 |

---

## Mandatory Reading

**CRITICAL: Implementation agent MUST read these files before starting any task:**

| Priority | File | Lines | Why Read This |
|----------|------|-------|---------------|
| P0 | `crates/shards-core/src/sessions/types.rs` | 1-120 | Session struct definition, serde defaults pattern |
| P0 | `crates/shards-core/src/sessions/handler.rs` | 43-183 | create_session() - where Session is constructed |
| P1 | `crates/shards/src/app.rs` | 10-44 | create command clap definition |
| P1 | `crates/shards/src/commands.rs` | 72-135 | handle_create_command() - CLI handler pattern |
| P1 | `crates/shards/src/table.rs` | 1-155 | TableFormatter - list table rendering |
| P2 | `crates/shards/src/commands.rs` | 296-365 | handle_status_command() - status output pattern |

---

## Patterns to Mirror

**SERDE_DEFAULT_PATTERN:**
```rust
// SOURCE: crates/shards-core/src/sessions/types.rs:17-19
fn default_last_activity() -> Option<String> {
    None
}

// And field usage at line 86:
#[serde(default = "default_last_activity")]
pub last_activity: Option<String>,
```

**CLI_FLAG_PATTERN:**
```rust
// SOURCE: crates/shards/src/app.rs:33-36
.arg(
    Arg::new("startup-command")
        .long("startup-command")
        .help("Agent startup command (overrides config)")
)
```

**TABLE_COLUMN_PATTERN:**
```rust
// SOURCE: crates/shards/src/table.rs:24-29
Self {
    branch_width,
    agent_width: 7,
    status_width: 7,
    // ... other widths
    command_width: 20,
}
```

**STATUS_OUTPUT_PATTERN:**
```rust
// SOURCE: crates/shards/src/commands.rs:306-314
println!("Shard Status: {}", branch);
println!("+---------------------------------------------------------+");
println!("| Branch:      {:<47} |", session.branch);
println!("| Agent:       {:<47} |", session.agent);
```

---

## Files to Change

| File | Action | Justification |
|------|--------|---------------|
| `crates/shards-core/src/sessions/types.rs` | UPDATE | Add `note: Option<String>` field to Session struct |
| `crates/shards-core/src/sessions/types.rs` | UPDATE | Add `note: Option<String>` field to CreateSessionRequest struct |
| `crates/shards-core/src/sessions/handler.rs` | UPDATE | Pass note from request to Session construction |
| `crates/shards/src/app.rs` | UPDATE | Add `--note` arg to create command |
| `crates/shards/src/commands.rs` | UPDATE | Extract note from args, pass to CreateSessionRequest |
| `crates/shards/src/table.rs` | UPDATE | Add Note column with truncation |
| `crates/shards/src/commands.rs` | UPDATE | Show full note in status output |

---

## NOT Building (Scope Limits)

Explicit exclusions to prevent scope creep:

- **No `--note` on other commands** - Only create, per PRD
- **No note editing** - YAGNI, can re-create shard if needed
- **No note search/filter** - YAGNI
- **No note validation** - Allow any string, trust the user
- **No note length limit** - Truncation in display handles long notes

---

## Step-by-Step Tasks

Execute in order. Each task is atomic and independently verifiable.

### Task 1: ADD serde default function for note

- **ACTION**: Add default function for the note field (for backward compatibility)
- **FILE**: `crates/shards-core/src/sessions/types.rs`
- **LOCATION**: After line 19 (after `default_last_activity` function)
- **IMPLEMENT**:
```rust
fn default_note() -> Option<String> {
    None
}
```
- **MIRROR**: Lines 17-19 (`default_last_activity` pattern)
- **VALIDATE**: `cargo check -p shards-core`

### Task 2: ADD note field to Session struct

- **ACTION**: Add `note` field to Session struct
- **FILE**: `crates/shards-core/src/sessions/types.rs`
- **LOCATION**: After `last_activity` field (line 86-87), before the closing brace
- **IMPLEMENT**:
```rust
    /// Optional description of what this shard is for.
    ///
    /// Set via `--note` flag during `shards create`. Shown truncated in list,
    /// full text in status output.
    #[serde(default = "default_note")]
    pub note: Option<String>,
```
- **MIRROR**: Lines 83-86 (`last_activity` field pattern)
- **GOTCHA**: Must use `#[serde(default = "default_note")]` for backward compatibility with existing session files
- **VALIDATE**: `cargo check -p shards-core`

### Task 3: UPDATE CreateSessionRequest to include note

- **ACTION**: Add `note` field to CreateSessionRequest struct
- **FILE**: `crates/shards-core/src/sessions/types.rs`
- **LOCATION**: In `CreateSessionRequest` struct (lines 104-108)
- **IMPLEMENT** (update struct and `new` function):
```rust
#[derive(Debug, Clone)]
pub struct CreateSessionRequest {
    pub branch: String,
    pub agent: Option<String>,
    pub note: Option<String>,
}

impl CreateSessionRequest {
    pub fn new(branch: String, agent: Option<String>, note: Option<String>) -> Self {
        Self { branch, agent, note }
    }
    // ... rest unchanged
}
```
- **VALIDATE**: `cargo check -p shards-core` (will fail until callers updated - that's expected)

### Task 4: UPDATE create_session() to use note field

- **ACTION**: Pass note from request to Session construction
- **FILE**: `crates/shards-core/src/sessions/handler.rs`
- **LOCATION**: In `create_session()` function, Session struct construction (lines 146-168)
- **IMPLEMENT** (add `note` field to Session initialization):
```rust
    let session = Session {
        id: session_id.clone(),
        project_id: project.id,
        branch: validated.name.clone(),
        worktree_path: worktree.path,
        agent: validated.agent.clone(),
        status: SessionStatus::Active,
        created_at: now.clone(),
        last_activity: Some(now),
        port_range_start: port_start,
        port_range_end: port_end,
        port_count: config.default_port_count,
        process_id: spawn_result.process_id,
        process_name: spawn_result.process_name.clone(),
        process_start_time: spawn_result.process_start_time,
        terminal_type: Some(spawn_result.terminal_type.clone()),
        terminal_window_id: spawn_result.terminal_window_id.clone(),
        command: if spawn_result.command_executed.trim().is_empty() {
            format!("{} (command not captured)", validated.agent)
        } else {
            spawn_result.command_executed.clone()
        },
        note: request.note.clone(),  // ADD THIS LINE
    };
```
- **VALIDATE**: `cargo check -p shards-core`

### Task 5: UPDATE all tests with note field

- **ACTION**: Add `note: None` to all Session constructions in tests
- **FILE**: `crates/shards-core/src/sessions/types.rs` (tests at lines 124-282)
- **FILE**: `crates/shards-core/src/sessions/handler.rs` (tests at lines 738-1287)
- **FILE**: `crates/shards-core/src/sessions/persistence.rs` (tests at lines 169-633)
- **IMPLEMENT**: Add `note: None,` to every `Session { ... }` construction in test code
- **EXAMPLE** (types.rs test at line 129):
```rust
let session = Session {
    id: "test/branch".to_string(),
    // ... other fields ...
    last_activity: Some("2024-01-01T00:00:00Z".to_string()),
    note: None,  // ADD THIS LINE
};
```
- **GOTCHA**: There are approximately 10+ Session constructions across test files
- **VALIDATE**: `cargo test -p shards-core`

### Task 6: ADD --note CLI arg to create command

- **ACTION**: Add `--note` argument to create subcommand
- **FILE**: `crates/shards/src/app.rs`
- **LOCATION**: In create subcommand definition (after line 43, after `--flags`)
- **IMPLEMENT**:
```rust
.arg(
    Arg::new("note")
        .long("note")
        .short('n')
        .help("Description of what this shard is for")
)
```
- **MIRROR**: Lines 33-36 (`--startup-command` pattern)
- **VALIDATE**: `cargo check -p shards && cargo run -- create --help`

### Task 7: UPDATE handle_create_command to pass note

- **ACTION**: Extract note from args and pass to CreateSessionRequest
- **FILE**: `crates/shards/src/commands.rs`
- **LOCATION**: In `handle_create_command()` function (lines 72-135)
- **IMPLEMENT** (add note extraction and update CreateSessionRequest::new call):
```rust
fn handle_create_command(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let branch = matches
        .get_one::<String>("branch")
        .ok_or("Branch argument is required")?;
    let note = matches.get_one::<String>("note").cloned();  // ADD THIS LINE

    let mut config = load_config_with_warning();

    // ... existing config override code ...

    info!(
        event = "cli.create_started",
        branch = branch,
        agent = config.agent.default,
        note = ?note  // ADD THIS to logging
    );

    let request = CreateSessionRequest::new(branch.clone(), agent_override, note);  // UPDATE THIS LINE

    // ... rest unchanged
}
```
- **GOTCHA**: Update the `info!` log macro to include note field for observability
- **VALIDATE**: `cargo check -p shards`

### Task 8: ADD Note column to TableFormatter

- **ACTION**: Add note_width field and Note column to table
- **FILE**: `crates/shards/src/table.rs`
- **LOCATION**: Throughout the file
- **IMPLEMENT**:

1. Add field to struct (after line 10):
```rust
pub struct TableFormatter {
    branch_width: usize,
    agent_width: usize,
    status_width: usize,
    created_width: usize,
    port_width: usize,
    process_width: usize,
    command_width: usize,
    note_width: usize,  // ADD THIS
}
```

2. Initialize in `new()` (after line 29):
```rust
Self {
    branch_width,
    agent_width: 7,
    status_width: 7,
    created_width: 19,
    port_width: 11,
    process_width: 11,
    command_width: 20,
    note_width: 30,  // ADD THIS
}
```

3. Update `print_row()` to include note (after line 77):
```rust
let note_display = session.note.as_deref().unwrap_or("");

println!(
    "| {:<width_branch$} | {:<width_agent$} | {:<width_status$} | {:<width_created$} | {:<width_port$} | {:<width_process$} | {:<width_command$} | {:<width_note$} |",
    truncate(&session.branch, self.branch_width),
    truncate(&session.agent, self.agent_width),
    format!("{:?}", session.status).to_lowercase(),
    truncate(&session.created_at, self.created_width),
    truncate(&port_range, self.port_width),
    truncate(&process_status, self.process_width),
    truncate(&session.command, self.command_width),
    truncate(note_display, self.note_width),  // ADD THIS
    width_branch = self.branch_width,
    width_agent = self.agent_width,
    width_status = self.status_width,
    width_created = self.created_width,
    width_port = self.port_width,
    width_process = self.process_width,
    width_command = self.command_width,
    width_note = self.note_width,  // ADD THIS
);
```

4. Update all border methods (`top_border`, `header_row`, `separator`, `bottom_border`) to include the note column:
```rust
// In top_border() - add after command column:
"-".repeat(self.note_width + 2),

// In header_row() - add "Note" header:
"Note",
width_note = self.note_width,

// In separator() - add after command:
"-".repeat(self.note_width + 2),

// In bottom_border() - add after command:
"-".repeat(self.note_width + 2),
```

- **VALIDATE**: `cargo check -p shards && cargo run -- list`

### Task 9: UPDATE status command to show full note

- **ACTION**: Add note display to status output
- **FILE**: `crates/shards/src/commands.rs`
- **LOCATION**: In `handle_status_command()` function (lines 296-365)
- **IMPLEMENT** (add after the "Created" line, around line 314):
```rust
println!("| Created:     {:<47} |", session.created_at);
if let Some(ref note) = session.note {
    println!("| Note:        {:<47} |", note);
}
println!("| Worktree:    {:<47} |", session.worktree_path.display());
```
- **GOTCHA**: Note may be longer than 47 chars - consider wrapping or truncating. For simplicity, let it overflow (user can resize terminal).
- **VALIDATE**: `cargo check -p shards && cargo run -- status <existing-branch>`

### Task 10: ADD backward compatibility test

- **ACTION**: Add test that verifies old session files (without note) can be deserialized
- **FILE**: `crates/shards-core/src/sessions/types.rs`
- **LOCATION**: In the `#[cfg(test)] mod tests` section (after line 177)
- **IMPLEMENT**:
```rust
#[test]
fn test_session_backward_compatibility_note() {
    // Test that sessions without note field can be deserialized
    let json_without_note = r#"{
        "id": "test/branch",
        "project_id": "test",
        "branch": "branch",
        "worktree_path": "/tmp/test",
        "agent": "claude",
        "status": "Active",
        "created_at": "2024-01-01T00:00:00Z",
        "port_range_start": 3000,
        "port_range_end": 3009,
        "port_count": 10,
        "process_id": null,
        "process_name": null,
        "process_start_time": null,
        "command": "claude-code"
    }"#;

    let session: Session = serde_json::from_str(json_without_note).unwrap();
    assert_eq!(session.note, None);
    assert_eq!(session.branch, "branch");
}
```
- **MIRROR**: Lines 155-175 (`test_session_backward_compatibility` pattern)
- **VALIDATE**: `cargo test -p shards-core test_session_backward_compatibility_note`

---

## Validation Commands

### Level 1: STATIC_ANALYSIS

```bash
cargo fmt --check && cargo clippy --all -- -D warnings
```

**EXPECT**: Exit 0, no errors or warnings

### Level 2: TYPE_CHECK

```bash
cargo check --all
```

**EXPECT**: Exit 0, no type errors

### Level 3: BUILD

```bash
cargo build --all
```

**EXPECT**: Exit 0, clean build

### Level 4: TESTS

```bash
cargo test --all
```

**EXPECT**: All tests pass

### Level 5: MANUAL_VALIDATION

```bash
# Test create with note
shards create test-notes --note "Testing the notes feature for JWT auth"
# Output should show creation succeeded

# Test list shows note (truncated)
shards list
# Should show: test-notes | claude | active | ... | Testing the notes feat...

# Test status shows full note
shards status test-notes
# Should show: Note: Testing the notes feature for JWT auth

# Test create without note (backward compat)
shards create test-no-note
shards list
# Note column should be empty for test-no-note

# Test JSON output includes note
shards list --json | jq '.[].note'
# Should output the note string or null

# Cleanup
shards destroy --force test-notes
shards destroy --force test-no-note
```

---

## Acceptance Criteria

- [ ] `shards create <branch> --note "..."` accepts and stores the note
- [ ] `shards list` shows Note column truncated to ~30 chars
- [ ] `shards status <branch>` shows full note if present
- [ ] Session JSON includes `note` field (null if not set)
- [ ] Existing session files without note field load correctly
- [ ] All validation commands (fmt, clippy, check, test, build) pass

---

## Completion Checklist

- [ ] Task 1: Added serde default function for note
- [ ] Task 2: Added note field to Session struct
- [ ] Task 3: Updated CreateSessionRequest to include note
- [ ] Task 4: Updated create_session() to use note field
- [ ] Task 5: Updated all tests with note field
- [ ] Task 6: Added --note CLI arg to create command
- [ ] Task 7: Updated handle_create_command to pass note
- [ ] Task 8: Added Note column to TableFormatter
- [ ] Task 9: Updated status command to show full note
- [ ] Task 10: Added backward compatibility test
- [ ] Level 1-4 validation commands pass
- [ ] Manual testing completed
