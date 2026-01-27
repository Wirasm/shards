# Feature: GUI Phase 9.3 - StatusIndicator Component

## Summary

Create a reusable StatusIndicator component for kild-ui that renders status dots and badges with consistent styling from the brand system. The component supports three statuses (Active, Stopped, Crashed) in two display modes (Dot and Badge), with appropriate colors and glow effects for each state.

## User Story

As a developer working on kild-ui
I want a reusable StatusIndicator component with typed status values
So that I can display consistent, branded status indicators throughout the UI

## Problem Statement

The current kild-ui codebase has status indication implemented inline:
- Status dots rendered as text characters (`●`) with direct RGB colors
- No glow effects on Active/Crashed states as specified in brand mockup
- Color values don't match the Tallinn Night brand palette
- No badge variant (just dots)
- ProcessStatus enum (Running/Stopped/Unknown) doesn't map cleanly to visual states

## Solution Statement

Create a `StatusIndicator` struct that implements GPUI's `RenderOnce` trait with:
1. A `Status` enum for type-safe visual states (Active, Stopped, Crashed)
2. Two display modes: Dot (8px circle) and Badge (pill with text)
3. Theme-based colors from the brand system
4. Glow effects for Active and Crashed states
5. Simple API via factory methods (`dot()`, `badge()`)

## Metadata

| Field | Value |
|-------|-------|
| Type | NEW_CAPABILITY |
| Complexity | LOW |
| Systems Affected | kild-ui |
| Dependencies | gpui 0.2, theme module (Phase 9.1) |
| Estimated Tasks | 3 |

---

## UX Design

### Before State

```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                              BEFORE STATE                                      ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  kild_list.rs:                                                                ║
║  ┌────────────────────────────────────────────────────────────────────────┐  ║
║  │ let status_color = match display.status {                               │  ║
║  │     ProcessStatus::Running => rgb(0x00ff00), // Bright green            │  ║
║  │     ProcessStatus::Stopped => rgb(0xff0000), // Bright red              │  ║
║  │     ProcessStatus::Unknown => rgb(0x888888), // Gray                    │  ║
║  │ };                                                                      │  ║
║  │ // ...                                                                  │  ║
║  │ .child(div().text_color(status_color).child("●"))                      │  ║
║  └────────────────────────────────────────────────────────────────────────┘  ║
║                                                                               ║
║  PROBLEMS:                                                                    ║
║  - Colors don't match brand (0x00ff00 vs Aurora #34D399)                     ║
║  - No glow effect on Active/Crashed                                          ║
║  - Text character instead of proper circle element                           ║
║  - No badge variant for detail panels                                        ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

### After State

```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                               AFTER STATE                                      ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  components/status_indicator.rs (NEW):                                         ║
║  ┌────────────────────────────────────────────────────────────────────────┐  ║
║  │ pub enum Status { Active, Stopped, Crashed }                            │  ║
║  │                                                                         │  ║
║  │ impl Status {                                                           │  ║
║  │     fn color(&self) -> Hsla { /* aurora, copper, ember */ }            │  ║
║  │     fn has_glow(&self) -> bool { /* Active, Crashed = true */ }        │  ║
║  │     fn label(&self) -> &'static str { /* "Active", etc. */ }           │  ║
║  │ }                                                                       │  ║
║  │                                                                         │  ║
║  │ pub struct StatusIndicator { ... }                                      │  ║
║  │                                                                         │  ║
║  │ impl StatusIndicator {                                                  │  ║
║  │     pub fn dot(status: Status) -> Self;                                 │  ║
║  │     pub fn badge(status: Status) -> Self;                               │  ║
║  │ }                                                                       │  ║
║  └────────────────────────────────────────────────────────────────────────┘  ║
║                                                                               ║
║  Usage in views:                                                               ║
║  ┌────────────────────────────────────────────────────────────────────────┐  ║
║  │ StatusIndicator::dot(Status::Active)    // 8px green dot with glow     │  ║
║  │ StatusIndicator::badge(Status::Stopped) // Pill: "● Stopped"           │  ║
║  └────────────────────────────────────────────────────────────────────────┘  ║
║                                                                               ║
║  VALUE: Consistent branding, glow effects, type-safe status mapping          ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

