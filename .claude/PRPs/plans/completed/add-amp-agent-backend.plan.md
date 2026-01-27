# Feature: Add AMP Agent Backend

## Summary

Add AMP (ampcode.com) as a supported AI coding agent in KILD. AMP is an interactive CLI tool similar to Claude Code that provides AI-assisted coding capabilities. The implementation follows the exact same pattern as existing agent backends (Claude, Kiro, Gemini, Codex).

## User Story

As a KILD user
I want to spawn AMP agents in isolated worktrees
So that I can use AMP for parallel AI development workflows

## Problem Statement

KILD currently supports Claude, Kiro, Gemini, and Codex agents. Users who prefer AMP (ampcode.com) cannot use it with KILD's worktree isolation system.

## Solution Statement

Implement AMP as a new agent backend following the established pattern:
1. Add `Amp` variant to `AgentType` enum
2. Create `AmpBackend` struct implementing `AgentBackend` trait
3. Register in the agent registry
4. Update CLI value parsers

## Metadata

| Field            | Value                                      |
| ---------------- | ------------------------------------------ |
| Type             | NEW_CAPABILITY                             |
| Complexity       | LOW                                        |
| Systems Affected | agents, CLI                                |
| Dependencies     | None (uses existing `which` crate)         |
| Estimated Tasks  | 8                                          |

---

## UX Design

### Before State

```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                              BEFORE STATE                                      ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║   $ kild create feature-x --agent amp                                         ║
║                     │                                                         ║
║                     ▼                                                         ║
║            ┌─────────────────┐                                                ║
║            │  ERROR: Invalid │                                                ║
║            │  agent 'amp'    │                                                ║
║            └─────────────────┘                                                ║
║                                                                               ║
║   SUPPORTED AGENTS: claude, kiro, gemini, codex                               ║
║   AMP NOT AVAILABLE                                                           ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

### After State

```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                               AFTER STATE                                      ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║   $ kild create feature-x --agent amp                                         ║
║                     │                                                         ║
║                     ▼                                                         ║
║   ┌─────────────┐         ┌─────────────┐         ┌─────────────┐            ║
║   │ Create      │ ──────► │ Spawn       │ ──────► │ AMP running │            ║
║   │ Worktree    │         │ Terminal    │         │ in worktree │            ║
║   └─────────────┘         └─────────────┘         └─────────────┘            ║
║                                                                               ║
║   SUPPORTED AGENTS: claude, kiro, gemini, codex, amp                          ║
║   AMP FULLY INTEGRATED                                                        ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

### Interaction Changes

| Location          | Before              | After                  | User Impact                |
| ----------------- | ------------------- | ---------------------- | -------------------------- |
| `kild create`     | amp not recognized  | amp creates kild       | Can use AMP with KILD      |
| `kild open`       | amp not recognized  | amp opens in kild      | Can reopen with AMP        |
| `kild restart`    | amp not recognized  | amp restarts kild      | Can restart with AMP       |
| `kild list`       | N/A                 | Shows "amp" as agent   | See AMP kilds in list      |

---

## Mandatory Reading

**CRITICAL: Implementation agent MUST read these files before starting any task:**

| Priority | File | Lines | Why Read This |
|----------|------|-------|---------------|
| P0 | `crates/kild-core/src/agents/backends/claude.rs` | 1-66 | Pattern to MIRROR exactly |
| P0 | `crates/kild-core/src/agents/traits.rs` | 1-33 | Trait to IMPLEMENT |
| P1 | `crates/kild-core/src/agents/types.rs` | 1-151 | Enum to UPDATE |
| P1 | `crates/kild-core/src/agents/registry.rs` | 1-238 | Registry to UPDATE |
| P1 | `crates/kild-core/src/agents/backends/mod.rs` | 1-12 | Module to UPDATE |
| P2 | `crates/kild/src/app.rs` | 28-32, 113-118, 214-219 | CLI to UPDATE |

---

## Patterns to Mirror

