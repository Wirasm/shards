//! Application state for shards-ui.
//!
//! Centralized state management for the GUI, including shard list,
//! create dialog, and form state.

use shards_core::Session;

/// Process status for a shard, distinguishing between running, stopped, and unknown states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessStatus {
    /// Process is confirmed running
    Running,
    /// Process is confirmed stopped (or no PID exists)
    Stopped,
    /// Could not determine status (process check failed)
    Unknown,
}

/// Display data for a shard, combining Session with computed process status.
#[derive(Clone)]
pub struct ShardDisplay {
    pub session: Session,
    pub status: ProcessStatus,
}

impl ShardDisplay {
    pub fn from_session(session: Session) -> Self {
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
}

/// Form state for creating a new shard.
#[derive(Clone, Debug)]
pub struct CreateFormState {
    pub branch_name: String,
    pub selected_agent: String,
    pub selected_agent_index: usize,
}

impl Default for CreateFormState {
    fn default() -> Self {
        let agents = shards_core::agents::valid_agent_names();
        let default_agent = shards_core::agents::default_agent_name();

        if agents.is_empty() {
            tracing::error!(
                event = "ui.create_form.no_agents_available",
                "Agent list is empty - using hardcoded fallback"
            );
            return Self {
                branch_name: String::new(),
                selected_agent: default_agent.to_string(),
                selected_agent_index: 0,
            };
        }

        let index = agents
            .iter()
            .position(|&a| a == default_agent)
            .unwrap_or_else(|| {
                tracing::info!(
                    event = "ui.create_form.default_agent_not_found",
                    default = default_agent,
                    selected = agents[0],
                    "Default agent not in list, using first available"
                );
                0
            });

        Self {
            branch_name: String::new(),
            selected_agent: agents[index].to_string(),
            selected_agent_index: index,
        }
    }
}

/// Main application state.
pub struct AppState {
    pub displays: Vec<ShardDisplay>,
    pub load_error: Option<String>,
    pub show_create_dialog: bool,
    pub create_form: CreateFormState,
    pub create_error: Option<String>,

    // Confirm dialog state
    pub show_confirm_dialog: bool,
    pub confirm_target_branch: Option<String>,
    pub confirm_error: Option<String>,

    // Relaunch error state (shown inline per-row)
    pub relaunch_error: Option<(String, String)>, // (branch, error_message)
}

impl AppState {
    /// Create new application state, loading sessions from disk.
    pub fn new() -> Self {
        let (displays, load_error) = crate::actions::refresh_sessions();

        Self {
            displays,
            load_error,
            show_create_dialog: false,
            create_form: CreateFormState::default(),
            create_error: None,
            show_confirm_dialog: false,
            confirm_target_branch: None,
            confirm_error: None,
            relaunch_error: None,
        }
    }

    /// Refresh sessions from disk.
    pub fn refresh_sessions(&mut self) {
        let (displays, load_error) = crate::actions::refresh_sessions();
        self.displays = displays;
        self.load_error = load_error;
    }

    /// Reset the create form to default state.
    pub fn reset_create_form(&mut self) {
        self.create_form = CreateFormState::default();
        self.create_error = None;
    }

    /// Reset the confirm dialog to default state.
    pub fn reset_confirm_dialog(&mut self) {
        self.show_confirm_dialog = false;
        self.confirm_target_branch = None;
        self.confirm_error = None;
    }

    /// Clear any relaunch error.
    pub fn clear_relaunch_error(&mut self) {
        self.relaunch_error = None;
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reset_confirm_dialog_clears_all_fields() {
        // Create state with confirm dialog open and an error
        let mut state = AppState {
            displays: Vec::new(),
            load_error: None,
            show_create_dialog: false,
            create_form: CreateFormState::default(),
            create_error: None,
            show_confirm_dialog: true,
            confirm_target_branch: Some("feature-branch".to_string()),
            confirm_error: Some("Some error".to_string()),
            relaunch_error: None,
        };

        state.reset_confirm_dialog();

        assert!(!state.show_confirm_dialog);
        assert!(state.confirm_target_branch.is_none());
        assert!(state.confirm_error.is_none());
    }

    #[test]
    fn test_clear_relaunch_error() {
        let mut state = AppState {
            displays: Vec::new(),
            load_error: None,
            show_create_dialog: false,
            create_form: CreateFormState::default(),
            create_error: None,
            show_confirm_dialog: false,
            confirm_target_branch: None,
            confirm_error: None,
            relaunch_error: Some(("branch".to_string(), "error".to_string())),
        };

        state.clear_relaunch_error();

        assert!(state.relaunch_error.is_none());
    }

    #[test]
    fn test_process_status_from_session_no_pid() {
        use shards_core::sessions::types::SessionStatus;
        use std::path::PathBuf;

        let session = Session {
            id: "test-id".to_string(),
            branch: "test-branch".to_string(),
            worktree_path: PathBuf::from("/tmp/test"),
            agent: "claude".to_string(),
            project_id: "test-project".to_string(),
            status: SessionStatus::Active,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            port_range_start: 0,
            port_range_end: 0,
            port_count: 0,
            process_id: None,
            process_name: None,
            process_start_time: None,
            terminal_type: None,
            terminal_window_id: None,
            command: String::new(),
            last_activity: None,
        };

        let display = ShardDisplay::from_session(session);
        assert_eq!(display.status, ProcessStatus::Stopped);
    }
}
