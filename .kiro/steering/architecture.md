# Architecture

## Vertical Slice Architecture

**Core Principle**: Organize by features, not layers. Each slice is self-contained with everything needed to understand and modify that feature.

**Note**: Project uses Cargo workspace structure with core library and CLI binary separated.

```
crates/
├── shards-core/       # Core library with all business logic
│   └── src/
│       ├── config/    # Foundation: configuration (loading, defaults, validation, types)
│       ├── logging/   # Foundation: structured logging
│       ├── errors/    # Foundation: base error traits
│       ├── events/    # Foundation: application lifecycle
│       ├── sessions/  # Feature slice: session lifecycle
│       ├── git/       # Feature slice: worktree management
│       ├── terminal/  # Feature slice: terminal launching
│       │   ├── backends/   # Terminal implementations (ghostty, iterm, terminal_app)
│       │   └── common/     # Shared utilities (applescript, detection, escape)
│       ├── agents/    # Feature slice: agent backend system
│       │   └── backends/   # Agent implementations (amp, claude, kiro, gemini, codex)
│       ├── health/    # Feature slice: health monitoring
│       ├── cleanup/   # Feature slice: cleanup operations
│       ├── process/   # Feature slice: process management
│       └── files/     # Feature slice: file operations
├── shards/            # CLI binary
│   └── src/
│       ├── main.rs    # CLI entry point
│       ├── app.rs     # Clap application definition
│       ├── commands.rs # CLI command handlers
│       └── table.rs   # Table formatting utilities
└── shards-ui/         # Future: TUI (minimal)
```

### Feature Slice Structure
Each feature contains its complete implementation:

```
crates/shards-core/src/sessions/
├── mod.rs             # Public API exports
├── handler.rs         # Orchestration (uses infrastructure)
├── operations.rs      # Pure business logic (no I/O)
├── types.rs           # Feature-specific types
├── errors.rs          # Feature-specific errors
└── tests.rs           # Feature tests
```

**Key Pattern**: `handler.rs` orchestrates I/O, `operations.rs` contains pure logic. This makes business logic trivially testable.

### Core Infrastructure
Universal foundation that exists before features:

```
crates/shards-core/src/
├── config/            # Application configuration
├── logging/           # Structured logging setup
├── errors/            # Base error traits
└── events/            # Application lifecycle
```

**Rule**: If removing any feature slice would still require this code, it belongs in core infrastructure.

### Three-Feature Rule
Code moves to `shared/` only when three features need it. Until then, duplicate it. Prefer coupling to feature over premature abstraction.

## Logging Strategy

**Event Naming Convention**: `{layer}.{domain}.{action}_{state}`

| Layer | Crate | Description |
|-------|-------|-------------|
| `cli` | `crates/shards/` | User-facing CLI commands |
| `core` | `crates/shards-core/` | Core library logic |

**Domains**: `session`, `terminal`, `git`, `cleanup`, `health`, `files`, `process`, `pid_file`, `app`

**State suffixes**: `_started`, `_completed`, `_failed`, `_skipped`

### Standard Events

| Event | When |
|-------|------|
| `cli.create_started` | CLI command begins |
| `core.session.create_started` | Handler begins |
| `core.session.create_completed` | Success |
| `core.session.create_failed` | Failure |
| `core.git.worktree.create_completed` | Worktree added |
| `core.terminal.spawn_completed` | Terminal launched |

### Implementation Pattern

```rust
use tracing::{info, error, warn};

pub fn create_session(request: CreateSessionRequest, config: &ShardsConfig) -> Result<Session, SessionError> {
    info!(
        event = "core.session.create_started",
        branch = request.branch,
        agent = agent
    );

    // ... operations ...

    info!(
        event = "core.session.create_completed",
        session_id = session.id,
        branch = session.branch,
        process_id = session.process_id
    );
    Ok(session)
}
```

### Required Fields by Context

- **Session ops**: `branch`, `session_id`, `agent`, `process_id`
- **Git ops**: `branch`, `worktree_path`, `project_id`
- **Terminal ops**: `terminal_type`, `window_title`, `working_directory`
- **Errors**: `error` (with `%e` for Display format)

### Log Levels

| Level | Use For |
|-------|---------|
| `error!` | Operation failed, requires attention |
| `warn!` | Degraded operation, fallback used, non-critical issues |
| `info!` | Operation lifecycle (_started, _completed), user-relevant events |
| `debug!` | Internal state, retry attempts, detailed flow |