### Interaction Changes

| Location | Before | After | User Impact |
|----------|--------|-------|-------------|
| Kild list rows | Text "●" with wrong colors | Proper 8px dot with brand colors | Matches mockup |
| Detail panel | No status badge | Badge with glow background | Clearer status |
| Header stats | Dots in text | StatusIndicator::dot() | Consistent styling |

---

## Mandatory Reading

**CRITICAL: Implementation agent MUST read these files before starting any task:**

| Priority | File | Lines | Why Read This |
|----------|------|-------|---------------|
| P0 | `crates/kild-ui/src/theme.rs` | all | Theme colors (aurora, copper, ember, glow functions) |
| P0 | `crates/kild-ui/src/views/kild_list.rs` | 145-204 | Current status color pattern to replace |
| P0 | `crates/kild-ui/src/state.rs` | 13-38 | ProcessStatus and GitStatus enums |
| P1 | `.claude/PRPs/branding/mockup-dashboard.html` | 358-382 | Dot CSS spec (8px, glow, pulse) |
| P1 | `.claude/PRPs/branding/mockup-dashboard.html` | 566-590 | Badge CSS spec |
| P2 | `crates/kild-ui/src/components/button.rs` | all | RenderOnce pattern to follow |

**External Documentation:**

| Source | Section | Why Needed |
|--------|---------|------------|
| [GPUI docs.rs - RenderOnce](https://docs.rs/gpui/latest/gpui/trait.RenderOnce.html) | Trait definition | Component pattern |

---

## Patterns to Mirror

**COMPONENT_STRUCTURE:**
```rust
// SOURCE: crates/kild-ui/src/components/button.rs (Phase 9.2)
// COPY THIS PATTERN:
#[derive(IntoElement)]
pub struct Button {
    // fields...
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        // build element...
    }
}
```

**STATUS_COLOR_PATTERN:**
```rust
// SOURCE: crates/kild-ui/src/views/kild_list.rs:145-149
// CURRENT PATTERN (to replace):
let status_color = match display.status {
    ProcessStatus::Running => rgb(0x00ff00), // Green
    ProcessStatus::Stopped => rgb(0xff0000), // Red
    ProcessStatus::Unknown => rgb(0x888888), // Gray
};
```

**THEME_COLOR_USAGE:**
```rust
// SOURCE: crates/kild-ui/src/theme.rs
// COPY THIS PATTERN:
pub fn aurora() -> Hsla { gpui::rgb(0x34D399) }
pub fn aurora_glow() -> Hsla { with_alpha(aurora(), 0.15) }
```

---

## Files to Change

| File | Action | Justification |
|------|--------|---------------|
| `crates/kild-ui/src/components/status_indicator.rs` | CREATE | StatusIndicator component |
| `crates/kild-ui/src/components/mod.rs` | UPDATE | Export StatusIndicator |

---

## NOT Building (Scope Limits)

Explicit exclusions to prevent scope creep:

- **Replacing existing status dots in views** - That's Phase 9.6
- **Pulse animation for Crashed** - GPUI animation is complex, defer to future
- **GitStatus indicator** - Keep as separate concept (orange dot for dirty)
- **Unknown status variant** - Map to Stopped visually for now

---

## Step-by-Step Tasks

Execute in order. Each task is atomic and independently verifiable.

### Task 1: CREATE `crates/kild-ui/src/components/status_indicator.rs`

- **ACTION**: Create the StatusIndicator component
- **IMPLEMENT**:

```rust
//! Status indicator component for kild session states.
//!
//! Provides consistent status visualization with dots and badges.
//! All colors come from the theme module (Tallinn Night brand).

use gpui::{div, prelude::*, px, Hsla, IntoElement, RenderOnce, SharedString, Window, App};

use crate::theme;

/// Visual status states for kilds.
///
/// Maps to ProcessStatus but with visual-focused naming.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Status {
    /// Running/active - Aurora (green) with glow
    #[default]
    Active,
    /// Stopped/idle - Copper (amber) no glow
    Stopped,
    /// Crashed/error - Ember (red) with glow
    Crashed,
}

impl Status {
    /// Get the primary color for this status.
    pub fn color(&self) -> Hsla {
        match self {
            Status::Active => theme::aurora(),
            Status::Stopped => theme::copper(),
            Status::Crashed => theme::ember(),
        }
    }

    /// Get the glow/background color for this status (15% alpha).
    pub fn glow_color(&self) -> Hsla {
        match self {
            Status::Active => theme::aurora_glow(),
            Status::Stopped => theme::copper_glow(),
            Status::Crashed => theme::ember_glow(),
        }
    }

    /// Whether this status should have a glow effect.
    pub fn has_glow(&self) -> bool {
        match self {
            Status::Active => true,
            Status::Stopped => false,
            Status::Crashed => true,
        }
    }

    /// Get the text label for badge display.
    pub fn label(&self) -> &'static str {
        match self {
            Status::Active => "Active",
            Status::Stopped => "Stopped",
            Status::Crashed => "Crashed",
        }
    }
}

/// Display size for the status indicator.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StatusSize {
    /// Small dot (8px circle)
    #[default]
    Dot,
    /// Badge with text (pill shape with dot + label)
    Badge,
}

/// A status indicator component.
///
/// # Example
///
/// ```rust
/// // Simple dot indicator
/// StatusIndicator::dot(Status::Active)
///
/// // Badge with label
/// StatusIndicator::badge(Status::Stopped)
/// ```
#[derive(IntoElement)]
pub struct StatusIndicator {
    status: Status,
    size: StatusSize,
}

impl StatusIndicator {
    /// Create a dot indicator (8px colored circle).
    pub fn dot(status: Status) -> Self {
        Self {
            status,
            size: StatusSize::Dot,
        }
    }

    /// Create a badge indicator (pill with dot + text label).
    pub fn badge(status: Status) -> Self {
        Self {
            status,
            size: StatusSize::Badge,
        }
    }
}

impl RenderOnce for StatusIndicator {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let color = self.status.color();
        let glow = self.status.glow_color();
        let has_glow = self.status.has_glow();

        match self.size {
            StatusSize::Dot => {
                // 8px circle with optional glow
                let mut dot = div()
                    .size(px(8.0))
                    .rounded_full()
                    .bg(color);

                // Add glow effect via box-shadow simulation
                // GPUI doesn't have box-shadow, so we use a subtle background
                if has_glow {
                    // For glow, we wrap in a container with glow background
                    div()
                        .size(px(16.0))
                        .rounded_full()
                        .bg(glow)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(dot)
                } else {
                    dot
                }
            }
            StatusSize::Badge => {
                // Pill shape: background glow + dot + text
                div()
                    .flex()
                    .items_center()
                    .gap(px(theme::SPACE_1))
                    .px(px(theme::SPACE_2))
                    .py(px(2.0))
                    .bg(glow)
                    .rounded(px(theme::RADIUS_SM))
                    .child(
                        // Small dot inside badge
                        div()
                            .size(px(6.0))
                            .rounded_full()
                            .bg(color),
                    )
                    .child(
                        div()
                            .text_color(color)
                            .text_size(px(theme::TEXT_XS))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .child(self.status.label()),
                    )
            }
        }
    }
}
```

- **GOTCHA**: GPUI doesn't have CSS `box-shadow`, so glow is simulated with a larger background element
- **GOTCHA**: The dot render returns different element types based on glow, which is fine since both impl `IntoElement`
- **GOTCHA**: Badge uses 6px dot (smaller than standalone 8px) per mockup visual balance
- **VALIDATE**: `cargo build -p kild-ui`

### Task 2: UPDATE `crates/kild-ui/src/components/mod.rs`

- **ACTION**: Export the StatusIndicator component
- **IMPLEMENT**:

```rust
//! Reusable UI components for kild-ui.
//!
//! This module contains extracted, styled components that ensure
//! visual consistency across the application.

