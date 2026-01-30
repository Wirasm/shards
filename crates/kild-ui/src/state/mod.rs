//! Application state for kild-ui.
//!
//! Centralized state management for the GUI. AppState is composed of
//! specialized modules that encapsulate related state:
//! - `DialogState`: Mutually exclusive dialog states (create, confirm, add project)
//! - `OperationErrors`: Per-branch and bulk operation error tracking
//! - `SelectionState`: Kild selection for detail panel
//! - `SessionStore`: Session display data with refresh tracking

pub mod app_state;
pub mod dialog;
pub mod errors;
pub mod selection;
pub mod sessions;

// Re-export all public types at module level so consumers use `crate::state::*`
pub use app_state::AppState;
pub use dialog::{
    AddProjectDialogField, AddProjectFormState, CreateDialogField, CreateFormState, DialogState,
};
pub use errors::OperationError;