**BACKEND_STRUCT_PATTERN:**
```rust
// SOURCE: crates/kild-core/src/agents/backends/claude.rs:1-28
// COPY THIS PATTERN:
//! Claude Code agent backend implementation.

use crate::agents::traits::AgentBackend;

/// Backend implementation for Claude Code.
pub struct ClaudeBackend;

impl AgentBackend for ClaudeBackend {
    fn name(&self) -> &'static str {
        "claude"
    }

    fn display_name(&self) -> &'static str {
        "Claude Code"
    }

    fn is_available(&self) -> bool {
        which::which("claude").is_ok()
    }

    fn default_command(&self) -> &'static str {
        "claude"
    }

    fn process_patterns(&self) -> Vec<String> {
        vec!["claude".to_string(), "claude-code".to_string()]
    }
}
```

**BACKEND_TEST_PATTERN:**
```rust
// SOURCE: crates/kild-core/src/agents/backends/claude.rs:30-66
// COPY THIS PATTERN:
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_backend_name() {
        let backend = ClaudeBackend;
        assert_eq!(backend.name(), "claude");
    }

    #[test]
    fn test_claude_backend_display_name() {
        let backend = ClaudeBackend;
        assert_eq!(backend.display_name(), "Claude Code");
    }

    #[test]
    fn test_claude_backend_default_command() {
        let backend = ClaudeBackend;
        assert_eq!(backend.default_command(), "claude");
    }

    #[test]
    fn test_claude_backend_process_patterns() {
        let backend = ClaudeBackend;
        let patterns = backend.process_patterns();
        assert!(patterns.contains(&"claude".to_string()));
        assert!(patterns.contains(&"claude-code".to_string()));
    }

    #[test]
    fn test_claude_backend_command_patterns() {
        let backend = ClaudeBackend;
        let patterns = backend.command_patterns();
        assert_eq!(patterns, vec!["claude".to_string()]);
    }
}
```

**AGENT_TYPE_ENUM_PATTERN:**
```rust
// SOURCE: crates/kild-core/src/agents/types.rs:11-16
// Current enum (add Amp variant):
pub enum AgentType {
    Claude,
    Kiro,
    Gemini,
    Codex,
    // ADD: Amp,
}
```

**MODULE_EXPORT_PATTERN:**
```rust
// SOURCE: crates/kild-core/src/agents/backends/mod.rs:1-12
// COPY THIS PATTERN:
mod claude;
mod codex;
mod gemini;
mod kiro;
// ADD: mod amp;

pub use claude::ClaudeBackend;
pub use codex::CodexBackend;
pub use gemini::GeminiBackend;
pub use kiro::KiroBackend;
// ADD: pub use amp::AmpBackend;
```

**REGISTRY_PATTERN:**
```rust
// SOURCE: crates/kild-core/src/agents/registry.rs:22-29
// COPY THIS PATTERN:
fn new() -> Self {
    let mut backends: HashMap<AgentType, Box<dyn AgentBackend>> = HashMap::new();
    backends.insert(AgentType::Claude, Box::new(ClaudeBackend));
    backends.insert(AgentType::Kiro, Box::new(KiroBackend));
    backends.insert(AgentType::Gemini, Box::new(GeminiBackend));
    backends.insert(AgentType::Codex, Box::new(CodexBackend));
    // ADD: backends.insert(AgentType::Amp, Box::new(AmpBackend));
    Self { backends }
}
```

---

## AMP-Specific Details

Based on Amp CLI documentation:

| Property | Value | Rationale |
|----------|-------|-----------|
| Canonical name | `"amp"` | Binary name is `amp` |
| Display name | `"Amp"` | Official product name |
| CLI binary | `amp` | Verified via `amp --version` |
| Default command | `"amp"` | Interactive mode (no args needed) |
| Process patterns | `["amp"]` | Simple binary name |

---

## Files to Change

| File | Action | Justification |
| ---- | ------ | ------------- |
| `crates/kild-core/src/agents/backends/amp.rs` | CREATE | New backend implementation |
| `crates/kild-core/src/agents/backends/mod.rs` | UPDATE | Export new module |
| `crates/kild-core/src/agents/types.rs` | UPDATE | Add Amp variant to enum |
| `crates/kild-core/src/agents/registry.rs` | UPDATE | Register AmpBackend |
| `crates/kild/src/app.rs` | UPDATE | Add "amp" to value_parser (3 places) |
| `crates/kild-core/src/config/validation.rs` | UPDATE | Add "amp" to test |