### Structured Logging Setup

```rust
// crates/shards-core/src/logging/mod.rs
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init_logging() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_current_span(false)
                .with_span_list(false)
        )
        .with(EnvFilter::from_default_env().add_directive("shards=info".parse().unwrap()))
        .init();
}
```

**Key Benefits**:
- Grep-friendly: `grep "_failed"` finds all failures
- Layer-aware: `grep "core\."` for library, `grep "cli\."` for CLI
- Hierarchical: `grep "core\.session\."` finds all session events
- AI-parseable: Structured JSON for log analysis

## Handler/Operations Pattern

### Handler Layer (I/O Orchestration)
```rust
// crates/shards-core/src/sessions/handler.rs
pub fn create_session(
    request: CreateSessionRequest,
    shards_config: &ShardsConfig,
) -> Result<Session, SessionError> {
    let agent = request.agent_or_default(&shards_config.agent.default);

    info!(event = "core.session.create_started", branch = request.branch, agent = agent);

    // 1. Validate input (pure)
    let validated = operations::validate_session_request(&request.branch, &agent_command, &agent)?;

    // 2. Detect git project (I/O)
    let project = git::handler::detect_project()?;

    // 3. Allocate port range (I/O)
    let (port_start, port_end) = operations::allocate_port_range(&sessions_dir, port_count, base_port)?;

    // 4. Create worktree (I/O)
    let worktree = git::handler::create_worktree(&shards_dir, &project, &validated.name, Some(shards_config))?;

    // 5. Launch terminal (I/O)
    let spawn_result = terminal::handler::spawn_terminal(&worktree.path, &validated.command, shards_config, Some(&session_id), Some(&shards_dir))?;

    // 6. Save session (I/O)
    operations::save_session_to_file(&session, &sessions_dir)?;

    info!(event = "core.session.create_completed", session_id = session.id, branch = validated.name);
    Ok(session)
}
```

### Operations Layer (Pure Logic)
```rust
// crates/shards-core/src/sessions/operations.rs
pub fn validate_session_request(branch: &str, command: &str, agent: &str) -> Result<ValidatedRequest, SessionError> {
    if branch.is_empty() {
        return Err(SessionError::InvalidName { reason: "Branch name cannot be empty".to_string() });
    }
    if command.is_empty() {
        return Err(SessionError::InvalidCommand { reason: "Command cannot be empty".to_string() });
    }
    Ok(ValidatedRequest { name: branch.to_string(), command: command.to_string(), agent: agent.to_string() })
}

pub fn allocate_port_range(sessions_dir: &Path, count: u16, base: u16) -> Result<(u16, u16), SessionError> {
    // Finds next available port range by scanning existing sessions
    // Returns (start_port, end_port)
}

pub fn generate_session_id(project_id: &str, branch: &str) -> String {
    format!("{}_{}", project_id, branch)
}
```

**Benefits**:
- `operations.rs` is trivially testable (no I/O)
- `handler.rs` orchestrates the workflow
- Clear separation of concerns
- AI can understand the flow easily

## Error Handling

Feature-specific errors with grep-able variants:

```rust
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Session '{name}' already exists")]
    AlreadyExists { name: String },
    
    #[error("Session '{name}' not found")]
    NotFound { name: String },
    
    #[error("Invalid session name")]
    InvalidName,
    
    #[error("Git operation failed: {source}")]
    GitError { 
        #[from]
        source: git2::Error 
    },
}
```

**No `unwrap()` or `expect()` in production code** - use `?` or explicit error handling.

## Testing Strategy

### Unit Tests (Collocated)
```rust
// In operations.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_session_request_success() {
        let result = validate_session_request("test", "echo hello");
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_calculate_port_range() {
        assert_eq!(calculate_port_range(0), (3000, 3099));
        assert_eq!(calculate_port_range(1), (3100, 3199));
    }
}
```

### Integration Tests
```rust
// tests/session_lifecycle.rs
#[test]
fn test_create_and_cleanup_session() {
    // Test sessions + git + terminal integration
}
```

## Migration Path

Current → Target:
1. Extract `core/logging.rs` with structured logging
2. Create feature slices (`sessions/`, `git/`, `terminal/`)
3. Split each feature into `handler.rs` + `operations.rs`
4. Add comprehensive logging to all handlers
5. Move tests to be collocated with code

**No backwards compatibility needed** - we can break things to get architecture right.
