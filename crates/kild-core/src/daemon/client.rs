//! Synchronous IPC client for communicating with the KILD daemon.
//!
//! Uses `std::os::unix::net::UnixStream` — no tokio dependency.
//! Constructs JSONL messages manually with `serde_json::json!()` to avoid
//! importing types from kild-daemon (which depends on kild-core).

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use tracing::{debug, info};

/// Result of creating a PTY session in the daemon.
#[derive(Debug, Clone)]
pub struct DaemonCreateResult {
    /// Daemon-assigned session identifier.
    pub daemon_session_id: String,
}

/// Error communicating with the daemon.
#[derive(Debug, thiserror::Error)]
pub enum DaemonClientError {
    #[error("Daemon is not running (socket not found at {path})")]
    NotRunning { path: String },

    #[error("Connection failed: {message}")]
    ConnectionFailed { message: String },

    #[error("Daemon returned error: {message}")]
    DaemonError { message: String },

    #[error("IPC protocol error: {message}")]
    ProtocolError { message: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Connect to the daemon socket with a timeout.
fn connect(socket_path: &Path) -> Result<UnixStream, DaemonClientError> {
    if !socket_path.exists() {
        return Err(DaemonClientError::NotRunning {
            path: socket_path.display().to_string(),
        });
    }

    let stream = UnixStream::connect(socket_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::ConnectionRefused {
            DaemonClientError::NotRunning {
                path: socket_path.display().to_string(),
            }
        } else {
            DaemonClientError::ConnectionFailed {
                message: e.to_string(),
            }
        }
    })?;

    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    Ok(stream)
}

/// Send a JSONL request and read the response line.
fn send_request(
    stream: &mut UnixStream,
    request: serde_json::Value,
) -> Result<serde_json::Value, DaemonClientError> {
    let msg = serde_json::to_string(&request).map_err(|e| DaemonClientError::ProtocolError {
        message: e.to_string(),
    })?;

    writeln!(stream, "{}", msg)?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;

    if line.is_empty() {
        return Err(DaemonClientError::ProtocolError {
            message: "Empty response from daemon".to_string(),
        });
    }

    let response: serde_json::Value =
        serde_json::from_str(&line).map_err(|e| DaemonClientError::ProtocolError {
            message: format!("Invalid JSON response: {}", e),
        })?;

    // Check for error responses
    if response.get("type").and_then(|t| t.as_str()) == Some("error") {
        let code = response
            .get("code")
            .and_then(|c| c.as_str())
            .unwrap_or("unknown");
        let message = response
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown daemon error");
        return Err(DaemonClientError::DaemonError {
            message: format!("[{}] {}", code, message),
        });
    }

    Ok(response)
}

/// Create a new PTY session in the daemon.
///
/// Sends a `create_session` JSONL message to the daemon via unix socket.
/// Blocks until the daemon responds with session ID or error.
pub fn create_pty_session(
    spawn_id: &str,
    working_directory: &Path,
    command: &str,
) -> Result<DaemonCreateResult, DaemonClientError> {
    let socket_path = crate::daemon::socket_path();

    info!(
        event = "core.daemon.create_pty_session_started",
        spawn_id = spawn_id,
        working_directory = %working_directory.display()
    );

    let request = serde_json::json!({
        "id": spawn_id,
        "type": "create_session",
        "spawn_id": spawn_id,
        "working_directory": working_directory.to_string_lossy(),
        "command": command,
    });

    let mut stream = connect(&socket_path)?;
    let response = send_request(&mut stream, request)?;

    let session_id = response
        .get("session")
        .and_then(|s| s.get("id"))
        .and_then(|id| id.as_str())
        .ok_or_else(|| DaemonClientError::ProtocolError {
            message: "Response missing session.id field".to_string(),
        })?
        .to_string();

    info!(
        event = "core.daemon.create_pty_session_completed",
        daemon_session_id = session_id
    );

    Ok(DaemonCreateResult {
        daemon_session_id: session_id,
    })
}

/// Stop a daemon-managed session (kill the PTY process).
pub fn stop_daemon_session(daemon_session_id: &str) -> Result<(), DaemonClientError> {
    let socket_path = crate::daemon::socket_path();

    info!(
        event = "core.daemon.stop_session_started",
        daemon_session_id = daemon_session_id
    );

    let request = serde_json::json!({
        "id": format!("stop-{}", daemon_session_id),
        "type": "stop_session",
        "session_id": daemon_session_id,
    });

    let mut stream = connect(&socket_path)?;
    send_request(&mut stream, request)?;

    info!(
        event = "core.daemon.stop_session_completed",
        daemon_session_id = daemon_session_id
    );

    Ok(())
}

/// Check if the daemon is running and responsive.
pub fn ping_daemon() -> Result<bool, DaemonClientError> {
    let socket_path = crate::daemon::socket_path();

    debug!(event = "core.daemon.ping_started");

    let request = serde_json::json!({
        "id": "ping",
        "type": "ping",
    });

    let mut stream = match connect(&socket_path) {
        Ok(s) => s,
        Err(DaemonClientError::NotRunning { .. }) => return Ok(false),
        Err(e) => return Err(e),
    };

    // Use a short timeout for ping
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;

    match send_request(&mut stream, request) {
        Ok(_) => {
            debug!(event = "core.daemon.ping_completed", alive = true);
            Ok(true)
        }
        Err(_) => {
            debug!(event = "core.daemon.ping_completed", alive = false);
            Ok(false)
        }
    }
}

/// Request the daemon to shut down gracefully.
pub fn request_shutdown() -> Result<(), DaemonClientError> {
    let socket_path = crate::daemon::socket_path();

    info!(event = "core.daemon.shutdown_started");

    let request = serde_json::json!({
        "id": "shutdown",
        "type": "daemon_stop",
    });

    let mut stream = connect(&socket_path)?;
    send_request(&mut stream, request)?;

    info!(event = "core.daemon.shutdown_completed");
    Ok(())
}