---

## NOT Building (Scope Limits)

Explicit exclusions to prevent scope creep:

- **No AMP-specific configuration options** - AMP uses same config pattern as other agents
- **No AMP authentication handling** - AMP handles its own sign-in interactively
- **No AMP-specific flags** - Users can add flags via existing `--flags` mechanism
- **No documentation updates** - CLAUDE.md already lists agents generically

---

## Step-by-Step Tasks

Execute in order. Each task is atomic and independently verifiable.

### Task 1: CREATE `crates/kild-core/src/agents/backends/amp.rs`

- **ACTION**: CREATE new backend implementation file
- **IMPLEMENT**:
  - Module doc comment: `//! Amp agent backend implementation.`
  - Import: `use crate::agents::traits::AgentBackend;`
  - Struct: `pub struct AmpBackend;`
  - Implement `AgentBackend` trait with:
    - `name()` returns `"amp"`
    - `display_name()` returns `"Amp"`
    - `is_available()` uses `which::which("amp").is_ok()`
    - `default_command()` returns `"amp"`
    - `process_patterns()` returns `vec!["amp".to_string()]`
  - Add test module with 5 tests (name, display_name, default_command, process_patterns, command_patterns)
- **MIRROR**: `crates/kild-core/src/agents/backends/claude.rs:1-66`
- **VALIDATE**: `cargo build -p kild-core`

### Task 2: UPDATE `crates/kild-core/src/agents/backends/mod.rs`

- **ACTION**: ADD module declaration and export
- **IMPLEMENT**:
  - Add `mod amp;` after other mod declarations
  - Add `pub use amp::AmpBackend;` after other pub use statements
- **MIRROR**: Existing pattern in file
- **VALIDATE**: `cargo build -p kild-core`

### Task 3: UPDATE `crates/kild-core/src/agents/types.rs` - Enum variant

- **ACTION**: ADD `Amp` variant to `AgentType` enum
- **IMPLEMENT**:
  - Add `Amp,` to enum (line ~16)
- **MIRROR**: `crates/kild-core/src/agents/types.rs:11-16`
- **VALIDATE**: `cargo build -p kild-core` (will fail until all match arms updated)

### Task 4: UPDATE `crates/kild-core/src/agents/types.rs` - Methods

- **ACTION**: UPDATE all match expressions for new variant
- **IMPLEMENT**:
  - `as_str()`: Add `AgentType::Amp => "amp",`
  - `parse()`: Add `"amp" => Some(AgentType::Amp),`
  - `all()`: Add `AgentType::Amp,` to array
- **MIRROR**: Existing patterns in each method
- **VALIDATE**: `cargo build -p kild-core`

### Task 5: UPDATE `crates/kild-core/src/agents/types.rs` - Tests

- **ACTION**: UPDATE test assertions for new agent count
- **IMPLEMENT**:
  - `test_agent_type_as_str()`: Add `assert_eq!(AgentType::Amp.as_str(), "amp");`
  - `test_agent_type_all()`: Change `assert_eq!(all.len(), 4);` to `5`, add `assert!(all.contains(&AgentType::Amp));`
- **MIRROR**: Existing test patterns
- **VALIDATE**: `cargo test -p kild-core -- agents::types`

### Task 6: UPDATE `crates/kild-core/src/agents/registry.rs`

- **ACTION**: IMPORT and REGISTER AmpBackend
- **IMPLEMENT**:
  - Update import: Add `AmpBackend` to use statement (line ~6)
  - Add registration: `backends.insert(AgentType::Amp, Box::new(AmpBackend));` (line ~28)
- **MIRROR**: Existing pattern in `AgentRegistry::new()`
- **VALIDATE**: `cargo build -p kild-core`

### Task 7: UPDATE `crates/kild-core/src/agents/registry.rs` - Tests

- **ACTION**: UPDATE test assertions for new agent
- **IMPLEMENT**:
  - `test_is_valid_agent()`: Add `assert!(is_valid_agent("amp"));`
  - `test_valid_agent_names()`: Change `assert_eq!(names.len(), 4);` to `5`, add `assert!(names.contains(&"amp"));`
  - `test_get_default_command()`: Add `assert_eq!(get_default_command("amp"), Some("amp"));`
  - `test_registry_contains_all_agents()`: Add `"amp"` to expected_agents array
