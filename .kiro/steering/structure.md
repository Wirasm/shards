# Project Structure

## Directory Layout
```
SHARDS/
├── src/                    # Rust source code
│   ├── main.rs            # CLI entry point and command handling
│   ├── git.rs             # Git worktree management module
│   ├── terminal.rs        # Cross-platform terminal launching
│   └── registry.rs        # Session tracking and persistence
├── .shards/               # Local worktrees directory (created at runtime)
│   └── <shard-name>/      # Individual shard worktrees
├── .kiro/                 # Kiro CLI configuration and steering docs
│   └── steering/          # Project steering documentation
├── target/                # Cargo build artifacts
├── Cargo.toml             # Rust project configuration
├── Cargo.lock             # Dependency lock file
└── README.md              # Project documentation
```

## File Naming Conventions
- **Rust modules**: Snake case (e.g., `git.rs`, `terminal.rs`, `registry.rs`)
- **Shard names**: User-defined, alphanumeric with hyphens/underscores
- **Branch names**: `shard_<uuid>` format for automatic branch creation
- **Registry file**: `~/.shards/registry.json` for global session tracking

## Module Organization
- **main.rs**: CLI argument parsing, command dispatch, and user interface
- **git.rs**: Git repository operations, worktree creation/cleanup, branch management
- **terminal.rs**: Platform-specific terminal launching with conditional compilation
- **registry.rs**: Session persistence, JSON serialization, and lifecycle tracking
- Each module has a single responsibility and minimal external dependencies

## Configuration Files
- **Cargo.toml**: Rust project dependencies and metadata
- **~/.shards/registry.json**: Global session registry (created at runtime)
- **.gitignore**: Excludes build artifacts and local worktrees
- No additional configuration files required for basic operation

## Documentation Structure
- **README.md**: User-facing documentation with usage examples
- **.kiro/steering/**: Project steering documentation
  - `product.md`: Product requirements and objectives
  - `tech.md`: Technical architecture and implementation details
  - `structure.md`: This file - project organization
- Inline code documentation using Rust doc comments

## Asset Organization
- No static assets required for CLI tool
- Terminal integration uses system-native terminal emulators
- Future GUI assets will be organized in separate directories when implemented

## Build Artifacts
- **target/**: Cargo build output directory
  - `target/debug/`: Development builds
  - `target/release/`: Optimized release builds
- **Cargo.lock**: Dependency resolution lock file
- Build artifacts are excluded from version control

## Environment-Specific Files
- Single binary works across all environments (dev/staging/prod)
- Platform-specific code handled through conditional compilation
- No environment-specific configuration files
- Session registry location consistent across environments (`~/.shards/`)
