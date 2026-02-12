//! View components for kild-ui.
//!
//! This module contains the view layer of the application:
//! - `main_view` - Root view composing multiplexer layout
//! - `rail` - 48px project icon rail (far left)
//! - `kild_sidebar` - 220px kild list with status grouping
//! - `teammate_tabs` - Horizontal tab bar for teammate switching
//! - `split_pane` - Split pane container with resize handles
//! - `pane_header` - Thin header above each terminal pane
//! - `minimized_bar` - Collapsed session bars for non-focused kilds
//! - `status_bar` - Bottom bar with alerts and keyboard hints
//! - `detail_view` - Sidebar inspect mode for selected kild
//! - `create_dialog` - Modal dialog for creating new kilds
//! - `confirm_dialog` - Modal dialog for confirming destructive actions
//! - `add_project_dialog` - Modal dialog for adding new projects

pub mod add_project_dialog;
pub mod confirm_dialog;
pub mod create_dialog;
pub mod detail_view;
pub mod kild_sidebar;
pub mod main_view;
pub mod minimized_bar;
pub mod pane_header;
pub mod rail;
pub mod split_pane;
pub mod status_bar;
pub mod teammate_tabs;

pub use main_view::MainView;
