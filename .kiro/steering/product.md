# Product Overview

## Product Purpose
Shards is a CLI tool that manages multiple AI coding agents running in isolated Git worktrees. It eliminates context switching between scattered terminals by providing centralized management of parallel AI development sessions.

## Target Users
Power users and agentic-forward engineers who want speed, control, and isolation. Users who run multiple AI agents simultaneously and need clean environment separation. Designed as a single-developer tool with no multi-tenant complexity.

## Supported Agents
- **Claude** - Claude Code CLI (default)
- **Kiro** - Kiro CLI
- **Gemini** - Gemini CLI
- **Codex** - OpenAI Codex CLI
- **Aether** - Aether CLI

## Key Features
- **Isolated Worktrees**: Each shard runs in its own Git worktree with user-specified or auto-generated branch names
- **Native Terminal Integration**: Launches AI agents in native terminal windows (Ghostty > iTerm > Terminal.app on macOS)
- **Session Persistence**: Track active shards with JSON-based session files in `~/.shards/sessions/`
- **Port Range Allocation**: Each session gets dedicated port range (10 ports) to avoid conflicts
- **Health Monitoring**: CPU/memory usage tracking, status detection (working/idle/stuck/crashed)
- **Lifecycle Management**: Create, list, status, restart, destroy, cleanup commands
- **Hierarchical Configuration**: CLI args → project config → user config → defaults

## Business Objectives
- Reduce context switching overhead when working with multiple AI agents
- Enable parallel AI development workflows without terminal management complexity
- Provide centralized dashboard for AI agent session management
- Support agent-driven workflows where AI assistants can spawn their own shards

## User Journey
1. Developer starts working on a project with an AI agent
2. AI agent or developer runs `shards create <branch> --agent <agent>` to create isolated workspace
3. New Git worktree is created, port range allocated, agent launches in terminal
4. Developer can continue working while agent operates in background
5. Use `shards list` or `shards health` to see all active sessions
6. Use `shards restart <branch>` to restart a stopped agent
7. Clean up with `shards destroy <branch>` when done
8. Use `shards cleanup` to remove orphaned resources

## Success Criteria
- Seamless creation and management of isolated AI agent sessions
- Zero context switching between different AI development tasks
- Reliable worktree and session lifecycle management
- Agent-friendly CLI interface for programmatic usage
- No silent failures - explicit error reporting
