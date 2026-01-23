# Project Structure

## Directory Layout
```
SHARDS/
├── crates/                # Cargo workspace crates
│   ├── shards-core/       # Core library with all business logic
│   │   ├── src/
│   │   │   ├── lib.rs     # Library root with public exports
│   │   │   ├── config/    # Foundation: configuration
│   │   │   │   ├── mod.rs       # Config module exports
│   │   │   │   ├── types.rs     # ShardsConfig, AgentConfig, etc.
│   │   │   │   ├── loading.rs   # Config file loading logic
│   │   │   │   ├── defaults.rs  # Default values
│   │   │   │   └── validation.rs # Config validation
│   │   │   ├── logging/   # Foundation: structured logging
│   │   │   │   └── mod.rs # init_logging() with JSON output
│   │   │   ├── errors/    # Foundation: base error traits
│   │   │   │   └── mod.rs # ShardsError trait, ConfigError
│   │   │   ├── events/    # Foundation: application lifecycle
│   │   │   │   └── mod.rs # log_app_startup(), log_app_error()
│   │   │   ├── sessions/  # Feature slice: session lifecycle
│   │   │   │   ├── mod.rs         # Public API exports
│   │   │   │   ├── handler.rs     # create/list/destroy/restart_session
│   │   │   │   ├── operations.rs  # Pure business logic
│   │   │   │   ├── types.rs       # Session, CreateSessionRequest
│   │   │   │   ├── errors.rs      # SessionError
│   │   │   │   ├── ports.rs       # Port range allocation
│   │   │   │   ├── persistence.rs # Session file I/O
│   │   │   │   └── validation.rs  # Input validation
│   │   │   ├── git/       # Feature slice: worktree management
│   │   │   │   ├── mod.rs         # Public API exports
│   │   │   │   ├── handler.rs     # detect_project, create/remove_worktree
│   │   │   │   ├── operations.rs  # Pure git logic
│   │   │   │   ├── types.rs       # Project, Worktree
│   │   │   │   └── errors.rs      # GitError
│   │   │   ├── terminal/  # Feature slice: terminal launching
│   │   │   │   ├── mod.rs         # Public API exports
│   │   │   │   ├── handler.rs     # spawn_terminal, close_terminal
│   │   │   │   ├── operations.rs  # Terminal detection
│   │   │   │   ├── traits.rs      # TerminalBackend trait
│   │   │   │   ├── registry.rs    # Backend registration
│   │   │   │   ├── types.rs       # SpawnConfig, SpawnResult, TerminalType
│   │   │   │   ├── errors.rs      # TerminalError
│   │   │   │   ├── backends/      # Terminal implementations
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── ghostty.rs
│   │   │   │   │   ├── iterm.rs
│   │   │   │   │   └── terminal_app.rs
│   │   │   │   └── common/        # Shared utilities
│   │   │   │       ├── mod.rs
│   │   │   │       ├── applescript.rs
│   │   │   │       ├── detection.rs
│   │   │   │       └── escape.rs
│   │   │   ├── agents/    # Feature slice: agent backend system
│   │   │   │   ├── mod.rs         # Public API exports
│   │   │   │   ├── traits.rs      # AgentBackend trait
│   │   │   │   ├── registry.rs    # Agent lookup and validation
│   │   │   │   ├── types.rs       # AgentType enum
│   │   │   │   ├── errors.rs      # AgentError
│   │   │   │   └── backends/      # Agent implementations
│   │   │   │       ├── mod.rs
│   │   │   │       ├── claude.rs
│   │   │   │       ├── kiro.rs
│   │   │   │       ├── gemini.rs
│   │   │   │       ├── codex.rs
│   │   │   │       └── aether.rs
│   │   │   ├── health/    # Feature slice: health monitoring
│   │   │   │   ├── mod.rs, handler.rs, operations.rs
│   │   │   │   ├── types.rs       # HealthOutput, ShardHealth, HealthStatus
│   │   │   │   ├── errors.rs, storage.rs
│   │   │   ├── cleanup/   # Feature slice: cleanup operations
│   │   │   │   ├── mod.rs, handler.rs, operations.rs
│   │   │   │   ├── types.rs       # CleanupStrategy, CleanupSummary
│   │   │   │   └── errors.rs
│   │   │   ├── process/   # Feature slice: process management
│   │   │   │   ├── mod.rs, operations.rs
│   │   │   │   ├── types.rs       # ProcessInfo, ProcessStatus
│   │   │   │   ├── errors.rs, pid_file.rs
│   │   │   └── files/     # Feature slice: file operations
│   │   │       ├── mod.rs, handler.rs, operations.rs
│   │   │       ├── types.rs       # IncludeConfig
│   │   │       └── errors.rs
│   │   └── Cargo.toml     # Core library dependencies
│   ├── shards/            # CLI binary
│   │   ├── src/
│   │   │   ├── main.rs    # CLI entry point (calls init_logging)
│   │   │   ├── app.rs     # Clap application definition
│   │   │   ├── commands.rs # CLI command handlers
│   │   │   └── table.rs   # Table formatting utilities
│   │   └── Cargo.toml     # CLI binary dependencies
│   └── shards-ui/         # Future: TUI (placeholder)
│       └── Cargo.toml     # UI dependencies
├── .shards/               # Project-level config directory
│   └── config.toml        # Project-specific configuration
├── ~/.shards/             # User-level data directory (at runtime)
│   ├── config.toml        # User configuration
│   ├── sessions/          # Session JSON files
│   └── worktrees/         # Worktree directories
│       └── <project>/
│           └── <branch>/
├── .kiro/                 # Kiro CLI configuration and steering docs
│   └── steering/          # Project steering documentation
├── target/                # Cargo build artifacts
├── Cargo.toml             # Workspace manifest
├── Cargo.lock             # Dependency lock file
├── CLAUDE.md              # Claude Code instructions (source of truth)
└── README.md              # Project documentation
```

