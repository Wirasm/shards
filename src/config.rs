use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_terminal")]
    pub terminal: String,
    #[serde(default)]
    pub default_agent: String,
    #[serde(default)]
    pub agents: HashMap<String, String>,
}

fn default_terminal() -> String {
    "auto".to_string()
}

impl Default for Config {
    fn default() -> Self {
        let mut agents = HashMap::new();
        agents.insert("kiro".to_string(), "kiro-cli chat".to_string());
        agents.insert("claude".to_string(), "claude-code".to_string());
        agents.insert("gemini".to_string(), "gemini-cli".to_string());

        Self {
            terminal: "auto".to_string(),
            default_agent: String::new(),
            agents,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let home_dir = dirs::home_dir().context("Could not find home directory")?;
        let config_path = home_dir.join(".shards").join("config.toml");
        
        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&content)
            .context("Failed to parse config.toml")?;
        
        Ok(config)
    }
}
