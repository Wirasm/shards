//! Agent-specific error types.

use crate::agents::supported_agents_string;
use crate::errors::KildError;

/// Errors that can occur during agent operations.
#[derive(Debug)]
pub enum AgentError {
    UnknownAgent { name: String },
    AgentNotAvailable { name: String },
}

impl std::fmt::Display for AgentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentError::UnknownAgent { name } => {
                write!(
                    f,
                    "Unknown agent '{}'. Supported: {}",
                    name,
                    supported_agents_string()
                )
            }
            AgentError::AgentNotAvailable { name } => {
                write!(f, "Agent '{}' CLI is not installed or not in PATH", name)
            }
        }
    }
}

impl std::error::Error for AgentError {}

impl KildError for AgentError {
    fn error_code(&self) -> &'static str {
        match self {
            AgentError::UnknownAgent { .. } => "UNKNOWN_AGENT",
            AgentError::AgentNotAvailable { .. } => "AGENT_NOT_AVAILABLE",
        }
    }

    fn is_user_error(&self) -> bool {
        matches!(
            self,
            AgentError::UnknownAgent { .. } | AgentError::AgentNotAvailable { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unknown_agent_error_display() {
        let error = AgentError::UnknownAgent {
            name: "unknown".to_string(),
        };
        let msg = error.to_string();
        // Verify message format
        assert!(msg.starts_with("Unknown agent 'unknown'. Supported: "));
        // Verify all valid agents are listed
        assert!(msg.contains("amp"), "Error should list amp");
        assert!(msg.contains("claude"), "Error should list claude");
        assert!(msg.contains("kiro"), "Error should list kiro");
        assert!(msg.contains("gemini"), "Error should list gemini");
        assert!(msg.contains("codex"), "Error should list codex");
        // Verify removed agents are NOT listed
        assert!(
            !msg.contains("aether"),
            "Error should NOT list removed agent aether"
        );
        // Verify error trait methods
        assert_eq!(error.error_code(), "UNKNOWN_AGENT");
        assert!(error.is_user_error());
    }

    #[test]
    fn test_agent_not_available_error_display() {
        let error = AgentError::AgentNotAvailable {
            name: "claude".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Agent 'claude' CLI is not installed or not in PATH"
        );
        assert_eq!(error.error_code(), "AGENT_NOT_AVAILABLE");
        assert!(error.is_user_error());
    }
}
