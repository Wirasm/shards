# Implementation Matrix

**Purpose**: Cross-PRD dependency tracking and optimal implementation order.
**Last Updated**: 2026-01-24

---

## PRD Overview

| PRD | Focus | Phases | Status |
|-----|-------|--------|--------|
| [CLI Core Features](./cli-core-features.prd.md) | CLI usability & scripting | 3 phases | Phase 1 in progress |
| [GPUI Native Terminal UI](./gpui-native-terminal-ui.prd.md) | Visual dashboard | 10+ phases | Phases 1-6 complete |

---

## Dependency Matrix

Shows which GUI phases depend on CLI features being implemented first.

| GUI Phase | Depends On (CLI) | Can Start After |
|-----------|------------------|-----------------|
| 7: Status Dashboard | None | Now |
| 7.5: Notes & Git Status | CLI 1.1 (`--note`) | CLI 1.1 complete |
| 7.6: Bulk Operations | CLI 2.5 (`open/stop --all`) | CLI 2.5 complete |
| 7.7: Quick Actions | CLI 1.2 (`cd`), 1.3 (`code`), 2.1 (`focus`) | CLI 1.2, 1.3, 2.1 complete |
| 8: Favorites | None | Phase 7.7 complete |
| 9: Theme & Components | None | Phase 8 complete |
| 10: Keyboard Shortcuts | None | Phase 9 complete |

---

## All Tasks (Flattened)

| ID | Task | PRD | Effort | Dependencies |
|----|------|-----|--------|--------------|
| CLI-1.1 | Session notes (`--note`) | CLI | Small | None |
| CLI-1.2 | `shards cd` | CLI | Small | None |
| CLI-1.3 | `shards code` | CLI | Small | None |
| CLI-1.4 | `--json` on list/status | CLI | Small | None |
| CLI-1.5 | `-q`/`--quiet` mode | CLI | Small | None |
| CLI-2.1 | `shards focus` | CLI | Medium | None |
| CLI-2.2 | `shards diff` | CLI | Small | None |
| CLI-2.3 | `shards commits` | CLI | Small | None |
| CLI-2.4 | `destroy --all` | CLI | Small | None |
| CLI-2.5 | `open/stop --all` | CLI | Small | None |
| CLI-2.6 | Fuzzy matching | CLI | Medium | None |
| GUI-7 | Status Dashboard | GUI | Medium | None |
| GUI-7.5 | Notes & Git Status | GUI | Medium | CLI-1.1 |
| GUI-7.6 | Bulk Operations | GUI | Small | CLI-2.5 |
| GUI-7.7 | Quick Actions | GUI | Medium | CLI-1.2, CLI-1.3, CLI-2.1 |
| GUI-8 | Favorites | GUI | Medium | GUI-7.7 |
| GUI-9 | Theme & Components | GUI | Medium | GUI-8 |
| GUI-10 | Keyboard Shortcuts | GUI | Medium | GUI-9 |

---

## Optimal Implementation Order

Based on dependencies, value delivery, and parallel execution opportunities.

### Wave 1: Foundation (Can Run in Parallel)

All CLI Phase 1 features have no dependencies and can be implemented simultaneously.

| Track A (CLI) | Track B (GUI) |
|---------------|---------------|
| CLI-1.1: `--note` | GUI-7: Status Dashboard |
| CLI-1.2: `cd` | |
| CLI-1.3: `code` | |
| CLI-1.4: `--json` | |

**Parallel execution**: Yes - CLI and GUI tracks are independent at this stage.

### Wave 2: Extended CLI + Notes GUI

| Track A (CLI) | Track B (GUI) |
|---------------|---------------|
| CLI-1.5: `--quiet` | GUI-7.5: Notes & Git Status |
| CLI-2.5: `open/stop --all` | (blocked until CLI-1.1 done) |
| CLI-2.1: `focus` | |

**Parallel execution**: Partial - GUI-7.5 must wait for CLI-1.1.

### Wave 3: Bulk Ops + Quick Actions

| Track A (CLI) | Track B (GUI) |
|---------------|---------------|
| CLI-2.2: `diff` | GUI-7.6: Bulk Operations |
| CLI-2.3: `commits` | GUI-7.7: Quick Actions |
| CLI-2.4: `destroy --all` | (blocked until CLI-2.5, 1.2, 1.3, 2.1) |
| CLI-2.6: Fuzzy matching | |

**Parallel execution**: Partial - GUI phases depend on their CLI counterparts.

### Wave 4: Polish

| Track A (CLI) | Track B (GUI) |
|---------------|---------------|
| (Phase 1-2 complete) | GUI-8: Favorites |
| | GUI-9: Theme & Components |
| | GUI-10: Keyboard Shortcuts |

**Parallel execution**: No - GUI phases are sequential here.

---

## Recommended Sprint Plan

### Sprint 1: CLI Foundation + GUI Refresh
**Goal**: Scriptability and live dashboard

| Task | Assignee | Effort |
|------|----------|--------|
| CLI-1.4: `--json` | - | Small |
| CLI-1.1: `--note` | - | Small |
| GUI-7: Status Dashboard | - | Medium |

**Deliverables**:
- `shards list --json` works
- `shards create --note "..."` works
- GUI auto-refreshes every 5 seconds

### Sprint 2: Navigation + Notes GUI
**Goal**: Quick navigation and notes visibility

| Task | Assignee | Effort |
|------|----------|--------|
| CLI-1.2: `cd` | - | Small |
| CLI-1.3: `code` | - | Small |
| CLI-1.5: `--quiet` | - | Small |
| GUI-7.5: Notes & Git Status | - | Medium |

