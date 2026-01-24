# Shards User Personas

This document defines the primary users of Shards. Both CLI and UI design decisions should be informed by these personas.

---

## Persona 1: Power User (Human)

**Who**: Agentic-forward engineers who run multiple AI agents simultaneously and need clean environment separation.

**Context**:
- Solo developer managing parallel AI workflows
- Works across multiple repositories
- Comfortable with terminal, git, and CLI tools
- Values speed and control over hand-holding
- Knows what they're doing - doesn't need warnings for obvious actions

**Goals**:
- Spin up isolated workspaces quickly
- See all active shards at a glance
- Switch between shards without context-switching friction
- Clean up shards when done

**Anti-goals**:
- Doesn't want confirmation dialogs for routine operations
- Doesn't want "are you sure?" prompts that slow them down
- Doesn't need tutorials or onboarding flows

**Tool usage**:
- **CLI**: Primary interface for scripting, quick one-off shards, headless/CI workflows
- **UI**: Dashboard for visual overview, managing many shards, favorites

**Design implications**:
- Trust the user - surface errors, don't prevent actions
- Fast paths for common operations
- Keyboard-first UI (vim-inspired shortcuts)
- No unnecessary friction or confirmation steps

---

## Persona 2: Agent (AI)

**Who**: AI agents (Claude, Kiro, Codex, etc.) running inside shards that use the CLI to orchestrate work.

**Context**:
- Runs inside a terminal in a shard
- Can execute shell commands including `shards` CLI
- May need to spawn helper shards for subtasks
- May need to stop itself or other shards when work is complete
- Operates programmatically - no interactive prompts

**Goals**:
- Spawn new shards for parallel work: `shards create feature-x --agent claude`
- Open additional agents in existing shards: `shards open feature-x --agent kiro`
- Stop agents when work is complete: `shards stop feature-x`
- Query shard status: `shards list --json`
- Clean up when done: `shards destroy feature-x`

**Anti-goals**:
- Cannot respond to interactive prompts (y/n confirmations)
- Cannot use the UI
- Should not need `--force` flags for normal operations

**Tool usage**:
- **CLI only** - agents don't use the UI
- Needs machine-readable output (`--json` flag)
- Needs non-interactive mode by default

**Design implications**:
- CLI must work without TTY/interactive prompts
- Provide `--json` output for programmatic parsing
- Exit codes should be meaningful and documented
- Error messages should be parseable
- Default behavior should be non-destructive (so agents can recover from mistakes)

---

## How Personas Inform Design

| Decision | Power User Need | Agent Need | Resolution |
|----------|-----------------|------------|------------|
| Confirmations | Annoying friction | Cannot respond | Skip confirmations; use `--force` only for truly dangerous operations |
| Output format | Human-readable default | Machine-readable | Default human-readable, `--json` flag for agents |
| Error handling | Clear error message | Parseable error + exit code | Both: clear message AND proper exit code |
| Interactive prompts | Acceptable rarely | Never | Avoid interactive prompts; use flags instead |
| Default behavior | Do what I mean | Safe/recoverable | Non-destructive defaults; explicit flags for destructive actions |

---

## CLI Command Design Checklist

When designing CLI commands, verify against both personas:

- [ ] Works without TTY (agent can use it)
- [ ] Has `--json` output option (agent can parse it)
- [ ] Has meaningful exit codes (agent can check success/failure)
- [ ] No interactive prompts in default path (agent won't hang)
- [ ] Clear error messages (human can understand)
- [ ] Fast execution (human won't wait)
- [ ] Minimal required flags (human won't type extra)

---

*This document should be referenced when designing new CLI commands or UI features.*
