# KILD Daemon Epic

*February 2026*

KILD today is a production CLI + GPUI dashboard managing parallel AI agents in external terminal windows. The daemon vision transforms KILD into the runtime itself — owning PTYs, embedding terminals, replacing tmux. This epic tracks that transformation.

**Research:** [daemon-vision-research.md](../branding/daemon-vision-research.md) — full technical research, architecture decisions, competitive analysis, and open questions.

**Brand/UX:** [mockup-embedded.html](../branding/mockup-embedded.html) — the target multiplexer UI. [brand-system.html](../branding/brand-system.html) — Tallinn Night design system.

**Vision:** [VISION.md](../branding/VISION.md) — mission, market positioning, expansion path.

---

## Where We Are

| Crate | Lines | State |
|-------|-------|-------|
| kild-core | 28k | Production. Sessions, git worktrees, terminal backends, config, forge, health, daemon client. |
| kild (CLI) | 6k | Production. 20+ commands, thin delegation to kild-core. Daemon subcommands + attach. |
| kild-daemon | 4k | Experimental. PTY ownership via portable-pty, JSONL IPC, session state machine, async tokio server. |
| kild-ui | 9k | Working. GPUI 0.2.2 (crates.io), 3-column metadata dashboard, embedded terminal rendering via alacritty_terminal + portable-pty. |
| kild-peek/core | 5k | Production. macOS window inspection, UI automation. |

**What works:** Git worktree isolation, session lifecycle, multi-agent tracking, PR integration, fleet health, cross-kild overlap detection, config hierarchy, daemon PTY ownership, daemon IPC, daemon-mode session create/stop/destroy, terminal attach with detach.

**What's missing:** Terminal resize handling, tmux shim, agent-aware rendering, multiplexer layout. Daemon gaps: auto-start, scrollback replay on attach, background mode, PTY exit notification.

---

## The Phases

### Phase 0: GPUI Foundation --- DONE (PR #293)

**Goal:** Adopt gpui-component library, replacing hand-built UI components.

**What shipped:**
- [x] Added `gpui-component 0.5.1` from crates.io (works with existing `gpui 0.2.2`)
- [x] Migrated Button → `gpui_component::button::Button` (~26 call sites, 7 files)
- [x] Migrated TextInput → `gpui_component::input::{Input, InputState}` (stateful, native keyboard)
- [x] Migrated Modal → inlined dialog rendering with `cx.theme()` integration
- [x] StatusIndicator kept as custom component (no equivalent for glow effects)
- [x] Tallinn Night theme bridge maps brand colors to gpui-component tokens
- [x] 447 lines of hand-built component code deleted, +1,197 / -1,067 across 20 files
- [x] Full build passes: fmt, clippy, tests, build

**Note:** GPUI source stays on crates.io 0.2.2 for now. Switch to Zed git repo deferred to Phase 1a when terminal rendering APIs are needed.

**Research artifacts:** `.claude/PRPs/plans/phase-0-research/` (3 research docs from agent team)

---

### Phase 1a: Terminal Rendering Spike --- DONE (PR #300)

**Goal:** Render a live terminal session inside kild-ui using GPUI. Prove the hardest technical piece.

**Why:** This is the core technical risk. If we can't render terminals performantly in GPUI, the entire daemon vision needs a different UI strategy. Everything else is software engineering — this is the unknown.

**What shipped:**
- [x] Added `alacritty_terminal` dependency (VT100 parsing + terminal grid state)
- [x] Added `portable-pty` for PTY management (already a dependency via kild-daemon; proven in Phase 1b)
- [x] Built `TerminalElement` — custom GPUI Element (request_layout → prepaint → paint) that reads `alacritty_terminal::Term` grid and paints via scene primitives (quads for backgrounds, shaped text runs for glyphs)
- [x] ANSI color mapping to Tallinn Night theme — 16 named colors (standard + bright + dim), 256-color indexed (6×6×6 cube + 24-step grayscale), truecolor passthrough
- [x] Keyboard input → PTY stdin via escape sequence translation with app cursor mode support (vim/tmux)
- [x] Ctrl+T toggles between 3-column kild layout and full-area terminal view
- [x] 4ms event batching (250Hz max refresh, 100-event cap) to prevent UI freeze on large output
- [x] FairMutex between PTY reader thread and GPUI renderer to prevent lock starvation
- [x] Structured `TerminalError` with `error_code()` / `is_user_error()` methods, 7 typed variants
- [x] Graceful error handling — PTY creation failure shows UI error banner instead of crashing
- [x] All silent failure sites addressed with structured tracing (zero `let _ =` on fallible ops)
- [x] Encapsulated `BatchedTextRun` type with private fields and validated constructor
- [x] 27 new unit tests for color mapping and input translation
- [x] 1,410 lines of new terminal module code, +1,639 / -17 across 16 files

