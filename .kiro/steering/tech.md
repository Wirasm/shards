# Technical Architecture

## Technology Stack
- **Language**: Rust (2024 edition)
- **CLI Framework**: clap 4.0 with derive macros
- **Git Operations**: git2 crate for worktree management
- **Terminal Integration**: Platform-specific terminal launching (osascript for macOS, gnome-terminal/konsole for Linux, Windows Terminal/cmd for Windows)
- **Session Storage**: JSON-based registry in `~/.shards/registry.json`
- **Process Management**: Standard library process spawning
- **Cross-platform Support**: Conditional compilation for platform-specific features

## Architecture Overview
```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   CLI Parser    │───▶│  Git Manager     │───▶│  Worktree       │
│   (clap)        │    │  (git2)          │    │  (.shards/*)    │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         │                       │
         ▼                       ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│ Session Registry│    │Terminal Launcher │───▶│ Native Terminal │
│ (~/.shards/)    │    │ (platform-spec)  │    │ (agent process) │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

## Development Environment
- Rust 1.89.0 or later
- Git repository (required for worktree operations)
- Platform-specific terminal emulator

## Code Standards
- Minimal implementations following the principle of least code
- Error handling with anyhow for user-friendly error messages
- Structured logging and clear user feedback
- Cross-platform compatibility with conditional compilation

## Testing Strategy
- Manual testing of CLI commands and workflows
- Integration testing of Git worktree operations
- Platform-specific testing for terminal launching

## Deployment Process
- Cargo build for local development
- Future: Binary releases for multiple platforms

## Performance Requirements
- Fast startup time for CLI operations
- Efficient Git operations for worktree management
- Minimal resource usage for session tracking

## Security Considerations
- No sensitive data storage
- Safe file system operations
- Proper cleanup of temporary resources
