use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Daemon-specific configuration.
///
/// Read from the `[daemon]` section of `~/.kild/config.toml`.
/// The daemon reads this itself; kild-core does not carry it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Path to the Unix domain socket.
    /// Default: `~/.kild/daemon.sock`
    #[serde(default = "default_socket_path")]
    pub socket_path: PathBuf,

    /// Path to the PID file.
    /// Default: `~/.kild/daemon.pid`
    #[serde(default = "default_pid_path")]
    pub pid_path: PathBuf,

    /// Per-session scrollback ring buffer size in bytes.
    /// Default: 65536 (64 KB)
    #[serde(default = "default_scrollback_buffer_size")]
    pub scrollback_buffer_size: usize,

    /// PTY output batching interval in milliseconds.
    /// Default: 4
    #[serde(default = "default_pty_output_batch_ms")]
    pub pty_output_batch_ms: u64,

    /// Per-client output buffer size before dropping oldest bytes.
    /// Default: 262144 (256 KB)
    #[serde(default = "default_client_buffer_size")]
    pub client_buffer_size: usize,

    /// Time in seconds to wait for agents to exit during shutdown.
    /// Default: 5
    #[serde(default = "default_shutdown_timeout_secs")]
    pub shutdown_timeout_secs: u64,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            socket_path: default_socket_path(),
            pid_path: default_pid_path(),
            scrollback_buffer_size: default_scrollback_buffer_size(),
            pty_output_batch_ms: default_pty_output_batch_ms(),
            client_buffer_size: default_client_buffer_size(),
            shutdown_timeout_secs: default_shutdown_timeout_secs(),
        }
    }
}

fn default_socket_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".kild")
        .join("daemon.sock")
}

fn default_pid_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".kild")
        .join("daemon.pid")
}

fn default_scrollback_buffer_size() -> usize {
    65536
}

fn default_pty_output_batch_ms() -> u64 {
    4
}

fn default_client_buffer_size() -> usize {
    262144
}

fn default_shutdown_timeout_secs() -> u64 {
    5
}

/// Runtime status of the daemon process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    /// Daemon process PID.
    pub pid: u32,
    /// Seconds since daemon started.
    pub uptime_secs: u64,
    /// Number of managed sessions.
    pub session_count: usize,
    /// Number of sessions with active PTYs.
    pub active_connections: usize,
}

/// Summary of a session as returned via IPC.
///
/// This is a wire type for the protocol, not the internal `DaemonSession`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub project_id: String,
    pub branch: String,
    pub worktree_path: String,
    pub agent: String,
    pub status: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pty_pid: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_config_defaults() {
        let config = DaemonConfig::default();
        assert!(config.socket_path.ends_with("daemon.sock"));
        assert_eq!(config.scrollback_buffer_size, 65536);
        assert_eq!(config.pty_output_batch_ms, 4);
        assert_eq!(config.client_buffer_size, 262144);
        assert_eq!(config.shutdown_timeout_secs, 5);
    }

    #[test]
    fn test_daemon_config_serde_roundtrip() {
        let config = DaemonConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: DaemonConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.scrollback_buffer_size, config.scrollback_buffer_size);
        assert_eq!(parsed.pty_output_batch_ms, config.pty_output_batch_ms);
        assert_eq!(parsed.client_buffer_size, config.client_buffer_size);
        assert_eq!(parsed.shutdown_timeout_secs, config.shutdown_timeout_secs);
    }

    #[test]
    fn test_daemon_status_serde_roundtrip() {
        let status = DaemonStatus {
            pid: 12345,
            uptime_secs: 3600,
            session_count: 3,
            active_connections: 2,
        };
        let json = serde_json::to_string(&status).unwrap();
        let parsed: DaemonStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.pid, 12345);
        assert_eq!(parsed.uptime_secs, 3600);
        assert_eq!(parsed.session_count, 3);
        assert_eq!(parsed.active_connections, 2);
    }

    #[test]
    fn test_session_info_serde() {
        let info = SessionInfo {
            id: "myapp_feature-auth".to_string(),
            project_id: "myapp".to_string(),
            branch: "feature-auth".to_string(),
            worktree_path: "/tmp/worktrees/feature-auth".to_string(),
            agent: "claude".to_string(),
            status: "running".to_string(),
            created_at: "2026-02-09T14:30:00Z".to_string(),
            note: Some("OAuth2 implementation".to_string()),
            client_count: Some(2),
            pty_pid: Some(12345),
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: SessionInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, info.id);
        assert_eq!(parsed.note, info.note);
        assert_eq!(parsed.client_count, Some(2));
    }

    #[test]
    fn test_session_info_optional_fields_omitted() {
        let info = SessionInfo {
            id: "test".to_string(),
            project_id: "proj".to_string(),
            branch: "branch".to_string(),
            worktree_path: "/tmp".to_string(),
            agent: "claude".to_string(),
            status: "stopped".to_string(),
            created_at: "2026-02-09T14:30:00Z".to_string(),
            note: None,
            client_count: None,
            pty_pid: None,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(!json.contains("note"));
        assert!(!json.contains("client_count"));
        assert!(!json.contains("pty_pid"));
    }
}
