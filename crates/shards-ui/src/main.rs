//! shards-ui: GUI for Shards
//!
//! GPUI-based visual dashboard for shard management.

use gpui::{
    App, Application, Bounds, Context, FontWeight, IntoElement, Render, SharedString,
    TitlebarOptions, Window, WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
    uniform_list,
};
use shards_core::{Session, session_ops};

/// Display data for a shard, combining Session with computed process status
#[derive(Clone)]
struct ShardDisplay {
    session: Session,
    is_running: bool,
}

impl ShardDisplay {
    fn from_session(session: Session) -> Self {
        let is_running = session
            .process_id
            .map(|pid| shards_core::process::is_process_running(pid).unwrap_or(false))
            .unwrap_or(false);

        Self {
            session,
            is_running,
        }
    }
}

/// Main view displaying the shard list
struct ShardListView {
    displays: Vec<ShardDisplay>,
}

impl ShardListView {
    fn new() -> Self {
        let sessions = match session_ops::list_sessions() {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(event = "ui.shard_list.load_failed", error = %e);
                Vec::new()
            }
        };

        let displays: Vec<ShardDisplay> = sessions
            .into_iter()
            .map(ShardDisplay::from_session)
            .collect();

        Self { displays }
    }
}

impl Render for ShardListView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let content = if self.displays.is_empty() {
            div()
                .flex()
                .flex_1()
                .justify_center()
                .items_center()
                .text_color(rgb(0x888888))
                .child("No active shards")
        } else {
            let item_count = self.displays.len();
            div().flex_1().child(
                uniform_list("shard-list", item_count, {
                    let displays = self.displays.clone();
                    move |range, _window, _cx| {
                        range
                            .map(|ix| {
                                let display = &displays[ix];
                                let status_color = if display.is_running {
                                    rgb(0x00ff00)
                                } else {
                                    rgb(0xff0000)
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
                                            .child(display.session.branch.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_color(rgb(0x888888))
                                            .child(display.session.agent.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_color(rgb(0x666666))
                                            .child(display.session.project_id.clone()),
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
