//! Amp agent backend implementation.

use crate::agents::traits::AgentBackend;

/// Backend implementation for Amp.
pub struct AmpBackend;

impl AgentBackend for AmpBackend {
    fn name(&self) -> &'static str {
        "amp"
    }

    fn display_name(&self) -> &'static str {
        "Amp"
    }

    fn is_available(&self) -> bool {
        which::which("amp").is_ok()
    }

    fn default_command(&self) -> &'static str {
        "amp"
    }

    fn process_patterns(&self) -> Vec<String> {
        vec!["amp".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amp_backend_name() {
        let backend = AmpBackend;
        assert_eq!(backend.name(), "amp");
    }

    #[test]
    fn test_amp_backend_display_name() {
        let backend = AmpBackend;
        assert_eq!(backend.display_name(), "Amp");
    }

    #[test]
    fn test_amp_backend_default_command() {
        let backend = AmpBackend;
        assert_eq!(backend.default_command(), "amp");
    }

    #[test]
    fn test_amp_backend_process_patterns() {
        let backend = AmpBackend;
        let patterns = backend.process_patterns();
        assert!(patterns.contains(&"amp".to_string()));
    }

    #[test]
    fn test_amp_backend_command_patterns() {
        let backend = AmpBackend;
        let patterns = backend.command_patterns();
        assert_eq!(patterns, vec!["amp".to_string()]);
    }
}
