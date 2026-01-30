use super::OperationError;

/// UI-only state mutations that don't cross interface boundaries.
///
/// These represent dialog, selection, and error state changes within the UI.
/// Unlike `Command`, these are not serialized or sent externally.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum UICommand {
    // --- Dialog commands ---
    /// Open the create kild dialog.
    OpenCreateDialog,
    /// Open the confirm destroy dialog for a branch.
    OpenConfirmDialog { branch: String },
    /// Open the add project dialog.
    OpenAddProjectDialog,
    /// Close whichever dialog is currently open.
    CloseDialog,
    /// Set an error message on the current dialog.
    SetDialogError { message: String },

    // --- Selection commands ---
    /// Select a kild by its session ID.
    SelectKild { id: String },
    /// Clear the current kild selection.
    ClearSelection,

    // --- Operation error commands ---
    /// Set an error for a specific branch operation.
    SetError { branch: String, message: String },
    /// Dismiss the error for a specific branch.
    DismissError { branch: String },
    /// Set bulk operation errors (e.g., "open all" failures).
    SetBulkErrors { errors: Vec<OperationError> },
    /// Clear all bulk operation errors.
    ClearBulkErrors,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_command_variants_construct() {
        let commands: Vec<UICommand> = vec![
            UICommand::OpenCreateDialog,
            UICommand::OpenConfirmDialog {
                branch: "feature".to_string(),
            },
            UICommand::OpenAddProjectDialog,
            UICommand::CloseDialog,
            UICommand::SetDialogError {
                message: "error".to_string(),
            },
            UICommand::SelectKild {
                id: "session-id".to_string(),
            },
            UICommand::ClearSelection,
            UICommand::SetError {
                branch: "feature".to_string(),
                message: "failed".to_string(),
            },
            UICommand::DismissError {
                branch: "feature".to_string(),
            },
            UICommand::SetBulkErrors {
                errors: vec![OperationError {
                    branch: "b1".to_string(),
                    message: "err".to_string(),
                }],
            },
            UICommand::ClearBulkErrors,
        ];

        // All variants should construct without panicking
        assert_eq!(commands.len(), 11);
    }

    #[test]
    fn test_ui_command_is_debug() {
        let cmd = UICommand::SelectKild {
            id: "test".to_string(),
        };
        let debug_str = format!("{:?}", cmd);
        assert!(debug_str.contains("SelectKild"));
    }

    #[test]
    fn test_ui_command_is_clone() {
        let cmd = UICommand::SetError {
            branch: "feature".to_string(),
            message: "oops".to_string(),
        };
        let cloned = cmd.clone();
        let original_debug = format!("{:?}", cmd);
        let cloned_debug = format!("{:?}", cloned);
        assert_eq!(original_debug, cloned_debug);
    }
}
