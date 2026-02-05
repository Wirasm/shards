# Investigation: peek: type command only delivers first character to GPUI apps

**Issue**: #174 (https://github.com/Wirasm/kild/issues/174)
**Type**: BUG
**Investigated**: 2026-02-05

### Assessment

| Metric     | Value  | Reasoning                                                                                                     |
| ---------- | ------ | ------------------------------------------------------------------------------------------------------------- |
| Severity   | HIGH   | Core interaction command broken for GPUI apps (kild-ui, Zed) — no workaround via type command                 |
| Complexity | LOW    | Single function change in one file, pattern already exists in sibling key command                              |
| Confidence | HIGH   | Root cause clearly identified in issue, confirmed by code analysis — single-event set_string vs per-char loop |

---

## Problem Statement

The `kild-peek type` command sends text as a single CGEvent with `set_string()` for the entire string. GPUI-based apps (kild-ui, Zed) only read the first character from the event's unicode string field. The command reports success with the full text length but only one character is delivered. The fix is to send individual per-character keyboard events with small delays, matching the pattern used by the working `key` command.

---

## Analysis

### Root Cause

The single-event approach used by `type_text` is incompatible with GPUI's event handling. GPUI reads only the first character from a CGEvent's unicode string field.

### Evidence Chain

WHY: Only first character "a" appears when typing "test-peek-interaction"
↓ BECAUSE: `type_text` creates one CGEvent and calls `set_string()` with the full text
Evidence: `handler.rs:343-346` — single event with keycode 0, `set_string(request.text())`

↓ BECAUSE: GPUI reads only the first unicode character from each CGEvent
↓ ROOT CAUSE: The implementation sends one event for all characters instead of one event per character
Evidence: `handler.rs:340-351` — no loop, no delay, single event posted

### Working Pattern (key command)

The `send_key_combo` function at `handler.rs:377-438` shows the correct approach:
- Creates separate key_down and key_up events (lines 392-403)
- Posts key_down, sleeps `KEY_EVENT_DELAY` (10ms), posts key_up (lines 419-421)
- This per-event approach works correctly with GPUI

### Affected Files

| File                                                       | Lines   | Action | Description                                     |
| ---------------------------------------------------------- | ------- | ------ | ----------------------------------------------- |
| `crates/kild-peek-core/src/interact/handler.rs`           | 326-366 | UPDATE | Replace single-event with per-character loop     |

### Integration Points

- `crates/kild-peek/src/commands.rs:707` calls `type_text()` — no changes needed
- `crates/kild-peek-core/src/interact/types.rs:79-111` — TypeRequest unchanged
- `crates/kild-peek-core/src/interact/mod.rs:7` — exports unchanged

### Git History

- **Introduced**: `b0bab43` (2026-01-29) — "feat(kild-peek): add click, type, key interaction commands"
- **Comment added**: `df81db4` (2026-01-29) — "fix(kild-peek): address review findings for interact module"
- **Last modified**: `06e4063` (2026-01-30) — "feat(kild-peek): add --wait/--timeout to interaction and element commands"
- **Implication**: Original bug — the single-event approach was the initial implementation

---

## Implementation Plan

### Step 1: Add a per-character delay constant

**File**: `crates/kild-peek-core/src/interact/handler.rs`
**Lines**: After line 41 (after `DRAG_EVENT_DELAY`)
**Action**: UPDATE

**Required change:**

Add a new constant for the delay between per-character events:

```rust
/// Delay between individual character events when typing text
const CHAR_EVENT_DELAY: Duration = Duration::from_millis(5);
```

**Why**: 5ms matches the proposed fix in the issue and provides enough settling time between characters without being noticeably slow. The existing `KEY_EVENT_DELAY` (10ms) is for key down→up pairs; this is for inter-character spacing.

---

### Step 2: Replace single-event with per-character loop in `type_text`

**File**: `crates/kild-peek-core/src/interact/handler.rs`
**Lines**: 337-351
**Action**: UPDATE

**Current code:**

