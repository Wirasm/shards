use crate::errors::KildError;

#[derive(Debug, thiserror::Error)]
pub enum ForgeError {
    #[error("No forge backend available for this repository")]
    NoForgeAvailable,

    #[error("Forge CLI '{cli}' not found or not executable")]
    CliNotFound { cli: String },

    #[error("Forge CLI error: {message}")]
    CliError { message: String },

    #[error("Failed to parse forge response: {message}")]
    ParseError { message: String },

    #[error("IO error during forge operation: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },
}

impl KildError for ForgeError {
    fn error_code(&self) -> &'static str {
        match self {
            ForgeError::NoForgeAvailable => "FORGE_NOT_AVAILABLE",
            ForgeError::CliNotFound { .. } => "FORGE_CLI_NOT_FOUND",
            ForgeError::CliError { .. } => "FORGE_CLI_ERROR",
            ForgeError::ParseError { .. } => "FORGE_PARSE_ERROR",
            ForgeError::IoError { .. } => "FORGE_IO_ERROR",
        }
    }

    fn is_user_error(&self) -> bool {
        matches!(
            self,
            ForgeError::NoForgeAvailable | ForgeError::CliNotFound { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_forge_available() {
        let error = ForgeError::NoForgeAvailable;
        assert_eq!(
            error.to_string(),
            "No forge backend available for this repository"
        );
        assert_eq!(error.error_code(), "FORGE_NOT_AVAILABLE");
        assert!(error.is_user_error());
    }

    #[test]
    fn test_cli_not_found() {
        let error = ForgeError::CliNotFound {
            cli: "gh".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Forge CLI 'gh' not found or not executable"
        );
        assert_eq!(error.error_code(), "FORGE_CLI_NOT_FOUND");
        assert!(error.is_user_error());
    }

    #[test]
    fn test_cli_error() {
        let error = ForgeError::CliError {
            message: "auth required".to_string(),
        };
        assert_eq!(error.to_string(), "Forge CLI error: auth required");
        assert_eq!(error.error_code(), "FORGE_CLI_ERROR");
        assert!(!error.is_user_error());
    }

    #[test]
    fn test_parse_error() {
        let error = ForgeError::ParseError {
            message: "invalid JSON".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Failed to parse forge response: invalid JSON"
        );
        assert_eq!(error.error_code(), "FORGE_PARSE_ERROR");
        assert!(!error.is_user_error());
    }

    #[test]
    fn test_io_error() {
        let error = ForgeError::IoError {
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
        };
        assert!(error.to_string().contains("IO error"));
        assert_eq!(error.error_code(), "FORGE_IO_ERROR");
        assert!(!error.is_user_error());
    }
}
