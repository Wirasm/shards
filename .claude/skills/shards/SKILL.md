---
name: shards
description: Manage parallel AI development sessions in isolated Git worktrees. Use when creating isolated workspaces, managing multiple AI agents, checking session status, or cleaning up development environments.
allowed-tools: Bash(shards:*)
---

# Shards CLI - Parallel AI Development Manager

Shards creates isolated Git worktrees for parallel AI development sessions. Each shard runs in its own workspace with dedicated port ranges and process tracking.

## Core Commands

### Create a Shard
```bash
shards create <branch> [--agent <agent>] [--flags <flags>] [--terminal <terminal>] [--startup-command <command>]
```

Creates an isolated workspace with:
- New Git worktree in `~/.shards/worktrees/<project>/<branch>/`
- Unique port range (10 ports, starting from 3000)
- Native terminal with AI agent launched
- Process tracking (PID, name, start time)
- Session metadata saved to `~/.shards/sessions/`

**Supported agents**: claude, kiro, gemini, codex, aether
**Supported terminal types**: ghostty, iterm, terminal, native

**Note**: `--flags` accepts space-separated syntax: `--flags '--trust-all-tools'`

**Example**:
```bash
shards create feature-auth --agent kiro --terminal ghostty
shards create bug-fix-123 --agent claude --flags '--trust-all-tools'
```

### List All Shards
```bash
shards list
```

Shows table with branch, agent, status, timestamps, port range, process status, and command.

### Restart a Shard
```bash
shards restart <branch> [--agent <agent>]
```

Restarts agent process without destroying worktree. Preserves uncommitted changes.

### Status (Detailed View)
```bash
shards status <branch>
```

Shows detailed info for a specific shard including worktree path, process metadata, port allocation.

### Health Monitoring
```bash
shards health [branch] [--json] [--watch] [--interval <seconds>]
```

Shows health dashboard with process status, CPU/memory metrics, and summary statistics.

### Destroy a Shard
```bash
shards destroy <branch>
```

Completely removes a shard: closes terminal, kills process, removes worktree and branch, deletes session.

### Cleanup Orphaned Resources
```bash
shards cleanup [--all] [--orphans] [--no-pid] [--stopped] [--older-than <days>]
```

Cleans up resources that got out of sync (crashes, manual deletions, etc.).

**Flags**:
- `--all`: Clean all orphaned resources (default)
- `--orphans`: Clean worktrees with no matching session
- `--no-pid`: Clean sessions without PID tracking
- `--stopped`: Clean sessions with dead processes
- `--older-than <days>`: Clean sessions older than N days

## Configuration

Hierarchical TOML config (later overrides earlier):
1. Hardcoded defaults
2. User config: `~/.shards/config.toml`
3. Project config: `./shards/config.toml`
4. CLI flags

## Key Features

- **Process Tracking**: Captures PID, process name, start time. Validates identity before killing.
- **Port Allocation**: Unique port range per shard (default: 10 ports from base 3000).
- **Session Persistence**: File-based storage in `~/.shards/sessions/`
- **Cross-Platform**: macOS, Linux, Windows with native terminal integration.

## Best Practices

- Use descriptive branch names: `feature-auth`, `bug-fix-123`, `issue-456`
- Always destroy shards when done to clean up resources
- Use `shards cleanup` after crashes or manual deletions

## Additional Resources

- For E2E testing after merges, see [cookbook/e2e-testing.md](cookbook/e2e-testing.md)
