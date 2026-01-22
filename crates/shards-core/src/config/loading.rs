//! Configuration loading and merging logic.
//!
//! This module handles loading configuration from files and merging
//! configurations from different sources (user config, project config).
//!
//! # Configuration Hierarchy
//!
//! Configuration is loaded in the following order (later sources override earlier ones):
//! 1. **Hardcoded defaults** - Built-in fallback values
//! 2. **User config** - `~/.shards/config.toml` (global user preferences)
//! 3. **Project config** - `./shards/config.toml` (project-specific overrides)
//! 4. **CLI arguments** - Command-line flags (highest priority)

use crate::agents;
use crate::config::types::{AgentConfig, HealthConfig, ShardsConfig, TerminalConfig};
use crate::config::validation::validate_config;
use std::fs;
use std::path::PathBuf;

/// Load configuration from the hierarchy of config files.
///
/// Loads and merges configuration from:
/// 1. Default values
/// 2. User config (`~/.shards/config.toml`)
/// 3. Project config (`./shards/config.toml`)
///
/// # Errors
///
/// Returns an error if validation fails. Missing config files are not errors.
pub fn load_hierarchy() -> Result<ShardsConfig, Box<dyn std::error::Error>> {
    let mut config = ShardsConfig::default();

    // Load user config
    match load_user_config() {
        Ok(user_config) => {
            config = merge_configs(config, user_config);
        }
        Err(e) => {
            // Check if this is a "file not found" error (expected) vs other errors (should warn)
            let is_not_found = e
                .downcast_ref::<std::io::Error>()
                .is_some_and(|io_err| io_err.kind() == std::io::ErrorKind::NotFound);

            if !is_not_found {
                tracing::warn!(
                    event = "config.user_config_load_failed",
                    error = %e,
                    "User config file exists but could not be loaded - using defaults"
                );
            }
        }
    }

    // Load project config
    match load_project_config() {
        Ok(project_config) => {
            config = merge_configs(config, project_config);
        }
        Err(e) => {
            // Check if this is a "file not found" error (expected) vs other errors (should warn)
            let is_not_found = e
                .downcast_ref::<std::io::Error>()
                .is_some_and(|io_err| io_err.kind() == std::io::ErrorKind::NotFound);

            if !is_not_found {
                tracing::warn!(
                    event = "config.project_config_load_failed",
                    error = %e,
                    "Project config file exists but could not be loaded - using defaults"
                );
            }
        }
    }

    // Validate the final configuration
    validate_config(&config)?;

    Ok(config)
}

/// Load the user configuration from ~/.shards/config.toml.
fn load_user_config() -> Result<ShardsConfig, Box<dyn std::error::Error>> {
    let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
    let config_path = home_dir.join(".shards").join("config.toml");
    load_config_file(&config_path)
}

/// Load the project configuration from ./shards/config.toml.
fn load_project_config() -> Result<ShardsConfig, Box<dyn std::error::Error>> {
    let config_path = std::env::current_dir()?.join("shards").join("config.toml");
    load_config_file(&config_path)
}

/// Load a configuration file from the given path.
fn load_config_file(path: &PathBuf) -> Result<ShardsConfig, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let config: ShardsConfig = toml::from_str(&content)?;
    Ok(config)
}

/// Merge two configurations, with override_config taking precedence.
///
/// For optional fields, override values replace base values only if present.
/// For collections (like agents HashMap), entries are merged with override taking precedence.
pub fn merge_configs(base: ShardsConfig, override_config: ShardsConfig) -> ShardsConfig {
    ShardsConfig {
        agent: AgentConfig {
            // Always use override agent if it was explicitly set in the config file
            // We can't distinguish between explicit "claude" and default "claude" here,
            // so we always prefer the override config's agent setting
            default: override_config.agent.default,
            startup_command: override_config
                .agent
                .startup_command
                .or(base.agent.startup_command),
            flags: override_config.agent.flags.or(base.agent.flags),
        },
        terminal: TerminalConfig {
            preferred: override_config
                .terminal
                .preferred
                .or(base.terminal.preferred),
            spawn_delay_ms: override_config.terminal.spawn_delay_ms,
            max_retry_attempts: override_config.terminal.max_retry_attempts,
        },
        agents: {
            let mut merged = base.agents;
            for (key, value) in override_config.agents {
                merged.insert(key, value);
            }
            merged
        },
        include_patterns: override_config.include_patterns.or(base.include_patterns),
        health: HealthConfig {
            idle_threshold_minutes: override_config
                .health
                .idle_threshold_minutes
                .or(base.health.idle_threshold_minutes),
            refresh_interval_secs: override_config
                .health
                .refresh_interval_secs
                .or(base.health.refresh_interval_secs),
            history_enabled: override_config.health.history_enabled || base.health.history_enabled,
            history_retention_days: override_config
                .health
                .history_retention_days
                .or(base.health.history_retention_days),
        },
    }
}

