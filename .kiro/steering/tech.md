# Technical Architecture

## Technology Stack
- **Language**: Rust (2024 edition)
- **CLI Framework**: clap 4.0 with derive macros
- **Git Operations**: git2 crate for worktree management
- **Terminal Integration**: Platform-specific terminal launching (Ghostty > iTerm > Terminal.app on macOS)
- **Session Storage**: File-based JSON persistence in `~/.shards/sessions/`
- **Configuration**: Hierarchical TOML config (CLI → project → user → defaults)
- **Logging**: Structured JSON logging with tracing and tracing-subscriber
- **Error Handling**: thiserror for feature-specific error types
- **Process Management**: sysinfo crate for process monitoring
- **Cross-platform Support**: Conditional compilation for platform-specific features

## Architecture Overview
```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   CLI Parser    │───▶│  Sessions        │───▶│  Git Handler    │
│   (clap)        │    │  Handler         │    │  (git2)         │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         │                       │                       │
         │                       ▼                       ▼
         │              ┌──────────────────┐    ┌─────────────────┐
         │              │  Terminal        │    │  Worktree       │
         │              │  Handler         │    │  (~/.shards/)   │
         │              └──────────────────┘    └─────────────────┘
         │                       │
         │                       ▼
         │              ┌──────────────────┐
         │              │  Agents          │
         │              │  Registry        │
         │              └──────────────────┘
         │                       │
         ▼                       ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│  Core Logging   │    │ Native Terminal  │    │  Health/Cleanup │
│  & Events       │    │ (agent process)  │    │  Monitoring     │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

## Vertical Slice Architecture

### Feature Slices
- **sessions/**: Session lifecycle (create, list, destroy, restart, status)
- **git/**: Git worktree operations via git2
- **terminal/**: Multi-backend terminal abstraction (Ghostty, iTerm, Terminal.app)
- **agents/**: Agent backend system (claude, kiro, gemini, codex, aether)
- **health/**: Session health monitoring with CPU/memory metrics
- **cleanup/**: Orphaned resource cleanup with multiple strategies
- **process/**: PID tracking with process reuse protection
- **files/**: File operations for worktree setup

### Core Infrastructure
- **config/**: Hierarchical TOML configuration (loading, defaults, validation, types)
- **logging/**: Structured JSON logging setup with tracing
- **errors/**: Base ShardsError trait with error_code() and is_user_error()
- **events/**: Application lifecycle events (startup, shutdown, errors)

## Development Environment
- Rust 1.89.0 or later (2024 edition)
- Git repository (required for worktree operations)
- macOS with Ghostty, iTerm, or Terminal.app

## Code Standards
- **Vertical slice architecture**: Features organized by domain, not layers
- **Handler/Operations pattern**: I/O orchestration separate from pure business logic
- **Structured logging**: Event naming `{layer}.{domain}.{action}_{state}`
- **Feature-specific errors**: thiserror-based with error codes
- **No unwrap/expect**: Explicit error handling with `?` operator
- **No silent failures**: Always surface errors, log fallbacks
- **Type safety**: Full use of Rust's type system

## Testing Strategy
- **Unit tests**: Collocated with code in `#[cfg(test)]` modules
- **Integration tests**: Session lifecycle workflows
- **All PRs must pass**: `cargo fmt --check`, `cargo clippy --all -- -D warnings`, `cargo test --all`

## Deployment Process
- `cargo build --all` for local development
- `cargo build --release` for optimized builds

## Performance Requirements
- Fast startup time for CLI operations
- Efficient Git operations for worktree management
- Minimal resource usage for session tracking

## Security Considerations
- No sensitive data storage in session files
- Safe file system operations with proper error handling
- Proper cleanup of temporary resources
- Branch name validation to prevent injection
