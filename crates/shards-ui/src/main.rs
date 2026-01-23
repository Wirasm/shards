//! shards-ui: GUI for Shards
//!
//! GPUI-based visual dashboard for shard management.

use gpui::{
    App, Application, Bounds, Context, FontWeight, IntoElement, Render, SharedString,
    TitlebarOptions, Window, WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
    uniform_list,
};
use shards_core::{Session, session_ops};

/// Process status for a shard, distinguishing between running, stopped, and unknown states
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProcessStatus {
    /// Process is confirmed running
    Running,
    /// Process is confirmed stopped (or no PID exists)
    Stopped,
    /// Could not determine status (process check failed)
    Unknown,
}

/// Display data for a shard, combining Session with computed process status
#[derive(Clone)]
struct ShardDisplay {
    session: Session,
    status: ProcessStatus,
}

impl ShardDisplay {
    fn from_session(session: Session) -> Self {
        let status = session.process_id.map_or(ProcessStatus::Stopped, |pid| {
            match shards_core::process::is_process_running(pid) {
                Ok(true) => ProcessStatus::Running,
                Ok(false) => ProcessStatus::Stopped,
                Err(e) => {
                    tracing::warn!(
                        event = "ui.shard_list.process_check_failed",
                        pid = pid,
                        branch = &session.branch,
                        error = %e
                    );
                    ProcessStatus::Unknown
                }
            }
        });

        Self { session, status }
    }

    fn session(&self) -> &Session {
        &self.session
    }

    fn status(&self) -> ProcessStatus {
        self.status
    }
}

/// Main view displaying the shard list.
///
/// This view holds a point-in-time snapshot of shard data loaded
/// at construction. Data does not refresh automatically - the
/// window must be reopened to see updates (Phase 3 behavior).
struct ShardListView {
    displays: Vec<ShardDisplay>,
    load_error: Option<String>,
}

impl ShardListView {
    fn new() -> Self {
        let (sessions, load_error) = match session_ops::list_sessions() {
            Ok(s) => (s, None),
            Err(e) => {
                tracing::error!(event = "ui.shard_list.load_failed", error = %e);
                (Vec::new(), Some(e.to_string()))
            }
        };

        let displays: Vec<ShardDisplay> = sessions
            .into_iter()
            .map(ShardDisplay::from_session)
            .collect();

        Self {
            displays,
            load_error,
        }
    }

    fn displays(&self) -> &[ShardDisplay] {
        &self.displays
    }

    fn load_error(&self) -> Option<&str> {
        self.load_error.as_deref()
    }
}

impl Render for ShardListView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let content = if let Some(error_msg) = self.load_error() {
            // Error state - show error message
            div()
                .flex()
                .flex_1()
                .justify_center()
                .items_center()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_color(rgb(0xff6b6b))
                        .child("Error loading shards"),
                )
                .child(
                    div()
                        .text_color(rgb(0x888888))
                        .text_sm()
                        .child(error_msg.to_string()),
                )
        } else if self.displays().is_empty() {
            // Empty state - no shards exist
            div()
                .flex()
                .flex_1()
                .justify_center()
                .items_center()
                .text_color(rgb(0x888888))
                .child("No active shards")
        } else {
            // List state - show shards
            let item_count = self.displays().len();
            div().flex_1().child(
                uniform_list("shard-list", item_count, {
                    let displays = self.displays.clone();
                    move |range, _window, _cx| {
                        range
                            .map(|ix| {
                                let display = &displays[ix];
                                let status_color = match display.status() {
                                    ProcessStatus::Running => rgb(0x00ff00), // Green
                                    ProcessStatus::Stopped => rgb(0xff0000), // Red
                                    ProcessStatus::Unknown => rgb(0xffa500), // Orange
                                };

                                div()
                                    .id(ix)
                                    .w_full()
                                    .px_4()
                                    .py_2()
                                    .flex()
                                    .gap_3()
                                    .child(div().text_color(status_color).child("●"))
                                    .child(
                                        div()
                                            .flex_1()
                                            .text_color(rgb(0xffffff))
                                            .child(display.session().branch.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_color(rgb(0x888888))
                                            .child(display.session().agent.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_color(rgb(0x666666))
                                            .child(display.session().project_id.clone()),
                                    )
                            })
                            .collect()
                    }
                })
                .h_full(),
            )
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x1e1e1e))
            .child(
                // Header
                div().px_4().py_3().flex().items_center().child(
                    div()
                        .text_xl()
                        .text_color(rgb(0xffffff))
                        .font_weight(FontWeight::BOLD)
                        .child("Shards"),
                ),
            )
            .child(content)
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(800.0), px(600.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some(SharedString::from("Shards")),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(|_| ShardListView::new()),
        )
        .expect("Failed to open window");
    });
}
