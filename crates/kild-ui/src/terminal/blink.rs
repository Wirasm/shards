use std::time::Duration;

use gpui::{Context, Task};

use super::terminal_view::TerminalView;

const BLINK_INTERVAL: Duration = Duration::from_millis(500);

/// Manages cursor blink state with epoch-based timer cancellation.
///
/// Each call to `enable()` or `pause()` increments the epoch and spawns a new
/// async blink cycle via `cx.spawn()` on the owning `TerminalView`. Old cycles
/// detect the stale epoch and exit, so at most one timer drives repaints.
pub struct BlinkManager {
    visible: bool,
    enabled: bool,
    epoch: usize,
    /// Stored to prevent cancellation of the current blink timer task.
    _task: Option<Task<()>>,
}

impl BlinkManager {
    pub fn new() -> Self {
        Self {
            visible: true,
            enabled: false,
            epoch: 0,
            _task: None,
        }
    }

    /// Whether the cursor should be painted this frame.
    pub fn visible(&self) -> bool {
        !self.enabled || self.visible
    }

    /// Start blinking. Resets visibility and spawns a new blink cycle.
    pub fn enable(&mut self, cx: &mut Context<TerminalView>) {
        self.enabled = true;
        self.visible = true;
        self.start_cycle(cx);
    }

    /// Show the cursor immediately and restart the blink timer.
    ///
    /// Call on every keystroke so the cursor stays visible during typing
    /// and only starts blinking again after a full interval of inactivity.
    pub fn pause(&mut self, cx: &mut Context<TerminalView>) {
        if !self.enabled {
            return;
        }
        self.visible = true;
        self.start_cycle(cx);
    }

    fn start_cycle(&mut self, cx: &mut Context<TerminalView>) {
        self.epoch = self.epoch.wrapping_add(1);
        let epoch = self.epoch;

        self._task = Some(cx.spawn(async move |this, cx: &mut gpui::AsyncApp| {
            loop {
                cx.background_executor().timer(BLINK_INTERVAL).await;

                let should_continue = this
                    .update(cx, |view, cx| {
                        if view.blink.epoch != epoch {
                            return false;
                        }
                        view.blink.visible = !view.blink.visible;
                        cx.notify();
                        true
                    })
                    .unwrap_or(false);

                if !should_continue {
                    break;
                }
            }
        }));
    }
}
