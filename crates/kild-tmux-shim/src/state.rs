use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use nix::fcntl::{Flock, FlockArg};
use serde::{Deserialize, Serialize};

use crate::errors::ShimError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneRegistry {
    pub next_pane_id: u32,
    pub session_name: String,
    pub panes: HashMap<String, PaneEntry>,
    pub windows: HashMap<String, WindowEntry>,
    pub sessions: HashMap<String, SessionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneEntry {
    pub daemon_session_id: String,
    pub title: String,
    pub border_style: String,
    pub window_id: String,
    pub hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowEntry {
    pub name: String,
    pub pane_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntry {
    pub name: String,
    pub windows: Vec<String>,
}

pub fn state_dir(session_id: &str) -> PathBuf {
    dirs::home_dir()
        .expect("home directory not found")
        .join(".kild")
        .join("shim")
        .join(session_id)
}

fn lock_path(session_id: &str) -> PathBuf {
    state_dir(session_id).join("panes.lock")
}

fn panes_path(session_id: &str) -> PathBuf {
    state_dir(session_id).join("panes.json")
}

fn acquire_lock(session_id: &str) -> Result<Flock<fs::File>, ShimError> {
    let lock = lock_path(session_id);
    if let Some(parent) = lock.parent() {
        fs::create_dir_all(parent).map_err(|e| ShimError::StateError {
            message: format!(
                "failed to create state directory {}: {}",
                parent.display(),
                e
            ),
        })?;
    }
    let lock_file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock)
        .map_err(|e| ShimError::StateError {
            message: format!("failed to open lock file {}: {}", lock.display(), e),
        })?;

    Flock::lock(lock_file, FlockArg::LockExclusive).map_err(|(_, e)| ShimError::StateError {
        message: format!("failed to acquire lock: {}", e),
    })
}

pub fn load(session_id: &str) -> Result<PaneRegistry, ShimError> {
    let data_path = panes_path(session_id);
    let _lock = acquire_lock(session_id)?;

    let content = fs::read_to_string(&data_path).map_err(|e| ShimError::StateError {
        message: format!("failed to read {}: {}", data_path.display(), e),
    })?;

    let registry: PaneRegistry =
        serde_json::from_str(&content).map_err(|e| ShimError::StateError {
            message: format!("failed to parse pane registry: {}", e),
        })?;

    // Lock released on _lock drop
    Ok(registry)
}

pub fn save(session_id: &str, registry: &PaneRegistry) -> Result<(), ShimError> {
    let data_path = panes_path(session_id);
    let _lock = acquire_lock(session_id)?;

    let content = serde_json::to_string_pretty(registry).map_err(|e| ShimError::StateError {
        message: format!("failed to serialize pane registry: {}", e),
    })?;

    let mut file = fs::File::create(&data_path).map_err(|e| ShimError::StateError {
        message: format!("failed to write {}: {}", data_path.display(), e),
    })?;
    file.write_all(content.as_bytes())?;
    file.flush()?;

    // Lock released on _lock drop
    Ok(())
}

pub fn allocate_pane_id(registry: &mut PaneRegistry) -> String {
    let id = format!("%{}", registry.next_pane_id);
    registry.next_pane_id += 1;
    id
}

#[allow(dead_code)]
pub fn init_registry(session_id: &str, daemon_session_id: &str) -> Result<(), ShimError> {
    let dir = state_dir(session_id);
    fs::create_dir_all(&dir)?;

    // Create lock file
    let lock = lock_path(session_id);
    fs::File::create(&lock)?;

    let mut panes = HashMap::new();
    panes.insert(
        "%0".to_string(),
        PaneEntry {
            daemon_session_id: daemon_session_id.to_string(),
            title: String::new(),
            border_style: String::new(),
            window_id: "0".to_string(),
            hidden: false,
        },
    );

    let mut windows = HashMap::new();
    windows.insert(
        "0".to_string(),
        WindowEntry {
            name: "main".to_string(),
            pane_ids: vec!["%0".to_string()],
        },
    );

    let mut sessions = HashMap::new();
    sessions.insert(
        "kild_0".to_string(),
        SessionEntry {
            name: "kild_0".to_string(),
            windows: vec!["0".to_string()],
        },
    );

    let registry = PaneRegistry {
        next_pane_id: 1,
        session_name: "kild_0".to_string(),
        panes,
        windows,
        sessions,
    };

    save(session_id, &registry)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_allocate_pane_id() {
        let mut registry = PaneRegistry {
            next_pane_id: 3,
            session_name: "test".to_string(),
            panes: HashMap::new(),
            windows: HashMap::new(),
            sessions: HashMap::new(),
        };

        assert_eq!(allocate_pane_id(&mut registry), "%3");
        assert_eq!(registry.next_pane_id, 4);
        assert_eq!(allocate_pane_id(&mut registry), "%4");
        assert_eq!(registry.next_pane_id, 5);
    }

    #[test]
    fn test_init_and_load_registry() {
        let test_id = format!("test-{}", uuid::Uuid::new_v4());
        let dir = state_dir(&test_id);

        init_registry(&test_id, "daemon-abc-123").unwrap();

        let registry = load(&test_id).unwrap();
        assert_eq!(registry.next_pane_id, 1);
        assert_eq!(registry.session_name, "kild_0");
        assert_eq!(registry.panes.len(), 1);
        assert_eq!(registry.panes["%0"].daemon_session_id, "daemon-abc-123");
        assert_eq!(registry.windows.len(), 1);
        assert_eq!(registry.sessions.len(), 1);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let test_id = format!("test-{}", uuid::Uuid::new_v4());
        let dir = state_dir(&test_id);
        fs::create_dir_all(&dir).unwrap();
        fs::File::create(lock_path(&test_id)).unwrap();

        let mut registry = PaneRegistry {
            next_pane_id: 2,
            session_name: "kild_0".to_string(),
            panes: HashMap::new(),
            windows: HashMap::new(),
            sessions: HashMap::new(),
        };
        registry.panes.insert(
            "%0".to_string(),
            PaneEntry {
                daemon_session_id: "d-1".to_string(),
                title: "main".to_string(),
                border_style: String::new(),
                window_id: "0".to_string(),
                hidden: false,
            },
        );
        registry.panes.insert(
            "%1".to_string(),
            PaneEntry {
                daemon_session_id: "d-2".to_string(),
                title: "worker".to_string(),
                border_style: "fg=blue".to_string(),
                window_id: "0".to_string(),
                hidden: false,
            },
        );

        save(&test_id, &registry).unwrap();
        let loaded = load(&test_id).unwrap();

        assert_eq!(loaded.next_pane_id, 2);
        assert_eq!(loaded.panes.len(), 2);
        assert_eq!(loaded.panes["%1"].title, "worker");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_state_dir_path() {
        let dir = state_dir("my-session");
        assert!(dir.ends_with(".kild/shim/my-session"));
    }

    #[test]
    fn test_load_invalid_json() {
        let test_id = format!("test-{}", uuid::Uuid::new_v4());
        let dir = state_dir(&test_id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("panes.json"), "not valid json{{{").unwrap();

        let result = load(&test_id);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("failed to parse pane registry"),
            "got: {}",
            err
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_load_missing_panes_file() {
        let test_id = format!("test-{}", uuid::Uuid::new_v4());
        let dir = state_dir(&test_id);
        fs::create_dir_all(&dir).unwrap();
        // Lock file exists but no panes.json

        let result = load(&test_id);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("failed to read"), "got: {}", err);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_load_empty_json_file() {
        let test_id = format!("test-{}", uuid::Uuid::new_v4());
        let dir = state_dir(&test_id);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("panes.json"), "").unwrap();

        let result = load(&test_id);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("failed to parse pane registry"),
            "got: {}",
            err
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_load_partial_json() {
        let test_id = format!("test-{}", uuid::Uuid::new_v4());
        let dir = state_dir(&test_id);
        fs::create_dir_all(&dir).unwrap();
        // Valid JSON but missing required fields
        fs::write(dir.join("panes.json"), r#"{"next_pane_id": 1}"#).unwrap();

        let result = load(&test_id);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("failed to parse pane registry"),
            "got: {}",
            err
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_save_and_load_without_pre_created_lock() {
        let test_id = format!("test-{}", uuid::Uuid::new_v4());
        let dir = state_dir(&test_id);
        // Don't create dir or lock file — acquire_lock should handle it

        let registry = PaneRegistry {
            next_pane_id: 1,
            session_name: "kild_0".to_string(),
            panes: HashMap::new(),
            windows: HashMap::new(),
            sessions: HashMap::new(),
        };

        // save should succeed because acquire_lock now creates the lock file on-demand
        save(&test_id, &registry).unwrap();

        let loaded = load(&test_id).unwrap();
        assert_eq!(loaded.next_pane_id, 1);
        assert_eq!(loaded.session_name, "kild_0");

        fs::remove_dir_all(&dir).ok();
    }
}
