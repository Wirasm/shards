# KILD — Vision & Mission

*Draft: January 2026*

---

## Mission

**Make parallel AI development accessible to everyone.**

Today, only senior engineers successfully manage multiple AI agents working in parallel. The cognitive overhead of coordinating agents, understanding git worktrees, managing terminal sessions, and preventing conflicts creates an expertise barrier that locks out most developers.

KILD removes that barrier. We provide the isolation, visibility, and control that makes parallel agent workflows feel as natural as managing browser tabs.

---

## Vision

**A world where developers direct fleets of AI agents as naturally as they manage browser tabs today.**

The future of software development isn't one human and one AI assistant. It's one human directing many AI agents — each working on a different task, in a different isolated environment, all visible at a glance.

We're building the control layer for that future.

---

## The Problem We Solve

### The Fog

When you run multiple AI agents simultaneously, you lose track:

- "Which terminal was the payment refactor?"
- "Did feature-auth finish or is it waiting for me?"
- "Something failed 20 minutes ago. Where?"
- "I have 6 tasks running. How many are actually progressing?"

This is **the fog** — the cognitive fragmentation that comes from parallel work without parallel visibility.

The fog is the enemy. KILD lifts the fog.

### The Expertise Barrier

Git worktrees are the standard isolation mechanism for parallel agents. Cursor, Windsurf, Conductor — they all use worktrees under the hood.

But worktrees are a power-user feature. Most developers have never used them. The mental model is unfamiliar. The commands are obscure. The result: parallel agent workflows remain inaccessible to most developers.

KILD abstracts the complexity. You don't need to understand worktrees. You just need to say "create a kild for this task" and start working.

---

## Strategic Positioning

### What KILD Is

- **An isolation layer** — Each agent gets its own universe. They can't step on each other.
- **A visibility layer** — See all your agents at a glance. Know what's running, stuck, or waiting.
- **A control layer** — Create, stop, destroy, focus. Manage your fleet with simple commands.
- **Agent-agnostic** — Works with Claude, Cursor, Kiro, Gemini, Codex, or any future agent.
- **IDE-agnostic** — Not locked to one editor. Works with your existing tools.

### What KILD Is Not

- Not an AI agent framework (we manage where agents run, not how they think)
- Not a replacement for git (git does the branching; we wrap it)
- Not an orchestration platform (we provide isolation and visibility, not workflow automation)
- Not locked to one ecosystem (works across agents, editors, and terminals)

---

## The Wedge

**Isolation is the foundation.**

You can't have 10 agents without isolation. They'll conflict, overwrite each other's work, and create chaos.

You can't have 100 agents without visibility. You'll lose track, miss failures, and waste compute on stuck tasks.

KILD provides both. That's the wedge.

---

## The Expansion Path

### Phase 1: Power Users (Now)

**The Tōryō** — Senior engineers running 10-30+ agents across multiple projects.

These users already understand the problem. They're already using worktrees manually, managing terminal sessions, losing track of what's running. KILD gives them the tool they've been building themselves with bash scripts and discipline.

**What they need:**
- Fast CLI with `--json` output for scripting
- Named sessions with status tracking
- Health monitoring and focus commands
- Non-interactive, agent-friendly interface

### Phase 2: Broader Developers (Next)

**The Rising Tide** — Developers adopting parallel workflows as agents improve.

As AI agents become more capable, more developers will want to run multiple agents. But they won't learn git worktrees. They need the complexity hidden behind excellent UX.

**What they need:**
- Native GUI that makes parallel agents visual and intuitive
- One-click creation, simple status views
- No terminal required for basic operations
- Gentle learning curve, powerful when needed

### Phase 3: Vibe Coders (Future)

**The New Majority** — Developers who direct agents without deep technical knowledge.

These users don't want to understand isolation, branches, or worktrees. They want to say "build me these 5 features" and have it work. They judge tools by how they feel, not how they're implemented.

**What they need:**
- Delightful UX that hides all complexity
- Smart defaults that just work
- Visual feedback that builds confidence
- The power of parallel agents without the cognitive overhead

---

## Why Now

### The Market Is Moving Fast

- **Cursor 2.0** ships with parallel agents and git worktree support
- **Windsurf Wave 13** launched 5-agent parallel sessions
- **Conductor** built an entire product around worktree-based agent orchestration
- **85% of developers** now regularly use AI tools for coding (up from ~40% in 2023)

The parallel agent pattern is no longer experimental. It's becoming the default workflow for productive teams.

### The Bottleneck Has Shifted

The DORA 2025 report found PR review time increased 91%. Code generation is no longer the bottleneck — review and coordination are.

When you can generate code 10x faster, the limiting factor becomes:
1. How many parallel workstreams can you manage?
2. How quickly can you review and merge results?

KILD addresses #1 directly and makes #2 easier by keeping work isolated and visible.

### The Expertise Barrier Is Real

"So far, only senior engineers are successfully managing parallel agent workflows. Junior developers struggle with the cognitive overhead."

This barrier is an opportunity. Whoever makes parallel agents accessible to everyone captures a massive market as agent capabilities improve.

---

## The Moat

### Brand

"KILD" is distinctive, memorable, and ownable. The mythology (Honryū, Tōryō, the Fog) creates emotional resonance that commodity tools lack.

### Agent-Agnostic

We're not locked to Claude, Cursor, or any single ecosystem. As new agents emerge, KILD works with all of them. The isolation layer is agent-independent.

### UX Focus

Our competition is building worktree support as a feature. We're building worktree management as the product. That focus means better UX, better edge case handling, better polish.

### Community

Apache 2.0 licensing means developers can use, modify, and contribute. The community becomes a moat as adoption grows.

---

## Success Metrics

### Near-term (6 months)
- 1,000+ active CLI users
- GUI in production use
- Recognition in AI developer tooling discussions

### Medium-term (12-18 months)
- 10,000+ active users across CLI and GUI
- Premium features generating revenue
- Partnerships or integrations with major AI tools

### Long-term (24+ months)
- Standard tool for parallel agent workflows
- Acquisition target for major AI/dev tooling company, or
- VC-backed expansion to full agent orchestration platform

---

## The One-Liner

**KILD: Parallel agents. Isolated worlds. One sane developer.**

---

*Fracture the Honryū.*