**New files in `crates/kild-ui/src/terminal/`:**
- `state.rs` — Terminal struct wrapping `Term<T>` + FairMutex + PTY lifecycle
- `terminal_element.rs` — Custom 3-phase Element with text run batching and background region merging
- `terminal_view.rs` — GPUI View with focus management and keyboard routing
- `colors.rs` — ANSI → Tallinn Night Hsla color mapping (Named, Indexed, Spec)
- `input.rs` — Keystroke → escape sequence translation
- `errors.rs` — Structured TerminalError with 7 variants
- `types.rs` — Encapsulated BatchedTextRun for rendering pipeline
- `mod.rs` — Module exports

**Decisions made:**
- **VT100 library:** `alacritty_terminal` — battle-tested, used by Alacritty and Zed, provides grid state + event system
- **Threading model:** Blocking PTY reader on dedicated OS thread (not GPUI BackgroundExecutor) because blocking reads would starve the cooperative async pool
- **Lock strategy:** `FairMutex` from alacritty_terminal prevents renderer from starving PTY reader (or vice versa)
- **Batching:** 4ms window (250Hz) with 100-event cap — balances responsiveness vs. render overhead
- **GPUI version:** Stayed on crates.io 0.2.2 — terminal rendering works with published API, no need for git-sourced GPUI

**Known gaps (deferred):**
- [ ] **Repaint signaling (input lag root cause):** The event batching task runs on `background_executor` and has no entity handle to call `cx.notify()` after processing PTY output. GPUI only repaints when something else triggers a frame (next keystroke). Fix: move batching task to `cx.spawn()` in `TerminalView` so it gets a `this` handle, call `this.update(cx, |_, cx| cx.notify())` after each batch. This is the single biggest UX issue.
- [ ] Terminal resize handling (SIGWINCH on PTY when element bounds change) — not implemented, terminal uses fixed 80×24
- [ ] Cursor blink animation — cursor renders as static block (focused) or thin bar (unfocused), no blink timer
- [ ] Selection and copy/paste — no mouse selection or clipboard integration
- [ ] Scrollback — alacritty_terminal supports it but `TermDimensions::total_lines == screen_lines` (no scrollback configured)
- [ ] Test coverage for PTY creation, event batching, and rendering edge cases — existing 27 tests cover color/input; stateful/async code deferred (requires complex mocking)
- [ ] Wide character rendering — spacer cells are skipped but no explicit width-2 glyph handling
- [ ] URL detection / clickable links

**Key reference:** Zed's terminal implementation uses the same pattern — `TerminalElement` with text run batching and background region merging for performance. Studied but not copied; written for KILD's needs.

**Signals of completion:** A shell session renders in kild-ui. You can type commands, see output, ANSI colors work, cursor tracks position. Ctrl+T toggles between dashboard and terminal.

**Scope:** ~1,500 lines of new code as estimated. The rendering pipeline was the hard part — proved feasible.

---

### Phase 1b: KILD Daemon --- DONE (PR #294)

**Goal:** KILD daemon owns PTYs and persists sessions. Agents survive terminal disconnects.

**Why:** This is the core architectural shift. Today, closing a terminal kills the agent. With the daemon, agents run in daemon-owned PTYs — close your laptop, reopen, reconnect. The daemon is the foundation that Phase 2 (multiplexer UI), Phase 3 (intelligence), and everything after builds on.