```rust
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|()| InteractionError::EventSourceFailed)?;

    // Create a keyboard event with keycode 0 and set the unicode string.
    // This sends text as a unicode string rather than individual key events,
    // which correctly handles special characters and international input.
    let event = CGEvent::new_keyboard_event(source, 0, true)
        .map_err(|()| InteractionError::KeyboardEventFailed { keycode: 0 })?;

    event.set_string(request.text());
    debug!(
        event = "peek.core.interact.type_posting",
        text_len = request.text().len()
    );
    event.post(CGEventTapLocation::HID);
```

**Required change:**

```rust
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|()| InteractionError::EventSourceFailed)?;

    // Send each character as an individual keyboard event.
    // GPUI and other Metal-based apps only read the first character from a
    // CGEvent's unicode string, so we must send one event per character.
    debug!(
        event = "peek.core.interact.type_posting",
        text_len = request.text().len()
    );
    for ch in request.text().chars() {
        let event = CGEvent::new_keyboard_event(source.clone(), 0, true)
            .map_err(|()| InteractionError::KeyboardEventFailed { keycode: 0 })?;
        event.set_string(&ch.to_string());
        event.post(CGEventTapLocation::HID);
        thread::sleep(CHAR_EVENT_DELAY);
    }
```

**Why**: Each character gets its own CGEvent with `set_string()` for that single character. The 5ms delay between events prevents event coalescing. This matches the approach in issue #174 and the reference in #141.

---

### Step 3: Verify existing tests still pass

No new test files needed. The existing unit tests in `types.rs` and `operations.rs` cover request construction and key parsing. The `type_text` function itself requires accessibility permissions for integration testing (manual only).

**Test cases already covered:**
- `test_type_request_new` (types.rs) — TypeRequest construction
- `test_type_request_with_wait` (types.rs) — wait timeout
- CLI output tests in `crates/kild-peek/tests/cli_output.rs`

---

## Patterns to Follow

**From codebase — key command's per-event delay pattern:**

```rust
// SOURCE: handler.rs:419-421
// Pattern for event posting with inter-event delay
key_down.post(CGEventTapLocation::HID);
thread::sleep(KEY_EVENT_DELAY);
key_up.post(CGEventTapLocation::HID);
```

**From codebase — delay constant naming:**

```rust
// SOURCE: handler.rs:31-41
// Pattern for delay constants
const MOUSE_EVENT_DELAY: Duration = Duration::from_millis(10);
const FOCUS_SETTLE_DELAY: Duration = Duration::from_millis(50);
const KEY_EVENT_DELAY: Duration = Duration::from_millis(10);
const DRAG_EVENT_DELAY: Duration = Duration::from_millis(25);
```

---

## Edge Cases & Risks

| Risk/Edge Case              | Mitigation                                                                               |
| --------------------------- | ---------------------------------------------------------------------------------------- |
| Empty string input          | Loop body never executes, success returned with text_length: 0 — correct behavior        |
| Long strings slow to type   | 5ms × N chars. 100 chars = 500ms. Acceptable for a dev tool                              |
| Unicode/emoji characters    | `set_string()` per character handles multi-byte chars correctly (same API as before)      |
| Event coalescing            | 5ms delay between events prevents coalescing                                             |
| CGEventSource clone per char| `CGEventSource::clone()` is cheap (refcounted) — same pattern used in `send_key_combo`   |

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

1. Open kild-ui, click "+ Create" to open the create dialog
2. Run: `cargo run -p kild-peek -- type --window "KILD" "test-peek-interaction"`
3. Verify the full string "test-peek-interaction" appears in the Branch Name input
4. Test with Zed editor (another GPUI app) to confirm cross-GPUI compatibility
5. Test with a standard AppKit app (e.g., TextEdit) to verify no regression

---

## Scope Boundaries

**IN SCOPE:**
- Replace single-event `set_string()` with per-character loop in `type_text()`
- Add `CHAR_EVENT_DELAY` constant

**OUT OF SCOPE (do not touch):**
- `send_key_combo` — already works correctly
- `click` / other interaction commands — unrelated
- TypeRequest struct — no changes needed
- CLI handler — no changes needed
- Tests — existing tests cover request types; integration test requires manual execution

---

## Metadata

- **Investigated by**: Claude
- **Timestamp**: 2026-02-05
- **Artifact**: `.claude/PRPs/issues/issue-174.md`
