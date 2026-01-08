use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardSession {
    pub name: String,
    pub worktree_path: PathBuf,
    pub agent_command: Vec<String>,
    pub created_at: SystemTime,
    pub status: SessionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    Active,
    Stopped,
}

pub struct SessionRegistry {
    registry_path: PathBuf,
}

impl SessionRegistry {
    pub fn new() -> Result<Self> {
        let home_dir = dirs::home_dir().context("Could not find home directory")?;
        let registry_path = home_dir.join(".shards").join("registry.json");
        
        // Create directory if it doesn't exist
        if let Some(parent) = registry_path.parent() {
            fs::create_dir_all(parent)?;
        }

        Ok(Self { registry_path })
    }

    pub fn load_sessions(&self) -> Result<HashMap<String, ShardSession>> {
        if !self.registry_path.exists() {
            return Ok(HashMap::new());
        }

        let content = fs::read_to_string(&self.registry_path)?;
        let sessions: HashMap<String, ShardSession> = serde_json::from_str(&content)
            .unwrap_or_default();
        
        Ok(sessions)
    }

    pub fn save_sessions(&self, sessions: &HashMap<String, ShardSession>) -> Result<()> {
        let content = serde_json::to_string_pretty(sessions)?;
        fs::write(&self.registry_path, content)?;
        Ok(())
    }

    pub fn add_session(&self, session: ShardSession) -> Result<()> {
        let mut sessions = self.load_sessions()?;
        sessions.insert(session.name.clone(), session);
        self.save_sessions(&sessions)
    }

    pub fn remove_session(&self, name: &str) -> Result<()> {
        let mut sessions = self.load_sessions()?;
        sessions.remove(name);
        self.save_sessions(&sessions)
    }

    pub fn update_session_status(&self, name: &str, status: SessionStatus) -> Result<()> {
        let mut sessions = self.load_sessions()?;
        if let Some(session) = sessions.get_mut(name) {
            session.status = status;
            self.save_sessions(&sessions)?;
        }
        Ok(())
    }

    pub fn list_sessions(&self) -> Result<Vec<ShardSession>> {
        let sessions = self.load_sessions()?;
        Ok(sessions.into_values().collect())
    }

    pub fn get_session(&self, name: &str) -> Result<Option<ShardSession>> {
        let sessions = self.load_sessions()?;
        Ok(sessions.get(name).cloned())
    }
}
