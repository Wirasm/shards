//! Agent registry for managing and looking up agent backends.

use std::collections::HashMap;
use std::sync::LazyLock;

use super::backends::{AetherBackend, ClaudeBackend, CodexBackend, GeminiBackend, KiroBackend};
use super::traits::AgentBackend;

/// Global registry of all supported agent backends.
static REGISTRY: LazyLock<AgentRegistry> = LazyLock::new(AgentRegistry::new);

/// Registry that manages all agent backend implementations.
pub struct AgentRegistry {
    backends: HashMap<&'static str, Box<dyn AgentBackend>>,
}

impl AgentRegistry {
    fn new() -> Self {
        let mut backends: HashMap<&'static str, Box<dyn AgentBackend>> = HashMap::new();
        backends.insert("claude", Box::new(ClaudeBackend));
        backends.insert("kiro", Box::new(KiroBackend));
        backends.insert("gemini", Box::new(GeminiBackend));
        backends.insert("codex", Box::new(CodexBackend));
        backends.insert("aether", Box::new(AetherBackend));
        Self { backends }
    }

    /// Get a reference to an agent backend by name.
    pub fn get(&self, name: &str) -> Option<&dyn AgentBackend> {
        self.backends.get(name).map(|b| b.as_ref())
    }

    /// Check if an agent name is valid/supported.
    pub fn is_valid_agent(&self, name: &str) -> bool {
        self.backends.contains_key(name)
    }

    /// Get all valid agent names.
    pub fn valid_agent_names(&self) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = self.backends.keys().copied().collect();
        names.sort();
        names
    }

    /// Get the default agent name.
    pub fn default_agent(&self) -> &'static str {
        "claude"
    }
}

/// Get a reference to an agent backend by name.
pub fn get_agent(name: &str) -> Option<&'static dyn AgentBackend> {
    REGISTRY.get(name)
}

/// Check if an agent name is valid/supported.
pub fn is_valid_agent(name: &str) -> bool {
    REGISTRY.is_valid_agent(name)
}

/// Get all valid agent names.
pub fn valid_agent_names() -> Vec<&'static str> {
    REGISTRY.valid_agent_names()
}

/// Get the default agent name.
pub fn default_agent_name() -> &'static str {
    REGISTRY.default_agent()
}

/// Get the default command for an agent by name.
pub fn get_default_command(name: &str) -> Option<&'static str> {
    get_agent(name).map(|backend| backend.default_command())
}

/// Get process patterns for an agent by name.
pub fn get_process_patterns(name: &str) -> Option<Vec<String>> {
    get_agent(name).map(|backend| backend.process_patterns())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_agent_known() {
        let backend = get_agent("claude");
        assert!(backend.is_some());
        assert_eq!(backend.unwrap().name(), "claude");

        let backend = get_agent("kiro");
        assert!(backend.is_some());
        assert_eq!(backend.unwrap().name(), "kiro");
    }

    #[test]
    fn test_get_agent_unknown() {
        assert!(get_agent("unknown").is_none());
        assert!(get_agent("").is_none());
    }

    #[test]
    fn test_is_valid_agent() {
        assert!(is_valid_agent("claude"));
        assert!(is_valid_agent("kiro"));
        assert!(is_valid_agent("gemini"));
        assert!(is_valid_agent("codex"));
        assert!(is_valid_agent("aether"));

        assert!(!is_valid_agent("unknown"));
        assert!(!is_valid_agent(""));
        assert!(!is_valid_agent("Claude")); // Case-sensitive
    }

    #[test]
    fn test_valid_agent_names() {
        let names = valid_agent_names();
        assert_eq!(names.len(), 5);
        assert!(names.contains(&"claude"));
        assert!(names.contains(&"kiro"));
        assert!(names.contains(&"gemini"));
        assert!(names.contains(&"codex"));
        assert!(names.contains(&"aether"));
    }

    #[test]
    fn test_default_agent_name() {
        assert_eq!(default_agent_name(), "claude");
    }

    #[test]
    fn test_get_default_command() {
        assert_eq!(get_default_command("claude"), Some("claude"));
        assert_eq!(get_default_command("kiro"), Some("kiro-cli chat"));
        assert_eq!(get_default_command("gemini"), Some("gemini"));
        assert_eq!(get_default_command("codex"), Some("codex"));
        assert_eq!(get_default_command("aether"), Some("aether"));
        assert_eq!(get_default_command("unknown"), None);
    }

    #[test]
    fn test_get_process_patterns() {
        let claude_patterns = get_process_patterns("claude");
        assert!(claude_patterns.is_some());
        let patterns = claude_patterns.unwrap();
        assert!(patterns.contains(&"claude".to_string()));
        assert!(patterns.contains(&"claude-code".to_string()));

        let kiro_patterns = get_process_patterns("kiro");
        assert!(kiro_patterns.is_some());
        let patterns = kiro_patterns.unwrap();
        assert!(patterns.contains(&"kiro-cli".to_string()));
        assert!(patterns.contains(&"kiro".to_string()));

        assert!(get_process_patterns("unknown").is_none());
    }

    #[test]
    fn test_registry_contains_all_agents() {
        // Ensure all expected agents are registered
        let expected_agents = ["claude", "kiro", "gemini", "codex", "aether"];
        for agent in expected_agents {
            assert!(
                is_valid_agent(agent),
                "Registry should contain agent: {}",
                agent
            );
        }
    }
}