**What shipped:**
- [x] New crate: `kild-daemon` (~4k lines) — tokio-based async daemon
  - `kild daemon start` — starts daemon (foreground), writes PID file, listens on `~/.kild/daemon.sock`
  - `kild daemon stop` — graceful shutdown via IPC
  - `kild daemon status` — show daemon running state
  - JSONL IPC server over unix socket with per-connection handlers
  - PTY lifecycle management via `portable-pty` (create, resize, destroy)
  - Session state machine (Creating → Running → Stopped) with multi-client attachment
  - Signal handling (SIGTERM/SIGINT) and graceful shutdown
  - PTY output broadcasting with scrollback ring buffer
  - PID file management
  - Integration tests covering protocol codec and session lifecycle
- [x] `kild create` gains `--daemon` / `--no-daemon` flags with `resolve_runtime_mode()` helper
- [x] `[daemon] enabled` and `[daemon] auto_start` config fields (`DaemonRuntimeConfig`)
- [x] `RuntimeMode` enum in kild-core state types, `daemon_session_id` on `AgentProcess`
- [x] `kild attach <branch>` — raw terminal mode, stdin forwarding, PTY output streaming, Ctrl+B d detach
- [x] `kild stop` / `kild destroy` work with daemon sessions (IPC to daemon instead of PID kill)
- [x] Sync daemon client in kild-core (`daemon::client`) for CLI consumers
- [x] Typed `DaemonClient` in kild-daemon with connection pooling and all operations
- [x] CLI without daemon continues to work exactly as today (Ghostty, iTerm, etc.)
- [x] +6,029 / -115 across 47 files

**Decisions made:**
- **PTY library:** `portable-pty` (cross-platform, used by Wezterm, flexible for daemon ownership)
- **Daemon lifecycle:** `kild daemon start` / `kild daemon stop` / `kild daemon status` subcommands
- **Default mode:** `[daemon] enabled` in config.toml + `--daemon` / `--no-daemon` flags override per-command
- **Socket path:** `~/.kild/daemon.sock` (user-global, one daemon per user)
- **IPC protocol:** Unix socket + JSONL with message ID correlation
- **Attach modes:** Terminal (`kild attach`) with Ctrl+B d detach. GUI attach deferred to Phase 2.

**Known gaps (follow-up work):**
- [ ] Ping message not yet handled by server
- [ ] No full client-server integration test (unit + protocol tests exist)
- [ ] Auto-start daemon on `kild create --daemon` when daemon not running
- [ ] Scrollback replay on attach (ring buffer exists, replay not wired)
- [ ] PTY exit notification to session manager (process exits not propagated)
- [ ] Daemon background mode (`kild daemon start` runs foreground only; no daemonization)
- [ ] `kild ui <branch>` — open kild-ui focused on a daemon session (deferred to Phase 2)

**Research artifacts:** `.claude/PRPs/plans/phase-1b-integration-plan.md`

---

### Phase 1c: tmux Shim (Experimental)

**Goal:** Make Claude Code agent teams work transparently inside KILD sessions via a tmux-compatible shim.

**Why:** Claude Code agent teams (experimental, Feb 2026) use tmux for split-pane mode — they check `$TMUX` and issue tmux CLI commands to create panes for teammates. A shim that intercepts these commands and routes them to the KILD daemon would make agent teams work inside KILD without any Anthropic cooperation.

**This is experimental.** Agent teams are themselves experimental. The tmux commands Claude Code uses may change. This phase is about trial-and-error discovery, not building to a spec.

**Approach:**
- [ ] Audit Claude Code's actual tmux usage (inspect npm source, observe real commands during team sessions)
- [ ] Build minimal `kild-tmux-shim` binary that handles observed commands
- [ ] Test with real agent team sessions, discover what's missing, iterate
- [ ] Expand shim coverage as needed based on what breaks

**What the shim does:**
- Binary on PATH that intercepts tmux CLI commands
- Routes them to KILD daemon IPC (depends on Phase 1b daemon being functional)
- Sets `$TMUX` env var so Claude Code detects "tmux" and activates split-pane mode
- KILD sees each teammate as a pane within the kild's session

**Signals of completion:** You can `kild create foo --daemon`, run Claude Code inside it, tell it to create an agent team, and the teammates spawn as daemon-managed PTYs visible in `kild list`.

**Scope:** Medium. Estimated ~500 lines for the shim binary, but scope depends on what tmux commands actually need supporting. Discovery-driven.

**Dependency:** Requires Phase 1b daemon with working IPC.

---

### Phase 2: Multiplexer UX

