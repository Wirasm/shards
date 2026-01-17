# Feature: GPUI Native Terminal UI

## Summary

Build a native desktop application using GPUI (Zed's UI framework) with embedded terminals powered by `alacritty_terminal`, enabling full control over shard sessions. This replaces the current fire-and-forget terminal spawning with an orchestrated multi-terminal interface where a main session can read output from, send prompts to, and coordinate work across multiple AI agent shards.

## User Story

As a developer managing multiple AI agents
I want a native UI with embedded terminals for each shard
So that I can orchestrate agents from a main session, read their output, and send prompts programmatically without losing context or control

## Problem Statement

The current CLI spawns external terminal windows via AppleScript (macOS only), immediately dropping all process handles. This makes it impossible to:
- Read agent output programmatically
- Send messages to running interactive sessions
- Detect task completion reliably
- Coordinate work across multiple shards
- Have a main agent orchestrate child shards

## Solution Statement

Create a GPUI-based native application that embeds terminals directly using `alacritty_terminal` (following Zed's architecture). Each shard becomes a managed PTY with full read/write access. A "main session" terminal can query shard outputs, send prompts, and coordinate work—transforming Shards from a launcher into an orchestration platform.

## Metadata

| Field            | Value                                                                    |
| ---------------- | ------------------------------------------------------------------------ |
| Type             | NEW_CAPABILITY                                                           |
| Complexity       | HIGH                                                                     |
| Systems Affected | terminal, sessions, process, core, cli + NEW: ui, pty                    |
| Dependencies     | gpui 0.1+, alacritty_terminal 0.24+, tokio 1.x                           |
| Estimated Tasks  | 18                                                                       |

---

## UX Design

### Before State
```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                              BEFORE STATE                                      ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║   ┌─────────────┐         ┌─────────────┐         ┌─────────────┐            ║
║   │   CLI       │ ──────► │  AppleScript│ ──────► │ Terminal.app│            ║
║   │  `shards    │         │  osascript  │         │  (external) │            ║
║   │   create`   │         │             │         │             │            ║
║   └─────────────┘         └─────────────┘         └─────────────┘            ║
║         │                                               │                     ║
║         │ Returns immediately                           │ Runs independently  ║
║         │ (fire-and-forget)                             │ (no connection)     ║
║         ▼                                               ▼                     ║
║   ┌─────────────┐                               ┌─────────────┐              ║
║   │ Session.json│                               │   Agent     │              ║
║   │  (PID only) │                               │  (claude)   │              ║
║   └─────────────┘                               └─────────────┘              ║
║                                                                               ║
║   USER_FLOW:                                                                  ║
║   1. Run `shards create mybranch --agent claude`                             ║
║   2. New Terminal.app window opens                                           ║
║   3. Claude starts in that window                                            ║
║   4. User manually switches to that window to interact                       ║
║   5. NO way to read output, send messages, or coordinate                     ║
║                                                                               ║
║   PAIN_POINTS:                                                                ║
║   - Cannot read agent output programmatically                                ║
║   - Cannot send prompts to running sessions                                  ║
║   - Cannot orchestrate multiple shards                                       ║
║   - macOS only (AppleScript)                                                 ║
║   - No completion detection                                                  ║
║                                                                               ║
║   DATA_FLOW:                                                                  ║
║   CLI ──► osascript ──► Terminal.app ──► [DISCONNECTED]                      ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

### After State
```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                               AFTER STATE                                      ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║   ┌─────────────────────────────────────────────────────────────────────────┐ ║
║   │                    Shards GPUI Application                              │ ║
║   │  ┌───────────────────────────────────────────────────────────────────┐  │ ║
║   │  │  MAIN SESSION (claude)                               [master]    │  │ ║
║   │  │  ─────────────────────────────────────────────────────────────── │  │ ║
║   │  │  > @shard:auth-fix status                                        │  │ ║
║   │  │  Main: The auth-fix shard completed. Reading output...           │  │ ║
║   │  │        ✓ Fixed login.rs authentication bug                       │  │ ║
║   │  │        ✓ Added 3 unit tests                                      │  │ ║
║   │  │  > @shard:auth-fix "Now add integration tests"                   │  │ ║
║   │  │  Main: Sent prompt. Monitoring shard...                          │  │ ║
║   │  └───────────────────────────────────────────────────────────────────┘  │ ║
║   │                              │                                          │ ║
║   │                              │ Direct PTY read/write                    │ ║
║   │                              ▼                                          │ ║
║   │  ┌─ SHARDS ─────────────────────────────────────────────────────────┐  │ ║
║   │  │ [auth-fix ●]  [feature-x ○]  [tests ●]  [+]                      │  │ ║
║   │  └──────────────────────────────────────────────────────────────────┘  │ ║
║   │  ┌───────────────────────────────────────────────────────────────────┐  │ ║
║   │  │  auth-fix (claude) ● working                                     │  │ ║
║   │  │  ─────────────────────────────────────────────────────────────── │  │ ║
║   │  │  Claude: Adding integration tests for authentication flow...     │  │ ║
║   │  │          ✓ test_login_success                                    │  │ ║
║   │  │          ○ test_login_rate_limiting (in progress)                │  │ ║
║   │  └───────────────────────────────────────────────────────────────────┘  │ ║
║   └─────────────────────────────────────────────────────────────────────────┘ ║
║                                                                               ║
║   USER_FLOW:                                                                  ║
║   1. Launch Shards app (or `shards ui`)                                      ║
║   2. Main session starts with orchestrating agent                            ║
║   3. Create shards via UI button or main agent command                       ║
║   4. Each shard is an embedded terminal with full PTY control                ║
║   5. Main agent can read ANY shard's output                                  ║
║   6. Main agent can send prompts to ANY shard                                ║
║   7. Tab between shards, or let main agent coordinate                        ║
║                                                                               ║
║   VALUE_ADD:                                                                  ║
║   - Full read/write control over all shard terminals                         ║
║   - Main session orchestrates child shards                                   ║
║   - Cross-platform (macOS + Linux)                                           ║
║   - Real-time output streaming and capture                                   ║
║   - Completion detection via PTY state                                       ║
║                                                                               ║
║   DATA_FLOW:                                                                  ║
║   ┌──────────────┐     ┌─────────────────────────────────────┐               ║
║   │ Main Session │◄───►│          ShardManager               │               ║
║   │    (PTY)     │     │  ┌───────┐ ┌───────┐ ┌───────┐     │               ║
║   └──────────────┘     │  │Shard 1│ │Shard 2│ │Shard N│     │               ║
║         │              │  │ (PTY) │ │ (PTY) │ │ (PTY) │     │               ║
║         │              │  └───┬───┘ └───┬───┘ └───┬───┘     │               ║
║         ▼              │      │         │         │         │               ║
║   read_shard()         │      ▼         ▼         ▼         │               ║
║   send_to_shard()      │  OutputBuffer (ring buffer)        │               ║
║                        └─────────────────────────────────────┘               ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

### Interaction Changes

| Location         | Before                        | After                                    | User Impact                              |
| ---------------- | ----------------------------- | ---------------------------------------- | ---------------------------------------- |
| Terminal spawn   | External window (AppleScript) | Embedded PTY in GPUI                     | All terminals in one app                 |
| Output reading   | Impossible                    | `shard.read_output(lines)`               | Main agent can see shard work            |
| Prompt sending   | Impossible                    | `shard.send_prompt(text)`                | Main agent can direct shards             |
| Session list     | CLI `shards list`             | Visual tab bar with status indicators    | Instant visual overview                  |
| Shard switching  | Alt-tab between windows       | Click tab or keyboard shortcut           | Seamless context switching               |
| Completion       | Unknown (check manually)      | PTY state detection + idle timeout       | Automatic orchestration possible         |
| Platform support | macOS only                    | macOS + Linux                            | Cross-platform development               |

---

## Mandatory Reading

**CRITICAL: Implementation agent MUST read these files before starting any task:**

| Priority | File                                | Lines   | Why Read This                                      |
| -------- | ----------------------------------- | ------- | -------------------------------------------------- |
| P0       | `src/terminal/handler.rs`           | 8-88    | Current spawn pattern to REPLACE                   |
| P0       | `src/terminal/types.rs`             | 4-24    | SpawnConfig/SpawnResult to EXTEND                  |
| P0       | `src/sessions/handler.rs`           | 8-109   | Session creation flow to INTEGRATE with            |
| P0       | `src/sessions/types.rs`             | 9-40    | Session struct to ADD PTY fields                   |
| P1       | `src/core/errors.rs`                | 3-12    | ShardsError trait to IMPLEMENT                     |
| P1       | `src/core/config.rs`                | 54-122  | Config hierarchy to ADD UI settings                |
| P1       | `src/process/operations.rs`         | 14-59   | Process tracking to REPLACE with PTY handles       |
| P2       | `src/cli/app.rs`                    | 3-71    | CLI structure for adding `ui` subcommand           |
| P2       | `src/lib.rs`                        | 1-10    | Module exports to ADD new modules                  |

**External Documentation:**

| Source                                                                                   | Section            | Why Needed                                 |
| ---------------------------------------------------------------------------------------- | ------------------ | ------------------------------------------ |
| [GPUI Docs](https://docs.rs/gpui)                                                        | Views & Rendering  | Core UI framework patterns                 |
| [GPUI Book - Getting Started](https://matinaniss.github.io/gpui-book/getting-started/)   | App structure      | Window, View, Entity setup                 |
| [alacritty_terminal Term](https://docs.rs/alacritty_terminal/latest/alacritty_terminal/) | Term, Pty          | Terminal emulation API                     |
| [Zed terminal.rs](https://github.com/zed-industries/zed/blob/main/crates/terminal/)      | Full file          | Reference implementation to MIRROR         |
| [gpui-component](https://longbridge.github.io/gpui-component/)                           | UI components      | Pre-built components (buttons, tabs, etc.) |

---

## Patterns to Mirror

**MODULE_ORGANIZATION:**
```rust
// SOURCE: src/terminal/mod.rs
// COPY THIS PATTERN for new modules:
pub mod errors;
pub mod handler;
pub mod operations;
pub mod types;

// Re-export public API
pub use errors::TerminalError;
pub use handler::spawn_terminal;
pub use types::{SpawnConfig, SpawnResult, TerminalType};
```

**ERROR_HANDLING:**
```rust
// SOURCE: src/terminal/errors.rs:3-47
// COPY THIS PATTERN:
#[derive(Debug, thiserror::Error)]
pub enum TerminalError {
    #[error("No supported terminal found (tried: iTerm, Terminal.app)")]
    NoTerminalFound,

    #[error("Failed to spawn terminal process: {message}")]
    SpawnFailed { message: String },

    #[error("IO error during terminal operation: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },
}

impl ShardsError for TerminalError {
    fn error_code(&self) -> &'static str {
        match self {
            TerminalError::NoTerminalFound => "NO_TERMINAL_FOUND",
            // ...
        }
    }
}
```

**LOGGING_PATTERN:**
```rust
// SOURCE: src/terminal/handler.rs:13-17
// COPY THIS PATTERN:
info!(
    event = "terminal.spawn_started",
    working_directory = %working_directory.display(),
    command = command
);
```

**CONFIG_PATTERN:**
```rust
// SOURCE: src/core/config.rs:54-64
// COPY THIS PATTERN for UI config:
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ShardsConfig {
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(default)]
    pub terminal: TerminalConfig,
    // ADD:
    #[serde(default)]
    pub ui: UiConfig,
}
```

**SESSION_STATE:**
```rust
// SOURCE: src/sessions/types.rs:9-40
// EXTEND THIS PATTERN:
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    // ... existing fields ...
    // ADD for GPUI:
    #[serde(skip)]
    pub pty_handle: Option<PtyHandle>,  // Runtime only, not serialized
}
```

**HANDLER_PATTERN:**
```rust
// SOURCE: src/sessions/handler.rs:8-20
// COPY THIS PATTERN:
pub fn create_session(request: CreateSessionRequest, config: &ShardsConfig) -> Result<Session, SessionError> {
    // 1. Validate input (pure)
    let validated = operations::validate_session_request(...)?;

    // 2. Perform I/O operations
    let project = git::handler::detect_project()?;

    // 3. Log event
    info!(event = "session.create_started", ...);

    // 4. Return result
    Ok(session)
}
```

---

## Files to Change

| File                                   | Action | Justification                                 |
| -------------------------------------- | ------ | --------------------------------------------- |
| `src/ui/mod.rs`                        | CREATE | New UI module root                            |
| `src/ui/app.rs`                        | CREATE | GPUI Application and window setup             |
| `src/ui/views/mod.rs`                  | CREATE | View components module                        |
| `src/ui/views/main_view.rs`            | CREATE | Main application view with layout             |
| `src/ui/views/terminal_view.rs`        | CREATE | Single terminal rendering view                |
| `src/ui/views/shard_tabs.rs`           | CREATE | Tab bar for shard switching                   |
| `src/ui/views/status_bar.rs`           | CREATE | Bottom status bar                             |
| `src/pty/mod.rs`                       | CREATE | PTY management module root                    |
| `src/pty/types.rs`                     | CREATE | Pty, PtyHandle, OutputBuffer types            |
| `src/pty/handler.rs`                   | CREATE | PTY creation, read, write operations          |
| `src/pty/errors.rs`                    | CREATE | PtyError enum                                 |
| `src/shard_manager/mod.rs`             | CREATE | Multi-shard orchestration                     |
| `src/shard_manager/types.rs`           | CREATE | ManagedShard, ShardStatus types               |
| `src/shard_manager/handler.rs`         | CREATE | create_shard, destroy_shard, list_shards      |
| `src/core/config.rs`                   | UPDATE | Add UiConfig struct                           |
| `src/sessions/types.rs`                | UPDATE | Add pty_handle field (runtime only)           |
| `src/cli/app.rs`                       | UPDATE | Add `ui` subcommand                           |
| `src/lib.rs`                           | UPDATE | Export new modules                            |
| `src/main.rs`                          | UPDATE | Handle `ui` command to launch GPUI app        |
| `Cargo.toml`                           | UPDATE | Add gpui, alacritty_terminal dependencies     |

---

## NOT Building (Scope Limits)

Explicit exclusions to prevent scope creep:

- **Windows support** - GPUI doesn't support Windows yet; defer to future
- **Remote terminals** - SSH connections; out of scope for v1
- **Terminal multiplexing** - Splits within a single terminal (tmux-style)
- **Custom themes** - Use GPUI defaults initially
- **Plugin system** - No extensibility hooks in v1
- **Settings UI** - Config via TOML files, not GUI settings panel
- **Session sync** - No cloud sync of sessions across machines
- **Collaborative features** - Single-user only for v1

---

## Step-by-Step Tasks

Execute in order. Each task is atomic and independently verifiable.

### Task 1: UPDATE `Cargo.toml` - Add dependencies

- **ACTION**: ADD gpui, alacritty_terminal, and supporting dependencies
- **IMPLEMENT**:
  ```toml
  [dependencies]
  gpui = "0.1"
  alacritty_terminal = "0.24"
  tokio = { version = "1", features = ["full", "sync"] }
  parking_lot = "0.12"  # FairMutex for terminal
  ```
- **MIRROR**: Existing dependency style in Cargo.toml
- **GOTCHA**: GPUI may require git dependency if crates.io version is outdated
- **VALIDATE**: `cargo check`

### Task 2: CREATE `src/pty/types.rs` - PTY type definitions

- **ACTION**: CREATE PTY-related type definitions
- **IMPLEMENT**:
  ```rust
  pub struct PtyHandle {
      pub reader: Box<dyn Read + Send>,
      pub writer: Box<dyn Write + Send>,
      pub child: Child,
  }

  pub struct OutputBuffer {
      lines: VecDeque<String>,
      max_lines: usize,
  }

  pub enum PtyStatus {
      Running,
      Exited(i32),
      Killed,
  }
  ```
- **MIRROR**: `src/terminal/types.rs:4-24` naming conventions
- **VALIDATE**: `cargo check`

### Task 3: CREATE `src/pty/errors.rs` - PTY error types

- **ACTION**: CREATE PtyError enum implementing ShardsError
- **IMPLEMENT**: SpawnFailed, ReadError, WriteError, ProcessExited
- **MIRROR**: `src/terminal/errors.rs:3-47`
- **IMPORTS**: `use crate::core::errors::ShardsError`
- **VALIDATE**: `cargo check`

### Task 4: CREATE `src/pty/handler.rs` - PTY operations

- **ACTION**: CREATE PTY spawn and I/O functions
- **IMPLEMENT**:
  ```rust
  pub fn spawn_pty(command: &str, working_dir: &Path, size: PtySize) -> Result<PtyHandle, PtyError>
  pub fn write_to_pty(handle: &mut PtyHandle, data: &[u8]) -> Result<(), PtyError>
  pub fn read_from_pty(handle: &mut PtyHandle, buffer: &mut [u8]) -> Result<usize, PtyError>
  pub fn resize_pty(handle: &PtyHandle, size: PtySize) -> Result<(), PtyError>
  ```
- **MIRROR**: `src/terminal/handler.rs:8-88` logging pattern
- **IMPORTS**: `use alacritty_terminal::tty::{Pty, Options}`
- **GOTCHA**: PTY reads are blocking - need async wrapper or thread
- **VALIDATE**: `cargo check`

### Task 5: CREATE `src/pty/mod.rs` - Module exports

- **ACTION**: CREATE module file with public API
- **IMPLEMENT**: Export types, handler functions, errors
- **MIRROR**: `src/terminal/mod.rs`
- **VALIDATE**: `cargo check`

### Task 6: CREATE `src/shard_manager/types.rs` - Managed shard types

- **ACTION**: CREATE types for shard orchestration
- **IMPLEMENT**:
  ```rust
  pub struct ManagedShard {
      pub session: Session,
      pub pty: PtyHandle,
      pub term: Arc<FairMutex<Term<ShardListener>>>,
      pub output_buffer: OutputBuffer,
      pub status: ShardStatus,
  }

  pub enum ShardStatus {
      Starting,
      Running,
      Idle,
      Stopped,
  }
  ```
- **MIRROR**: `src/sessions/types.rs:9-40`
- **VALIDATE**: `cargo check`

### Task 7: CREATE `src/shard_manager/handler.rs` - Shard management

- **ACTION**: CREATE shard lifecycle management
- **IMPLEMENT**:
  ```rust
  pub struct ShardManager {
      shards: HashMap<String, ManagedShard>,
  }

  impl ShardManager {
      pub fn create_shard(&mut self, name: &str, agent: &str) -> Result<&ManagedShard, ShardError>
      pub fn destroy_shard(&mut self, name: &str) -> Result<(), ShardError>
      pub fn get_shard(&self, name: &str) -> Option<&ManagedShard>
      pub fn send_to_shard(&mut self, name: &str, text: &str) -> Result<(), ShardError>
      pub fn read_shard_output(&self, name: &str, lines: usize) -> Option<Vec<String>>
      pub fn list_shards(&self) -> Vec<&ManagedShard>
  }
  ```
- **MIRROR**: `src/sessions/handler.rs:8-109` pattern
- **VALIDATE**: `cargo check`

### Task 8: CREATE `src/shard_manager/errors.rs` - Shard errors

- **ACTION**: CREATE ShardError enum
- **IMPLEMENT**: ShardNotFound, ShardAlreadyExists, PtyError, SendFailed
- **MIRROR**: `src/terminal/errors.rs:3-47`
- **VALIDATE**: `cargo check`

### Task 9: CREATE `src/shard_manager/mod.rs` - Module exports

- **ACTION**: CREATE module file
- **MIRROR**: `src/sessions/mod.rs`
- **VALIDATE**: `cargo check`

### Task 10: CREATE `src/ui/app.rs` - GPUI application setup

- **ACTION**: CREATE main GPUI application and window
- **IMPLEMENT**:
  ```rust
  pub fn run_ui(config: &ShardsConfig) -> Result<(), Box<dyn Error>> {
      App::new().run(|cx: &mut AppContext| {
          cx.open_window(
              WindowOptions { /* ... */ },
              |cx| cx.new_view(|cx| MainView::new(cx))
          );
      });
  }
  ```
- **MIRROR**: Zed's app initialization patterns
- **GOTCHA**: GPUI requires main thread execution
- **VALIDATE**: `cargo check`

### Task 11: CREATE `src/ui/views/terminal_view.rs` - Terminal rendering

- **ACTION**: CREATE terminal view component using alacritty_terminal
- **IMPLEMENT**:
  ```rust
  pub struct TerminalView {
      term: Arc<FairMutex<Term<ViewListener>>>,
      pty: PtyHandle,
  }

  impl Render for TerminalView {
      fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
          // Render terminal grid to GPUI elements
      }
  }
  ```
- **MIRROR**: Zed's terminal.rs:178-233
- **GOTCHA**: Terminal rendering is complex - start with basic text grid
- **VALIDATE**: `cargo check`

### Task 12: CREATE `src/ui/views/shard_tabs.rs` - Tab bar

- **ACTION**: CREATE shard tab bar component
- **IMPLEMENT**: Tab list, active indicator, add button, close button
- **MIRROR**: Standard GPUI tab patterns
- **VALIDATE**: `cargo check`

### Task 13: CREATE `src/ui/views/main_view.rs` - Main layout

- **ACTION**: CREATE main view composing all components
- **IMPLEMENT**:
  ```rust
  pub struct MainView {
      main_terminal: View<TerminalView>,
      shard_tabs: View<ShardTabs>,
      active_shard: Option<View<TerminalView>>,
      shard_manager: Model<ShardManager>,
  }
  ```
- **MIRROR**: Zed's workspace layout patterns
- **VALIDATE**: `cargo check`

### Task 14: CREATE `src/ui/views/status_bar.rs` - Status bar

- **ACTION**: CREATE bottom status bar
- **IMPLEMENT**: Shard count, active shard name, status indicators
- **VALIDATE**: `cargo check`

### Task 15: CREATE `src/ui/views/mod.rs` - Views module

- **ACTION**: CREATE module exports
- **VALIDATE**: `cargo check`

### Task 16: CREATE `src/ui/mod.rs` - UI module root

- **ACTION**: CREATE module with app and views exports
- **VALIDATE**: `cargo check`

### Task 17: UPDATE `src/core/config.rs` - Add UI config

- **ACTION**: ADD UiConfig struct to ShardsConfig
- **IMPLEMENT**:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize, Default)]
  pub struct UiConfig {
      #[serde(default = "default_font_size")]
      pub font_size: f32,
      #[serde(default)]
      pub theme: Option<String>,
  }
  ```
- **MIRROR**: Existing config patterns
- **VALIDATE**: `cargo check`

### Task 18: UPDATE `src/cli/app.rs` and `src/main.rs` - Add ui command

- **ACTION**: ADD `ui` subcommand to launch GPUI app
- **IMPLEMENT**:
  ```rust
  .subcommand(
      Command::new("ui")
          .about("Launch the Shards graphical interface")
  )
  ```
- **MIRROR**: Existing subcommand pattern
- **VALIDATE**: `cargo check && cargo run -- ui`

---

## Testing Strategy

### Unit Tests to Write

| Test File                        | Test Cases                              | Validates                |
| -------------------------------- | --------------------------------------- | ------------------------ |
| `src/pty/tests/types_test.rs`    | OutputBuffer push/read, capacity        | Buffer behavior          |
| `src/pty/tests/handler_test.rs`  | spawn_pty mock, write/read simulation   | PTY operations           |
| `src/shard_manager/tests/*.rs`   | create, destroy, list, send             | Shard lifecycle          |

### Integration Tests

| Test                          | Description                                        |
| ----------------------------- | -------------------------------------------------- |
| `test_pty_echo`               | Spawn shell, write command, read output            |
| `test_shard_lifecycle`        | Create shard, send prompt, read response, destroy  |
| `test_multiple_shards`        | Create 3 shards, interact with each                |

### Edge Cases Checklist

- [ ] PTY spawn fails (command not found)
- [ ] PTY process exits unexpectedly
- [ ] Read from closed PTY
- [ ] Write to closed PTY
- [ ] Create shard with duplicate name
- [ ] Destroy non-existent shard
- [ ] Send to stopped shard
- [ ] Buffer overflow (max lines exceeded)
- [ ] Resize terminal during operation
- [ ] Unicode in terminal output
- [ ] ANSI escape sequences rendering
- [ ] Rapid input (typing speed)

---

## Validation Commands

### Level 1: STATIC_ANALYSIS

```bash
cargo check && cargo clippy
```

**EXPECT**: Exit 0, no errors or warnings

### Level 2: UNIT_TESTS

```bash
cargo test --lib
```

**EXPECT**: All tests pass

### Level 3: FULL_BUILD

```bash
cargo build --release
```

**EXPECT**: Binary compiles successfully

### Level 4: SMOKE_TEST

```bash
cargo run -- ui
# Should open window, display terminal, accept input
```

**EXPECT**: Window opens, no crashes

### Level 5: PTY_VALIDATION

```bash
# In UI:
# 1. Create shard
# 2. Type "echo hello"
# 3. Verify "hello" appears in output
```

**EXPECT**: Echo command works

### Level 6: ORCHESTRATION_VALIDATION

```bash
# In main session:
# 1. Create shard named "test"
# 2. Type: @shard:test "echo orchestrated"
# 3. Verify output appears in test shard
# 4. Verify main session can read output
```

**EXPECT**: Cross-shard communication works

---

## Acceptance Criteria

- [ ] GPUI window opens with main terminal view
- [ ] Can type in terminal and see output
- [ ] Can create new shard via UI
- [ ] Shard tabs display correctly
- [ ] Can switch between shards
- [ ] Can send prompt to shard from main session
- [ ] Can read shard output from main session
- [ ] Session persistence works (shards survive restart)
- [ ] `cargo clippy` passes with no warnings
- [ ] All unit tests pass
- [ ] Works on macOS and Linux

---

## Completion Checklist

- [ ] All 18 tasks completed in dependency order
- [ ] Each task validated immediately after completion
- [ ] Level 1: `cargo check && cargo clippy` passes
- [ ] Level 2: `cargo test --lib` passes
- [ ] Level 3: `cargo build --release` succeeds
- [ ] Level 4: Smoke test - window opens
- [ ] Level 5: PTY validation - echo works
- [ ] Level 6: Orchestration validation - cross-shard works
- [ ] All acceptance criteria met

---

## Risks and Mitigations

| Risk                              | Likelihood | Impact | Mitigation                                              |
| --------------------------------- | ---------- | ------ | ------------------------------------------------------- |
| GPUI API instability (pre-1.0)    | HIGH       | HIGH   | Pin to specific commit, prepare for breaking changes    |
| alacritty_terminal complexity     | MEDIUM     | HIGH   | Start with basic rendering, iterate on correctness      |
| PTY blocking I/O in UI thread     | HIGH       | HIGH   | Use background thread + channel for PTY I/O             |
| Terminal rendering performance    | MEDIUM     | MEDIUM | Batch updates, use dirty rect tracking                  |
| Cross-platform PTY differences    | MEDIUM     | MEDIUM | Test on both macOS and Linux early                      |
| Main agent orchestration UX       | LOW        | MEDIUM | Start with explicit commands, iterate on natural syntax |

---

## Notes

### Zed's Terminal Architecture (Reference)

Based on [Zed's terminal.rs](https://github.com/zed-industries/zed/blob/main/crates/terminal/src/terminal.rs):

```rust
// Key struct fields from Zed:
pub struct Terminal {
    pty_tx: Notifier,                           // PTY write channel
    completion_tx: Sender<Option<ExitStatus>>,  // Exit notification
    term: Arc<FairMutex<Term<ZedListener>>>,    // Terminal state
    events: VecDeque<InternalEvent>,            // Event queue
}
```

Key patterns:
- Uses `FairMutex` from `parking_lot` for terminal access
- `ZedListener` implements `alacritty_terminal::event::EventListener`
- PTY I/O happens on background thread
- Events marshaled to UI thread via channels

### GPUI Rendering Model

- Views implement `Render` trait
- `render()` called each frame (target 120fps)
- Immediate mode: rebuild entire view tree each frame
- State in Models, accessed via `cx.observe()`
- Tailwind-style styling API

### Future Considerations

1. **Agent SDK integration** - Main session could use Claude Agent SDK for native orchestration
2. **MCP tools** - Expose shard management as MCP tools for agents
3. **Session streaming** - Stream session transcripts for analysis
4. **Collaboration** - Multi-user shard viewing (long-term)
