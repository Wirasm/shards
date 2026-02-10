# Investigation: kild open --no-agent doesn't set session status to active

**Issue**: #257 (https://github.com/Wirasm/kild/issues/257)
**Type**: BUG
**Investigated**: 2026-02-10

### Assessment

| Metric     | Value  | Reasoning                                                                                          |
| ---------- | ------ | -------------------------------------------------------------------------------------------------- |
| Severity   | MEDIUM | Session shows wrong status in `kild list`, but the shell process runs fine — cosmetic/UX bug       |
| Complexity | LOW    | Single conditional on one line in one file; fix is removing/adjusting one condition                 |
| Confidence | HIGH   | Root cause is a single boolean condition at handler.rs:921, confirmed via git blame and code trace  |

---

## Problem Statement

When running `kild open <branch> --no-agent`, the session status remains "stopped" even though a shell process is successfully spawned and running. The `kild list` output shows `Run(1/1)` in the process column (confirming the shell is alive) but "stopped" in the status column. Normal `kild open <branch>` (with agent) correctly sets status to "active".

---

## Analysis

### Root Cause

WHY: Session status shows "stopped" after `kild open --no-agent`
-> BECAUSE: The status update at handler.rs:922 is guarded by a conditional that evaluates to `false` for bare shell + terminal mode

-> BECAUSE: Line 921 checks `if !is_bare_shell || use_daemon` — when `is_bare_shell=true` and `use_daemon=false`, this evaluates to `false || false = false`, skipping the status update

-> ROOT CAUSE: The conditional at handler.rs:921 was designed to keep bare shell sessions as "Stopped" in terminal mode, based on the reasoning that "no agent is running". However, a shell process IS running — the session should be Active regardless of whether the process is an agent or a bare shell.

Evidence: `crates/kild-core/src/sessions/handler.rs:919-923`:
```rust
// When bare shell in terminal mode, keep session Stopped (no agent is running).
// Bare shell in daemon mode IS active (the daemon PTY is running).
if !is_bare_shell || use_daemon {
    session.status = SessionStatus::Active;
}
```

### Evidence Chain

WHY: Status shows "stopped" after open
-> BECAUSE: `session.status = SessionStatus::Active` at line 922 is skipped
Evidence: `handler.rs:921` - `if !is_bare_shell || use_daemon` evaluates to `false` when `is_bare_shell=true, use_daemon=false`

-> BECAUSE: The original PR #251 (commit 4ec1462) introduced this logic intentionally, treating bare shell as "not active"
Evidence: `git blame -L 919,923 handler.rs` shows lines 919-921 from commit 71f3343 (PR #301) and lines 922-923 from commit 4ec1462 (PR #251)

-> ROOT CAUSE: The design assumption was wrong — a spawned shell process makes the session active, regardless of whether it's an agent or bare shell. The terminal backend spawns a real process either way.

### Affected Files

| File                                            | Lines   | Action | Description                                    |
| ----------------------------------------------- | ------- | ------ | ---------------------------------------------- |
| `crates/kild-core/src/sessions/handler.rs`      | 919-923 | UPDATE | Remove bare-shell conditional, always set Active |

### Integration Points

- `crates/kild/src/commands/open.rs:38` — CLI calls `open_session`
- `crates/kild-core/src/state/dispatch.rs:62` — UI dispatches `Command::OpenKild` to `open_session`
- `crates/kild-core/src/sessions/persistence.rs` — saves session with the status

### Git History

- **Introduced**: 4ec1462 - 2026-02-05 - "feat: add --no-agent flag to kild open (#251)"
- **Modified**: 71f3343 - 2026-02-10 - "feat: add kild-tmux-shim crate for agent team support in daemon sessions (#301)" — added `|| use_daemon` to support daemon bare shells
- **Implication**: Original design decision, not a regression. The bare shell was always kept as Stopped in terminal mode.

---

## Implementation Plan

### Step 1: Remove bare-shell conditional in open_session

**File**: `crates/kild-core/src/sessions/handler.rs`
**Lines**: 919-923
**Action**: UPDATE

**Current code:**
```rust
// When bare shell in terminal mode, keep session Stopped (no agent is running).
// Bare shell in daemon mode IS active (the daemon PTY is running).
if !is_bare_shell || use_daemon {
    session.status = SessionStatus::Active;
}
```

**Required change:**
```rust
session.status = SessionStatus::Active;
```

**Why**: A process is spawned in all cases (agent or bare shell, terminal or daemon). The session is active whenever a process is running. Remove the conditional and always set Active.

---

## Patterns to Follow

**From codebase — `create_session` always sets Active:**

```rust
// SOURCE: crates/kild-core/src/sessions/handler.rs:415
// create_session always creates sessions as Active, including --no-agent
let session = Session::new(
    session_id.clone(),
    project.id,
    validated.name.clone(),
    worktree.path,
    validated.agent.clone(),
    SessionStatus::Active,  // Always Active
    // ...
);
```

---

## Edge Cases & Risks

| Risk/Edge Case                      | Mitigation                                                         |
| ----------------------------------- | ------------------------------------------------------------------ |
| Terminal bare shell exits immediately | `stop_session` or process monitoring will set status back to Stopped |
| Daemon bare shell behavior changes  | No change — daemon path already sets Active via the removed condition |

---

## Validation

### Automated Checks

```bash
cargo fmt --check
cargo clippy --all -- -D warnings
cargo test --all
cargo build --all
```

### Manual Verification

1. `cargo run -p kild -- create test-bare --no-agent` then `cargo run -p kild -- list` — should show Active
2. `cargo run -p kild -- stop test-bare` then `cargo run -p kild -- open test-bare --no-agent` then `cargo run -p kild -- list` — should show Active
3. `cargo run -p kild -- stop test-bare` then `cargo run -p kild -- open test-bare` then `cargo run -p kild -- list` — should show Active (regression check)

---

## Scope Boundaries

**IN SCOPE:**
- Remove the bare-shell conditional in `open_session` so status is always set to Active

**OUT OF SCOPE (do not touch):**
- `create_session` — already correctly sets Active for all modes
- `restart_session` — already correctly sets Active unconditionally
- Adding tests — this is a one-line fix with trivial behavior change

---

## Metadata

- **Investigated by**: Claude
- **Timestamp**: 2026-02-10
- **Artifact**: `.claude/PRPs/issues/issue-257.md`