**Goal:** kild-ui becomes the full multiplexer from the mockup — project rail, kild sidebar, terminal pane grid, teammate tabs, minimized sessions.

**Why:** This is what users see. The daemon (Phase 1b) gives us persistence and PTY ownership. This phase gives us the UX that makes it feel like a product, not a tech demo.

**Deliverables:**
- [ ] Project rail (48px icon column) — project switching, add project, settings
- [ ] Kild sidebar with dual mode — list view (grouped by status, agent trees) + detail view (click to inspect: note, session info, git stats, PR, path)
- [ ] Terminal pane grid — split panes with resize handles, thin pane headers
- [ ] Teammate tab bar — status dots per teammate, tab switching
- [ ] Minimized kild bars — collapsed single-line at bottom, expand on click
- [ ] Keyboard shortcut system — per-terminal focus routing, global shortcuts for navigation
- [ ] Status bar — notifications, active shortcut hints
- [ ] Hover-reveal micro-actions — copy path, open PR URL

**Key reference:** [mockup-embedded.html](../branding/mockup-embedded.html) is the definitive mockup. Every interaction pattern is documented there.

**Signals of completion:** The mockup is real. You can switch projects, browse kilds, see terminal output in split panes, minimize sessions, inspect kild details.

**Scope:** High. Full view rewrite of kild-ui. Leverage gpui-component (Sidebar, Tabs, Dock, Resizable) heavily.

---

### Phase 3: Agent Intelligence

**Goal:** KILD understands what agents are doing and acts on that knowledge. Checkpointing, notifications, cross-session awareness.

**Why:** This is where KILD becomes more than a terminal multiplexer. tmux shows you terminal output. KILD shows you agent state — thinking, editing, stuck, idle, waiting for permission. This is the differentiator.

**Deliverables:**
- [ ] Agent state tracking — hook integration + file watching for real-time agent state (thinking/editing/stuck/idle/done)
- [ ] Notification system — contextual alerts when agents need attention, finish tasks, hit errors
- [ ] Checkpoint engine — periodic snapshots of session state (git SHA + scrollback + agent context)
- [ ] Restore from checkpoint — recover after daemon crash or manual rollback
- [ ] Cross-session conflict detection — continuous `collect_file_overlaps()` (foundation exists in `kild overlaps`)
- [ ] Agent-aware rendering — status indicators, thinking animation, tool call cards inline with terminal output (stretch)

**Signals of completion:** You see "agent thinking..." in the UI. You get notified when an agent finishes. You can checkpoint and restore a session.

**Scope:** Medium-High. Builds on Phase 1b daemon infrastructure.

---

### Phase 4: Session Orchestration

**Goal:** Session forking, session graphs (DAG), templates, fleet command center.

**Why:** This is the "six innovations" layer from the research doc. No other tool has session forking or dependency graphs between sessions. This is where KILD becomes categorically different.

**Deliverables:**
- [ ] Session forking — try parallel approaches from same checkpoint
- [ ] Session graph (DAG) — dependencies between sessions, auto-start downstream when upstream completes
- [ ] Templates — reusable workflow definitions in TOML
- [ ] Fleet dashboard — merge readiness overview, session graph visualization, bulk operations
- [ ] Tool call rendering — syntax-highlighted cards inline with terminal (if not done in Phase 3)

**Signals of completion:** You can fork a session, try two approaches, pick the winner. You can define "when auth finishes, start api-integration automatically."

**Scope:** High. Significant new concepts and UI.

---

### Phase 5: Portability & Remote

**Goal:** Sessions move between machines. Remote daemon access. Mobile monitoring.

**Why:** The Tōryō-from-anywhere vision. Start locally, continue on cloud. Monitor from your phone.

**Deliverables:**
- [ ] Session serialization — export/import between machines
- [ ] IPC over TCP — remote daemon access (upgrade unix socket to TCP/TLS)
- [ ] Remote kild-ui — connect to daemon on another machine
- [ ] Mobile client (read-only) — status monitoring, approve/deny decisions
- [ ] Cloud deployment — daemon on VPS, agents in containers

**Signals of completion:** You deploy a daemon on a VPS. Connect kild-ui from your laptop. Agents run in the cloud. You check in from your phone.

