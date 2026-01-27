# Shards CLI - AI Agent Instructions

## What is Shards?

Shards is a CLI tool that manages multiple AI coding agents in isolated Git worktrees. Think of it as "browser tabs for AI agents" - each shard runs in its own workspace without interfering with others.

## When to Use Shards

### ✅ **Perfect Use Cases**
- **Parallel development**: Work on multiple features/bugs simultaneously with different AI agents
- **Context isolation**: Prevent agents from conflicting over the same working directory
- **Background tasks**: Let an AI agent work on a side issue while you focus on something else
- **Agent collaboration**: Multiple AI agents working on different parts of the same project
- **Experimentation**: Try different approaches in isolated environments

### ❌ **Not Suitable For**
- Single-threaded development (just use your main branch)
- Non-Git projects (Shards requires a Git repository)
- Projects where you need agents to share the same working directory

## Supported Agents

- **amp** - Amp CLI
- **claude** - Claude Code CLI (default)
- **kiro** - Kiro CLI
- **gemini** - Gemini CLI
- **codex** - OpenAI Codex CLI

## How to Use Shards

### **Basic Workflow**

1. **Create a new shard** (creates worktree + launches agent):
   ```bash
   shards create <branch> --agent <agent>
   ```

2. **Check what's running**:
   ```bash
   shards list
   ```

3. **Check health of all shards**:
   ```bash
   shards health
   ```

4. **Remove shard when done**:
   ```bash
   shards destroy <branch>
   ```

### **All Commands**

```bash
# Create shard with specific agent
shards create feature-auth --agent claude

# List all active shards
shards list

# Get detailed status of a shard
shards status feature-auth

# Restart an agent in existing worktree
shards restart feature-auth
shards restart feature-auth --agent kiro  # Switch agent

# Health monitoring
shards health                    # All shards
shards health feature-auth       # Single shard
shards health --watch            # Live dashboard
shards health --json             # JSON output

# Clean up orphaned resources
shards cleanup
shards cleanup --orphans         # Only orphaned worktrees/branches
shards cleanup --stopped         # Only stopped sessions
shards cleanup --older-than 7    # Sessions older than N days

# Destroy shard (kills process, removes worktree)
shards destroy feature-auth
```

## What Happens When You Create a Shard

1. **Git worktree created** in `~/.shards/worktrees/<project>/<branch>/`
2. **New branch created** (or uses existing branch if specified)
3. **Port range allocated** (10 ports per session, starting from 3000)
4. **Agent launched** in terminal window (Ghostty > iTerm > Terminal.app)
5. **Session persisted** to `~/.shards/sessions/<session-id>.json`

## AI Agent Integration

### **For AI Agents Reading This**

You can use Shards to create isolated workspaces for yourself:

```bash
# Create a new workspace for a specific task
shards create my-task --agent kiro

# This will:
# - Create a new Git worktree
# - Launch a terminal with the agent
# - Track the session with process info
# - Allocate dedicated port range
```

### **Agent-to-Agent Workflow**

```bash
# Agent A creates workspace for Agent B
shards create claude-review --agent claude

# Agent B can later check what's running
shards list

# Check health status
shards health

# Agent A can clean up when done
shards destroy claude-review
```

## Best Practices

### **Naming Conventions**
- Use descriptive shard names: `bug-fix-auth`, `feature-payments`, `refactor-db`
- Include issue numbers: `issue-123`, `ticket-456`
- Use agent prefixes: `kiro-debugging`, `claude-testing`

### **Lifecycle Management**
- Always `shards destroy <branch>` when done to clean up worktrees
- Use `shards list` to see what's currently active
- Use `shards health` to monitor agent status
- Use `shards cleanup` to remove orphaned resources

### **Configuration**
Shards supports hierarchical configuration (highest priority wins):
1. CLI arguments
2. Project config: `.shards/config.toml`
3. User config: `~/.shards/config.toml`
4. Defaults

Example config:
```toml
[agent]
default = "claude"

[terminal]
preferred = "ghostty"

[agents.claude]
startup_command = "claude"
flags = "--dangerously-skip-permissions"
```

## Troubleshooting

### **Common Issues**
- **"Not in a Git repository"**: Run shards from within a Git project
- **"Shard already exists"**: Use a different name or destroy the existing shard first
- **Terminal doesn't open**: Check if your terminal emulator is supported (Ghostty, iTerm, Terminal.app on macOS)

### **Recovery Commands**
```bash
# Check what's running
shards list

# Check health status
shards health

# Clean up orphaned resources
shards cleanup --orphans

# Force destroy a stuck shard
shards destroy <branch-name>
```

## Requirements

- Must be run from within a Git repository
- macOS: Ghostty (preferred), iTerm, or Terminal.app
- Supported agents must be installed and in PATH

---

**Remember**: Shards is designed for parallel AI development. Use it when you need multiple agents working simultaneously in isolated environments!