- **MIRROR**: Existing test patterns
- **VALIDATE**: `cargo test -p kild-core -- agents::registry`

### Task 8: UPDATE `crates/kild/src/app.rs`

- **ACTION**: ADD "amp" to all three value_parser arrays
- **IMPLEMENT**:
  - Line ~32 (create command): Change `["claude", "kiro", "gemini", "codex"]` to `["claude", "kiro", "gemini", "codex", "amp"]`
  - Line ~117 (open command): Same change
  - Line ~218 (restart command): Same change
- **MIRROR**: Existing value_parser pattern
- **VALIDATE**: `cargo build -p kild`

### Task 9: UPDATE `crates/kild-core/src/config/validation.rs` - Test

- **ACTION**: ADD "amp" to test array
- **IMPLEMENT**:
  - `test_config_validation_all_valid_agents()`: Change `["claude", "kiro", "gemini", "codex"]` to `["claude", "kiro", "gemini", "codex", "amp"]`
- **MIRROR**: Existing test pattern
- **VALIDATE**: `cargo test -p kild-core -- config::validation`

---

## Testing Strategy

### Unit Tests to Write

| Test File | Test Cases | Validates |
| --------- | ---------- | --------- |
| `crates/kild-core/src/agents/backends/amp.rs` | name, display_name, default_command, process_patterns, command_patterns | Backend implementation |

### Edge Cases Checklist

- [x] AMP binary not installed → `is_available()` returns false (handled by `which` crate)
- [x] Case-insensitive parsing → `AgentType::parse("AMP")` works (handled by `to_lowercase()`)
- [x] Serde serialization → `"amp"` via `#[serde(rename_all = "lowercase")]`

---

## Validation Commands

### Level 1: STATIC_ANALYSIS

```bash
cargo fmt --check && cargo clippy --all -- -D warnings
```

**EXPECT**: Exit 0, no errors or warnings

### Level 2: UNIT_TESTS

```bash
cargo test -p kild-core -- agents
```

**EXPECT**: All agent tests pass

### Level 3: FULL_SUITE

```bash
cargo test --all && cargo build --all
```

**EXPECT**: All tests pass, build succeeds

### Level 4: MANUAL_VALIDATION

```bash
# Verify amp is in help output
cargo run -p kild -- create --help | grep -q "amp"

# Verify amp is recognized (will fail if amp not installed, but shouldn't error on "invalid agent")
cargo run -p kild -- create test-amp --agent amp 2>&1 | grep -v "invalid"
```

---

## Acceptance Criteria

- [x] `AmpBackend` struct implements `AgentBackend` trait
- [x] `AgentType::Amp` variant exists and parses correctly
- [x] `AmpBackend` registered in agent registry
- [x] CLI accepts "amp" for `--agent` flag on create, open, restart commands
- [x] All existing tests pass
- [x] New backend has 5 unit tests mirroring Claude pattern
- [x] `cargo clippy --all -- -D warnings` passes
- [x] `cargo fmt --check` passes

---

## Completion Checklist

- [ ] Task 1: amp.rs created with full implementation and tests
- [ ] Task 2: backends/mod.rs exports AmpBackend
- [ ] Task 3-5: types.rs updated with Amp variant and tests
- [ ] Task 6-7: registry.rs imports, registers, and tests AmpBackend
- [ ] Task 8: app.rs value_parsers include "amp"
- [ ] Task 9: validation.rs test includes "amp"
- [ ] Level 1: `cargo fmt --check && cargo clippy --all -- -D warnings` passes
- [ ] Level 2: `cargo test -p kild-core -- agents` passes
- [ ] Level 3: `cargo test --all && cargo build --all` passes

---

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| AMP CLI not installed on user system | HIGH | LOW | `is_available()` returns false gracefully |
| AMP changes CLI binary name | LOW | MEDIUM | Process patterns can be updated |

---

## Notes

- AMP uses interactive sign-in, no API key needed
- Default command is just `amp` (enters interactive mode)
- AMP has execute mode (`amp -x "prompt"`) but KILD uses interactive mode
- Pattern is identical to Claude backend - simplest possible implementation