**Deliverables**:
- `shards cd feature-x` prints path
- `shards code feature-x` opens editor
- GUI shows notes in list view
- GUI shows git dirty indicator

### Sprint 3: Bulk Operations
**Goal**: Power user bulk actions

| Task | Assignee | Effort |
|------|----------|--------|
| CLI-2.5: `open/stop --all` | - | Small |
| CLI-2.1: `focus` | - | Medium |
| GUI-7.6: Bulk Operations | - | Small |

**Deliverables**:
- `shards open --all` launches all stopped
- `shards stop --all` stops all running
- `shards focus feature-x` brings terminal to front
- GUI has "Open All" / "Stop All" buttons

### Sprint 4: Quick Actions
**Goal**: Complete per-shard actions

| Task | Assignee | Effort |
|------|----------|--------|
| GUI-7.7: Quick Actions | - | Medium |
| CLI-2.2: `diff` | - | Small |
| CLI-2.3: `commits` | - | Small |

**Deliverables**:
- GUI has Copy Path, Open Editor, Focus buttons
- `shards diff feature-x` shows changes
- `shards commits feature-x` shows history

### Sprint 5: Polish
**Goal**: Visual polish and power user features

| Task | Assignee | Effort |
|------|----------|--------|
| CLI-2.4: `destroy --all` | - | Small |
| CLI-2.6: Fuzzy matching | - | Medium |
| GUI-8: Favorites | - | Medium |

### Sprint 6: Final Polish
**Goal**: Professional finish

| Task | Assignee | Effort |
|------|----------|--------|
| GUI-9: Theme & Components | - | Medium |
| GUI-10: Keyboard Shortcuts | - | Medium |

---

## Visual Dependency Graph

```
                    ┌─────────────────────────────────────────────┐
                    │              IMPLEMENTATION FLOW             │
                    └─────────────────────────────────────────────┘

WAVE 1 (Parallel)
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ CLI: --json │     │ CLI: --note │     │  CLI: cd    │     │ CLI: code   │
└─────────────┘     └──────┬──────┘     └──────┬──────┘     └──────┬──────┘
       │                   │                   │                   │
       │                   │                   │                   │
       ▼                   │                   │                   │
┌─────────────┐            │                   │                   │
│GUI: Status  │            │                   │                   │
│  Dashboard  │            │                   │                   │
└─────────────┘            │                   │                   │
                           ▼                   │                   │
WAVE 2                ┌─────────────┐          │                   │
                      │GUI: Notes & │          │                   │
                      │ Git Status  │          │                   │
                      └─────────────┘          │                   │
                                               │                   │
┌─────────────┐     ┌─────────────┐            │                   │
│CLI: --quiet │     │CLI: open/   │            │                   │
└─────────────┘     │ stop --all  │            │                   │
                    └──────┬──────┘            │                   │
                           │                   │                   │
WAVE 3                     ▼                   │                   │
                    ┌─────────────┐            │                   │
                    │ GUI: Bulk   │            │                   │
                    │ Operations  │            │                   │
                    └─────────────┘            │                   │
                                               │                   │
┌─────────────┐                                │                   │
│ CLI: focus  │────────────────────────────────┼───────────────────┤
└─────────────┘                                │                   │
                                               ▼                   ▼
                                        ┌─────────────────────────────┐
                                        │     GUI: Quick Actions      │
                                        │  (Copy, Editor, Focus)      │
                                        └──────────────┬──────────────┘
                                                       │
WAVE 4 (Sequential)                                    ▼
                                        ┌─────────────────────────────┐
                                        │       GUI: Favorites        │
                                        └──────────────┬──────────────┘
                                                       │
                                                       ▼
                                        ┌─────────────────────────────┐
                                        │   GUI: Theme & Components   │
                                        └──────────────┬──────────────┘
                                                       │
                                                       ▼
                                        ┌─────────────────────────────┐
                                        │   GUI: Keyboard Shortcuts   │
                                        └─────────────────────────────┘
```

---

## Quick Reference: What Can Run Now?

| Task | Blocked By | Can Start Now? |
|------|------------|----------------|
| CLI-1.1: `--note` | Nothing | ✅ YES |
| CLI-1.2: `cd` | Nothing | ✅ YES |
| CLI-1.3: `code` | Nothing | ✅ YES |
| CLI-1.4: `--json` | Nothing | ✅ YES |
| CLI-1.5: `--quiet` | Nothing | ✅ YES |
| CLI-2.1: `focus` | Nothing | ✅ YES |
| CLI-2.5: `open/stop --all` | Nothing | ✅ YES |
| GUI-7: Status Dashboard | Nothing | ✅ YES |
| GUI-7.5: Notes | CLI-1.1 | ⏳ After CLI-1.1 |
| GUI-7.6: Bulk Ops | CLI-2.5 | ⏳ After CLI-2.5 |
| GUI-7.7: Quick Actions | CLI-1.2, 1.3, 2.1 | ⏳ After those 3 |

---

## Notes

1. **Parallel Tracks**: CLI and GUI can largely proceed in parallel, with GUI phases waiting only for their specific CLI dependencies.

2. **Small PRs**: Each task is designed to be a single, focused PR. Don't combine tasks.

3. **Value Delivery**: The order prioritizes features that deliver value quickly (`--json`, `--note`, Status Dashboard).

4. **Testing**: Each CLI feature should have tests before the GUI phase that depends on it begins.