/// Get the command to run for a specific agent.
///
/// Resolution order:
/// 1. Agent-specific settings from `[agents.<name>]` section
/// 2. Global agent config from `[agent]` section
/// 3. Built-in default command for the agent
/// 4. Raw agent name as fallback
pub fn get_agent_command(config: &ShardsConfig, agent_name: &str) -> String {
    // Check agent-specific settings first
    if let Some(agent_settings) = config.agents.get(agent_name)
        && let Some(command) = &agent_settings.startup_command
    {
        let mut full_command = command.clone();
        if let Some(flags) = &agent_settings.flags {
            full_command.push(' ');
            full_command.push_str(flags);
        }
        return full_command;
    }

    // Fall back to global agent config
    let base_command = config.agent.startup_command.as_deref().unwrap_or_else(|| {
        match agents::get_default_command(agent_name) {
            Some(cmd) => cmd,
            None => {
                tracing::warn!(
                    event = "config.agent_command_fallback",
                    agent = agent_name,
                    "No default command found for agent '{}', using raw name as command",
                    agent_name
                );
                agent_name
            }
        }
    });

    let mut full_command = base_command.to_string();
    if let Some(flags) = &config.agent.flags {
        full_command.push(' ');
        full_command.push_str(flags);
    }

    full_command
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::AgentSettings;
    use std::env;
    use std::fs;

    #[test]
    fn test_get_agent_command_defaults() {
        let config = ShardsConfig::default();

        assert_eq!(get_agent_command(&config, "claude"), "claude");
        assert_eq!(get_agent_command(&config, "kiro"), "kiro-cli chat");
        assert_eq!(get_agent_command(&config, "gemini"), "gemini");
        assert_eq!(get_agent_command(&config, "unknown"), "unknown");
    }

    #[test]
    fn test_get_agent_command_with_flags() {
        let mut config = ShardsConfig::default();
        config.agent.flags = Some("--yolo".to_string());

        assert_eq!(get_agent_command(&config, "claude"), "claude --yolo");
    }

    #[test]
    fn test_get_agent_command_specific_agent() {
        let mut config = ShardsConfig::default();
        let agent_settings = AgentSettings {
            startup_command: Some("cc".to_string()),
            flags: Some("--dangerous".to_string()),
        };
        config.agents.insert("claude".to_string(), agent_settings);

        assert_eq!(get_agent_command(&config, "claude"), "cc --dangerous");
        assert_eq!(get_agent_command(&config, "kiro"), "kiro-cli chat");
    }

    #[test]
    fn test_config_hierarchy_integration() {
        // Create temporary directories for testing
        let temp_dir = env::temp_dir().join("shards_config_test");
        let user_config_dir = temp_dir.join("user");
        let project_config_dir = temp_dir.join("project");

        // Clean up any existing test directories
        let _ = fs::remove_dir_all(&temp_dir);

        // Create test directories
        fs::create_dir_all(&user_config_dir).unwrap();
        fs::create_dir_all(&project_config_dir.join("shards")).unwrap();

        // Create user config
        let user_config_content = r#"
[agent]
default = "kiro"
startup_command = "kiro-cli chat"

[terminal]
preferred = "iterm2"
"#;
        fs::write(user_config_dir.join("config.toml"), user_config_content).unwrap();

        // Create project config that overrides some settings
        let project_config_content = r#"
[agent]
default = "claude"
flags = "--yolo"
"#;
        fs::write(
            project_config_dir.join("shards").join("config.toml"),
            project_config_content,
        )
        .unwrap();

        // Test loading user config
        let user_config = load_config_file(&user_config_dir.join("config.toml")).unwrap();
        assert_eq!(user_config.agent.default, "kiro");
        assert_eq!(
            user_config.agent.startup_command,
            Some("kiro-cli chat".to_string())
        );
        assert_eq!(user_config.terminal.preferred, Some("iterm2".to_string()));

        // Test loading project config
        let project_config =
            load_config_file(&project_config_dir.join("shards").join("config.toml")).unwrap();
        assert_eq!(project_config.agent.default, "claude");
        assert_eq!(project_config.agent.flags, Some("--yolo".to_string()));

        // Test merging configs (project overrides user)
        let merged = merge_configs(user_config, project_config);
        assert_eq!(merged.agent.default, "claude"); // Overridden by project
        assert_eq!(
            merged.agent.startup_command,
            Some("kiro-cli chat".to_string())
        ); // From user
        assert_eq!(merged.agent.flags, Some("--yolo".to_string())); // From project
        assert_eq!(merged.terminal.preferred, Some("iterm2".to_string())); // From user

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_toml_parsing_edge_cases() {
        // Test empty config
        let empty_config: ShardsConfig = toml::from_str("").unwrap();
        assert_eq!(empty_config.agent.default, "claude");

        // Test partial config
        let partial_config: ShardsConfig = toml::from_str(
            r#"
[terminal]
preferred = "iterm2"
"#,
        )
        .unwrap();
        assert_eq!(partial_config.agent.default, "claude"); // Should use default
        assert_eq!(
            partial_config.terminal.preferred,
            Some("iterm2".to_string())
        );

        // Test invalid TOML should fail
        let invalid_result: Result<ShardsConfig, _> = toml::from_str("invalid toml [[[");
        assert!(invalid_result.is_err());
    }

    #[test]
    fn test_health_config_from_toml() {
        let config: ShardsConfig = toml::from_str(
            r#"
[health]
idle_threshold_minutes = 5
history_enabled = true
"#,
        )
        .unwrap();
        assert_eq!(config.health.idle_threshold_minutes(), 5);
        assert!(config.health.history_enabled);
        // Defaults should still apply for unspecified fields
        assert_eq!(config.health.refresh_interval_secs(), 5);
        assert_eq!(config.health.history_retention_days(), 7);
    }

    #[test]
    fn test_health_config_merge() {
        let user_config: ShardsConfig = toml::from_str(
            r#"
[health]
idle_threshold_minutes = 15
history_retention_days = 30
"#,
        )
        .unwrap();

        // Project config with only history_enabled set
        let project_config: ShardsConfig = toml::from_str(
            r#"
[health]
history_enabled = true
"#,
        )
        .unwrap();

        let merged = merge_configs(user_config, project_config);

        // User-set values should be preserved when project doesn't override
        assert_eq!(merged.health.idle_threshold_minutes(), 15);
        assert_eq!(merged.health.history_retention_days(), 30);
        // Project-set values should be used
        assert!(merged.health.history_enabled);
    }
}
