use std::collections::HashMap;

use serde::Deserialize;

/// Per-pane teammate info discovered from the shim registry.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TeammatePane {
    pub pane_id: String,
    pub daemon_session_id: String,
    pub title: String,
    pub is_leader: bool,
}

/// Manages teammate panes per kild session.
#[allow(dead_code)]
pub struct TeammateStore {
    teammates: HashMap<String, Vec<TeammatePane>>,
}

// Local deserialization types matching the shim pane registry format.
// Intentionally decoupled from kild-tmux-shim crate.

#[derive(Deserialize)]
struct PaneRegistry {
    panes: HashMap<String, PaneEntry>,
}

#[derive(Deserialize)]
struct PaneEntry {
    daemon_session_id: String,
    #[serde(default)]
    title: String,
}

#[allow(dead_code)]
impl TeammateStore {
    pub fn new() -> Self {
        Self {
            teammates: HashMap::new(),
        }
    }

    /// Read the shim pane registry for a session and return teammate panes.
    ///
    /// Returns empty vec if the file is missing, locked, or contains invalid JSON.
    pub fn load_from_registry(kild_session_id: &str) -> Vec<TeammatePane> {
        let Some(home) = dirs::home_dir() else {
            return Vec::new();
        };

        let path = home
            .join(".kild")
            .join("shim")
            .join(kild_session_id)
            .join("panes.json");

        let contents = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        let registry: PaneRegistry = match serde_json::from_str(&contents) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };

        let mut panes: Vec<TeammatePane> = registry
            .panes
            .into_iter()
            .map(|(pane_id, entry)| TeammatePane {
                is_leader: pane_id == "%0",
                pane_id,
                daemon_session_id: entry.daemon_session_id,
                title: entry.title,
            })
            .collect();

        // Sort by pane_id for deterministic order
        panes.sort_by(|a, b| a.pane_id.cmp(&b.pane_id));
        panes
    }

    /// Reload teammate panes from disk for a session, updating the cache.
    pub fn refresh_teammates(&mut self, kild_session_id: &str) {
        let panes = Self::load_from_registry(kild_session_id);
        self.teammates.insert(kild_session_id.to_string(), panes);
    }

    /// Get cached teammate panes for a kild session.
    pub fn get_teammates(&self, kild_id: &str) -> &[TeammatePane] {
        self.teammates
            .get(kild_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Check if a kild session has any cached teammates.
    pub fn has_teammates(&self, kild_id: &str) -> bool {
        self.teammates.get(kild_id).is_some_and(|v| !v.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_store_is_empty() {
        let store = TeammateStore::new();
        assert!(!store.has_teammates("any-id"));
        assert!(store.get_teammates("any-id").is_empty());
    }

    #[test]
    fn test_get_teammates_returns_empty_for_unknown() {
        let store = TeammateStore::new();
        let teammates = store.get_teammates("nonexistent");
        assert!(teammates.is_empty());
    }

    #[test]
    fn test_has_teammates_false_for_empty_vec() {
        let mut store = TeammateStore::new();
        store.teammates.insert("k1".to_string(), Vec::new());
        assert!(!store.has_teammates("k1"));
    }

    #[test]
    fn test_has_teammates_true_when_populated() {
        let mut store = TeammateStore::new();
        store.teammates.insert(
            "k1".to_string(),
            vec![TeammatePane {
                pane_id: "%0".to_string(),
                daemon_session_id: "d-1".to_string(),
                title: "leader".to_string(),
                is_leader: true,
            }],
        );
        assert!(store.has_teammates("k1"));
    }

    #[test]
    fn test_get_teammates_returns_cached_data() {
        let mut store = TeammateStore::new();
        store.teammates.insert(
            "k1".to_string(),
            vec![
                TeammatePane {
                    pane_id: "%0".to_string(),
                    daemon_session_id: "d-1".to_string(),
                    title: "".to_string(),
                    is_leader: true,
                },
                TeammatePane {
                    pane_id: "%1".to_string(),
                    daemon_session_id: "d-2".to_string(),
                    title: "worker".to_string(),
                    is_leader: false,
                },
            ],
        );

        let teammates = store.get_teammates("k1");
        assert_eq!(teammates.len(), 2);
        assert!(teammates[0].is_leader);
        assert!(!teammates[1].is_leader);
        assert_eq!(teammates[1].title, "worker");
    }

    #[test]
    fn test_load_from_registry_returns_empty_for_missing_session() {
        // Uses a session ID that won't have a real file
        let panes = TeammateStore::load_from_registry("nonexistent-test-session-12345");
        assert!(panes.is_empty());
    }

    #[test]
    fn test_refresh_updates_cache() {
        let mut store = TeammateStore::new();

        // Pre-populate with some data
        store.teammates.insert(
            "k1".to_string(),
            vec![TeammatePane {
                pane_id: "%0".to_string(),
                daemon_session_id: "d-old".to_string(),
                title: "old".to_string(),
                is_leader: true,
            }],
        );

        // Refresh with nonexistent session replaces with empty
        store.refresh_teammates("k1");
        assert!(!store.has_teammates("k1"));
    }

    #[test]
    fn test_deserialize_pane_registry() {
        let json = r#"{
            "next_pane_id": 2,
            "session_name": "kild_0",
            "panes": {
                "%0": { "daemon_session_id": "d-1", "title": "", "border_style": "", "window_id": "0", "hidden": false },
                "%1": { "daemon_session_id": "d-2", "title": "worker", "border_style": "", "window_id": "0", "hidden": false }
            },
            "windows": {},
            "sessions": {}
        }"#;

        let registry: PaneRegistry = serde_json::from_str(json).unwrap();
        assert_eq!(registry.panes.len(), 2);
        assert_eq!(registry.panes["%0"].daemon_session_id, "d-1");
        assert_eq!(registry.panes["%1"].title, "worker");
    }

    #[test]
    fn test_leader_detection() {
        let json = r#"{
            "panes": {
                "%0": { "daemon_session_id": "d-1", "title": "" },
                "%1": { "daemon_session_id": "d-2", "title": "worker" }
            }
        }"#;

        let registry: PaneRegistry = serde_json::from_str(json).unwrap();
        let mut panes: Vec<TeammatePane> = registry
            .panes
            .into_iter()
            .map(|(pane_id, entry)| TeammatePane {
                is_leader: pane_id == "%0",
                pane_id,
                daemon_session_id: entry.daemon_session_id,
                title: entry.title,
            })
            .collect();
        panes.sort_by(|a, b| a.pane_id.cmp(&b.pane_id));

        assert_eq!(panes.len(), 2);
        assert!(panes[0].is_leader);
        assert_eq!(panes[0].pane_id, "%0");
        assert!(!panes[1].is_leader);
        assert_eq!(panes[1].pane_id, "%1");
    }
}
