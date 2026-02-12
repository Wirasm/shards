//! Collapsed single-line session bar for non-focused active kilds.
//!
//! Each bar is a clickable 28px strip showing branch name, agent, and status dot.
//! Clicking a bar focuses that kild in the main area.

use gpui::{Context, IntoElement, div, prelude::*, px};

use crate::components::{Status, StatusIndicator};
use crate::state::AppState;
use crate::theme;
use crate::views::main_view::MainView;

/// Map a process status to a visual status.
fn status_from_process(process_status: &kild_core::ProcessStatus) -> Status {
    match process_status {
        kild_core::ProcessStatus::Running => Status::Active,
        kild_core::ProcessStatus::Stopped | kild_core::ProcessStatus::Unknown => Status::Stopped,
    }
}

/// Render a single minimized bar for a kild session.
fn render_bar(
    branch: &str,
    agent: &str,
    status: Status,
    session_id: String,
    cx: &mut Context<MainView>,
) -> impl IntoElement {
    let id = gpui::ElementId::Name(format!("minimized-bar-{}", session_id).into());
    div()
        .id(id)
        .w_full()
        .h(px(28.0))
        .flex()
        .items_center()
        .gap(px(theme::SPACE_2))
        .px(px(theme::SPACE_3))
        .bg(theme::surface())
        .border_b_1()
        .border_color(theme::border_subtle())
        .cursor_pointer()
        .hover(|this| this.bg(theme::elevated()))
        .on_click({
            let session_id = session_id.clone();
            cx.listener(move |view, _, window, cx| {
                view.on_kild_sidebar_click(&session_id, window, cx);
            })
        })
        .child(StatusIndicator::dot(status))
        .child(
            div()
                .text_size(px(theme::TEXT_SM))
                .text_color(theme::text_bright())
                .child(branch.to_string()),
        )
        .child(
            div()
                .text_size(px(theme::TEXT_XS))
                .text_color(theme::text_muted())
                .child(agent.to_string()),
        )
}

/// Render stacked minimized bars for non-focused active kilds.
///
/// Shows a compact bar for each running session that is not currently focused,
/// allowing quick switching between active kilds.
pub fn render_minimized_bars(state: &AppState, cx: &mut Context<MainView>) -> impl IntoElement {
    let focused_id = state.focused_kild_id();
    let bars: Vec<_> = state
        .filtered_displays()
        .into_iter()
        .filter(|d| d.process_status == kild_core::ProcessStatus::Running)
        .filter(|d| focused_id != Some(d.session.id.as_str()))
        .map(|d| {
            let status = status_from_process(&d.process_status);
            render_bar(
                &d.session.branch,
                &d.session.agent,
                status,
                d.session.id.clone(),
                cx,
            )
            .into_any_element()
        })
        .collect();

    div().w_full().flex().flex_col().children(bars)
}
