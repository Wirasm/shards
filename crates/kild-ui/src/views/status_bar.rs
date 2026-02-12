//! Bottom status bar with alerts and keyboard hints.
//!
//! Thin 24px bar at the bottom of the main window showing system state
//! on the left and keyboard shortcuts on the right.

use gpui::{Context, IntoElement, div, prelude::*, px};

use crate::state::AppState;
use crate::theme;
use crate::views::main_view::MainView;

/// Render the bottom status bar.
///
/// Left side shows a brief status message (placeholder "Ready" for now).
/// Right side shows keyboard shortcut hints for split/nav operations.
pub fn render_status_bar(_state: &AppState, _cx: &mut Context<MainView>) -> impl IntoElement {
    div()
        .w_full()
        .h(px(24.0))
        .flex()
        .items_center()
        .justify_between()
        .px(px(theme::SPACE_3))
        .bg(theme::obsidian())
        .border_t_1()
        .border_color(theme::border_subtle())
        .child(
            div()
                .text_size(px(theme::TEXT_XS))
                .text_color(theme::text_muted())
                .child("Ready"),
        )
        .child(
            div()
                .text_size(px(theme::TEXT_XS))
                .text_color(theme::text_muted())
                .child("Ctrl+\\ split  Ctrl+K/J nav"),
        )
}
