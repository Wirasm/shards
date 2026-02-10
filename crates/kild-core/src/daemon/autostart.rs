use tracing::{error, info};

use crate::config::KildConfig;
use crate::daemon::{client, socket_path};

/// Errors from daemon auto-start.
#[derive(Debug, thiserror::Error)]
pub enum DaemonAutoStartError {
    #[error(
        "Daemon is not running. To fix this, either:\n  \
         - Start it manually: kild daemon start\n  \
         - Enable auto-start in config: [daemon] auto_start = true\n  \
         - Use --no-daemon to launch in an external terminal instead"
    )]
    Disabled,

    #[error("Failed to start daemon: {message}")]
    SpawnFailed { message: String },

    #[error("Daemon auto-start timed out: {message}")]
    Timeout { message: String },

    #[error("Could not determine daemon binary path: {message}")]
    BinaryNotFound { message: String },
}

/// Ensure the daemon is running, auto-starting it if configured.
///
/// 1. Pings the daemon — if alive, returns immediately.
/// 2. Checks `config.daemon_auto_start()` — if disabled, returns `Disabled` error.
/// 3. Spawns `kild daemon start --foreground` in background.
/// 4. Polls socket + ping with 5s timeout, 100ms interval.
pub fn ensure_daemon_running(config: &KildConfig) -> Result<(), DaemonAutoStartError> {
    if client::ping_daemon().unwrap_or(false) {
        return Ok(());
    }

    if !config.daemon_auto_start() {
        return Err(DaemonAutoStartError::Disabled);
    }

    info!(event = "core.daemon.auto_start_started");
    eprintln!("Starting daemon...");

    let daemon_binary =
        std::env::current_exe().map_err(|e| DaemonAutoStartError::BinaryNotFound {
            message: e.to_string(),
        })?;

    std::process::Command::new(&daemon_binary)
        .args(["daemon", "start", "--foreground"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .spawn()
        .map_err(|e| DaemonAutoStartError::SpawnFailed {
            message: e.to_string(),
        })?;

    let socket = socket_path();
    let timeout = std::time::Duration::from_secs(5);
    let start = std::time::Instant::now();

    loop {
        if socket.exists() && client::ping_daemon().unwrap_or(false) {
            info!(event = "core.daemon.auto_start_completed");
            eprintln!("Daemon started.");
            return Ok(());
        }
        if start.elapsed() > timeout {
            error!(event = "core.daemon.auto_start_failed");
            if socket.exists() {
                return Err(DaemonAutoStartError::Timeout {
                    message: "Daemon socket exists but not responding to ping after 5s.\n\
                              Try: kild daemon stop && kild daemon start"
                        .to_string(),
                });
            } else {
                return Err(DaemonAutoStartError::Timeout {
                    message: "Daemon process spawned but socket not created after 5s.\n\
                              Check daemon logs: kild daemon start --foreground"
                        .to_string(),
                });
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_start_disabled_returns_error() {
        // If a daemon is already running, ensure_daemon_running returns Ok
        // regardless of config (correct behavior - no need to auto-start).
        // Only test the Disabled path when daemon is not running.
        if client::ping_daemon().unwrap_or(false) {
            return;
        }

        let mut value = serde_json::to_value(KildConfig::default()).unwrap();
        value["daemon"]["auto_start"] = serde_json::Value::Bool(false);
        let config: KildConfig = serde_json::from_value(value).unwrap();

        let result = ensure_daemon_running(&config);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(matches!(err, DaemonAutoStartError::Disabled));

        let msg = err.to_string();
        assert!(
            msg.contains("kild daemon start"),
            "should mention manual start"
        );
        assert!(
            msg.contains("auto_start = true"),
            "should mention config option"
        );
        assert!(
            msg.contains("--no-daemon"),
            "should mention --no-daemon flag"
        );
    }

    #[test]
    fn test_auto_start_default_config_is_enabled() {
        assert!(KildConfig::default().daemon_auto_start());
    }

    #[test]
    fn test_auto_start_error_display_messages() {
        let disabled = DaemonAutoStartError::Disabled;
        assert!(disabled.to_string().contains("not running"));

        let spawn_failed = DaemonAutoStartError::SpawnFailed {
            message: "permission denied".to_string(),
        };
        assert!(spawn_failed.to_string().contains("permission denied"));

        let timeout = DaemonAutoStartError::Timeout {
            message: "socket not created".to_string(),
        };
        assert!(timeout.to_string().contains("socket not created"));

        let not_found = DaemonAutoStartError::BinaryNotFound {
            message: "no such file".to_string(),
        };
        assert!(not_found.to_string().contains("no such file"));
    }
}