## Architecture Principles

### Cargo Workspace Structure
- **Multi-crate workspace**: Core library (`shards-core`) + CLI binary (`shards`) + UI placeholder (`shards-ui`)
- **Clear separation**: Business logic in core library, CLI implementation in binary crate
- **Code sharing**: Both CLI and future UI can depend on the same core library

### Vertical Slice Architecture
- **Feature-based organization**: Each feature (sessions, git, terminal, health, cleanup, etc.) is self-contained
- **Handler/Operations pattern**: `handler.rs` for I/O orchestration, `operations.rs` for pure logic
- **Feature-specific types and errors**: Each slice defines its own domain types
- **Minimal coupling**: Features interact through well-defined interfaces

### Core Infrastructure
- **Foundation services**: Configuration, logging, base errors, lifecycle events
- **Shared only when needed**: Code moves to shared utilities only when 3+ features need it
- **No premature abstraction**: Prefer duplication over wrong abstraction

## File Naming Conventions
- **Rust modules**: Snake case (e.g., `handler.rs`, `operations.rs`, `types.rs`)
- **Branch names**: User-defined branch names for shards
- **Worktree directories**: `~/.shards/worktrees/<project>/<branch>/`
- **Session files**: `~/.shards/sessions/<session-id>.json`
- **Session ID format**: `<project-id>_<branch-name>`

## Module Organization

### CLI Binary (`crates/shards/src/`)
- **main.rs**: Entry point, initializes logging and runs CLI
- **app.rs**: Clap application definition with command structure
- **commands.rs**: Command handlers that delegate to `shards-core` library
- **table.rs**: Table formatting utilities for CLI output
- **Thin layer**: Minimal logic, delegates to core library

### Core Library (`crates/shards-core/src/`)
- **lib.rs**: Library root with public API exports
- **config/**: Application configuration and environment setup
- **logging/**: Structured JSON logging with tracing
- **errors/**: Base error traits and common error handling
- **events/**: Application lifecycle events (startup, shutdown, errors)

### Feature Slices (in `shards-core`)
Each feature follows the same pattern:
- **mod.rs**: Public API exports for the feature
- **handler.rs**: I/O orchestration with structured logging
- **operations.rs**: Pure business logic (no I/O, easily testable)
- **types.rs**: Feature-specific data structures
- **errors.rs**: Feature-specific error types with thiserror

Feature slices include: `sessions/`, `git/`, `terminal/`, `health/`, `cleanup/`, `process/`, `files/`

## Configuration Files
- **Cargo.toml** (root): Workspace manifest with shared dependencies
- **crates/shards-core/Cargo.toml**: Core library dependencies
- **crates/shards/Cargo.toml**: CLI binary dependencies
- **crates/shards-ui/Cargo.toml**: UI placeholder dependencies
- **~/.shards/sessions/*.json**: Session persistence files
- **~/.shards/config.toml**: User-level configuration
- **.shards/config.toml**: Project-level configuration
- **.gitignore**: Excludes build artifacts and local config

### Configuration Hierarchy (highest priority wins)
1. CLI arguments
2. Project config (`.shards/config.toml`)
3. User config (`~/.shards/config.toml`)
4. Defaults

## Documentation Structure
- **CLAUDE.md**: Primary source of truth for AI agents (checked into repo)
- **README.md**: User-facing documentation with usage examples
- **.kiro/steering/**: Supplementary project documentation
  - `architecture.md`: Architecture patterns and logging conventions
  - `product.md`: Product requirements and objectives
  - `progress.md`: Development log (historical)
  - `tech.md`: Technical stack details
  - `structure.md`: This file - project organization
  - `ai-instruction.md`: AI agent usage instructions
- **Inline documentation**: Rust doc comments for public APIs

## Build Artifacts
- **target/**: Cargo build output directory
  - `target/debug/`: Development builds
  - `target/release/`: Optimized release builds
- **Cargo.lock**: Dependency resolution lock file
- Build artifacts are excluded from version control

## Testing Strategy
- **Unit tests**: Collocated with code, especially in `operations.rs` modules
- **Integration tests**: Cross-feature workflows in `tests/` directory
- **Manual testing**: CLI command validation and platform-specific testing