**Scope:** Very high. Network protocol, auth, mobile client.

---

## Dependencies Between Phases

```
Phase 0 (DONE) ──→ Phase 1a (DONE) ──→ Phase 2
                                           ↑
Phase 1b (DONE) ──→ Phase 1c (experimental)
       ↓
  Phase 3 ──→ Phase 4 ──→ Phase 5
```

- **Phase 0** (DONE) — gpui-component adopted
- **Phase 1a** (DONE) — terminal rendering in GPUI via alacritty_terminal + portable-pty
- **Phase 1b** (DONE) — daemon crate with PTY ownership, IPC, session state machine
- **Phase 1c** (tmux shim) is unblocked — Phase 1b daemon IPC is functional, can be picked up anytime
- **Phase 2** (multiplexer UX) is unblocked — both Phase 1a (terminal panes) and Phase 1b (daemon) are done. This is the next critical path.
- **Phase 3** (intelligence) needs Phase 1b (DONE) daemon for hooks and state tracking
- **Phase 4** and **Phase 5** are sequential and build on everything before them

## What Stays the Same

The CLI and external terminal workflow is **not going away**. The daemon is a parallel runtime option. Power users who prefer `kild create foo` → Ghostty window keep that workflow forever.

Stable through all phases:
- Terminal backends (`terminal/` module) — Ghostty, iTerm, Terminal.app, Alacritty
- Session handlers (`sessions/`) — create, open, stop, destroy, complete
- Git worktree operations (`git/` module)
- Config hierarchy (`config/` module)
- Forge/PR operations (`forge/` module)
- Health monitoring (`health/` module)
- Cleanup strategies (`cleanup/` module)
- File inclusion patterns (`files/` module)
- Agent backend definitions (`agents/` module)
- CLI stays fully functional — daemon is additive

## What Gets Added (Not Replaced)

The daemon is **additive**. External terminal backends (Ghostty, iTerm, Terminal.app, Alacritty) stay fully functional for CLI users. The daemon is an optional runtime — you can still `kild create foo` and get an agent in a Ghostty window.

- New `kild-daemon` crate — PTY ownership, IPC server, session persistence (Phase 1b)
- New `kild-tmux-shim` crate — tmux command translation for agent teams (Phase 1c, experimental)
- New terminal rendering module in kild-ui — GPUI Element for embedded terminals (Phase 1a)
- kild-ui views — rewrite for multiplexer layout (Phase 2)
- Hand-built UI components — replaced by gpui-component (Phase 0, DONE)

## Key Technical Decisions

### Decided
- **Daemon lifecycle:** `kild daemon start` / `kild daemon stop` subcommands
- **Daemon scope:** One daemon per user, manages all projects
- **Default mode:** Config `[daemon] enabled = true` + `--daemon` / `--no-daemon` flags
- **Socket path:** `~/.kild/daemon.sock`
- **IPC protocol:** Unix socket + JSONL. gRPC deferred to Phase 5 (remote access).
- **Attach modes:** `kild attach foo` (terminal) + `kild ui foo` (GUI)
- **Additive architecture:** External terminal backends stay. Daemon is opt-in.
- **Multiplexer UI:** GPUI (native desktop), not ratatui TUI. Primary reasons: 7,500 lines of existing kild-ui code, GPU-accelerated terminal rendering, mixed content (terminals + dashboards + notifications), mouse/font support. A lightweight ratatui TUI client may be added later as a secondary interface (e.g., for SSH access), but is not on the roadmap.

### Resolved
- **PTY library:** `portable-pty` — decided and shipped in Phase 1b (PR #294). Cross-platform, used by Wezterm, flexible for daemon ownership.

### Resolved (Phase 1a)
- **GPUI version:** Staying on crates.io 0.2.2. Terminal rendering works with published API — no need for git-sourced GPUI. Revisit only if Phase 2 multiplexer layout needs APIs not in 0.2.2.

### Still Open
1. **tmux shim scope:** Discovery-driven in Phase 1c. Audit Claude Code's actual usage rather than assuming commands upfront.
2. **Terminal resize:** Phase 1a deferred SIGWINCH handling. Phase 2 multiplexer will need dynamic resize as panes change size.

---

*This epic is a living document. Each phase gets its own detailed plan when we start it.*