mod button;
mod status_indicator;

pub use button::{Button, ButtonVariant};
pub use status_indicator::{Status, StatusIndicator, StatusSize};
```

- **MIRROR**: Existing mod.rs pattern from Phase 9.2
- **VALIDATE**: `cargo build -p kild-ui`

### Task 3: VERIFY StatusIndicator is accessible and renders

- **ACTION**: Verify the component compiles and exports are accessible
- **IMPLEMENT**: Temporarily add a test usage in a view (remove after verification)

```rust
// Temporary verification in kild_list.rs (remove after):
use crate::components::{Status, StatusIndicator};

// In render():
// let _test_dot = StatusIndicator::dot(Status::Active);
// let _test_badge = StatusIndicator::badge(Status::Stopped);
```

- **VALIDATE**:
  ```bash
  cargo build -p kild-ui
  cargo clippy -p kild-ui -- -D warnings
  ```

---

## Testing Strategy

### Unit Tests to Write

No unit tests for this phase - it's a visual component. Validation is through:
1. Compilation (type safety)
2. Visual inspection when integrated (Phase 9.6)

### Edge Cases Checklist

- [x] All 3 status values render with correct colors
- [x] Active and Crashed have glow, Stopped does not
- [x] Badge shows correct label text
- [x] Dot is proper circle (not text character)

---

## Validation Commands

### Level 1: STATIC_ANALYSIS

```bash
cargo fmt -p kild-ui --check && cargo clippy -p kild-ui -- -D warnings
```

**EXPECT**: Exit 0, no errors or warnings

### Level 2: BUILD

```bash
cargo build -p kild-ui
```

**EXPECT**: Compiles successfully, Status and StatusIndicator exported

### Level 3: FULL_SUITE

```bash
cargo build --all && cargo clippy --all -- -D warnings
```

**EXPECT**: All crates build, no warnings

---

## Acceptance Criteria

- [x] `Status` enum with 3 variants (Active, Stopped, Crashed)
- [x] `StatusSize` enum with 2 variants (Dot, Badge)
- [x] `StatusIndicator` struct with `dot()` and `badge()` factory methods
- [x] `RenderOnce` implemented correctly
- [x] Colors sourced from theme module (aurora, copper, ember)
- [x] Glow effects on Active and Crashed (via background)
- [x] Badge shows text label with proper styling
- [x] `cargo build -p kild-ui` succeeds
- [x] `cargo clippy -p kild-ui -- -D warnings` passes

---

## Completion Checklist

- [ ] Task 1: status_indicator.rs created with full implementation
- [ ] Task 2: components/mod.rs updated with exports
- [ ] Task 3: Verified accessibility from views
- [ ] Level 1: `cargo fmt` and `cargo clippy` pass
- [ ] Level 2: `cargo build -p kild-ui` succeeds
- [ ] Level 3: Full workspace builds

---

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Theme module not ready (Phase 9.1) | HIGH | HIGH | Must complete 9.1 first |
| GPUI lacks box-shadow | HIGH | LOW | Use background element for glow simulation |
| Different return types in match | LOW | LOW | Both branches return impl IntoElement |

---

## Notes

**Design Decision: Glow Simulation**

GPUI doesn't have CSS `box-shadow`. The mockup specifies `box-shadow: 0 0 8px rgba(color, 0.15)`. We simulate this by wrapping the dot in a larger (16px) container with the glow color as background. This creates a similar visual effect.

**Design Decision: No Pulse Animation**

The mockup specifies a 2-second pulse animation for Crashed status. GPUI animation is more complex and would require significant additional code. Deferring this to a future enhancement - the red color + glow is sufficient indication for now.

**Mapping ProcessStatus to Status**

When integrating (Phase 9.6), map as follows:
- `ProcessStatus::Running` → `Status::Active`
- `ProcessStatus::Stopped` → `Status::Stopped`
- `ProcessStatus::Unknown` → `Status::Stopped` (fallback)

**Dependency on Phase 9.1**

This component REQUIRES the theme module from Phase 9.1. The glow functions (`aurora_glow()`, `copper_glow()`, `ember_glow()`) must exist before this can compile.

---

## Sources

- [GPUI docs.rs - RenderOnce](https://docs.rs/gpui/latest/gpui/trait.RenderOnce.html)
- Mockup: `.claude/PRPs/branding/mockup-dashboard.html` lines 358-382 (dot CSS)
- Mockup: `.claude/PRPs/branding/mockup-dashboard.html` lines 566-590 (badge CSS)
