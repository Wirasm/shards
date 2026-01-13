use crate::core::errors::ShardsError;

#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("Process '{pid}' not found")]
    NotFound { pid: u32 },

    #[error("Failed to kill process '{pid}': {message}")]
    KillFailed { pid: u32, message: String },

    #[error("Access denied for process '{pid}'")]
    AccessDenied { pid: u32 },

    #[error("System error: {message}")]
    SystemError { message: String },
}

impl ShardsError for ProcessError {
    fn error_code(&self) -> &'static str {
        match self {
            ProcessError::NotFound { .. } => "PROCESS_NOT_FOUND",
            ProcessError::KillFailed { .. } => "PROCESS_KILL_FAILED",
            ProcessError::AccessDenied { .. } => "PROCESS_ACCESS_DENIED",
            ProcessError::SystemError { .. } => "PROCESS_SYSTEM_ERROR",
        }
    }

    fn is_user_error(&self) -> bool {
        matches!(
            self,
            ProcessError::NotFound { .. } | ProcessError::AccessDenied { .. }
        )
    }
}
