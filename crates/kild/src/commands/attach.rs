use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

use clap::ArgMatches;
use nix::sys::termios;
use tracing::{error, info};

pub(crate) fn handle_attach_command(
    matches: &ArgMatches,
) -> Result<(), Box<dyn std::error::Error>> {
    let branch = matches
        .get_one::<String>("branch")
        .ok_or("Branch argument is required")?;

    info!(event = "cli.attach_started", branch = branch);

    // 1. Look up session to get daemon_session_id
    let session = kild_core::session_ops::get_session(branch)?;

    let daemon_session_id = session
        .latest_agent()
        .and_then(|a| a.daemon_session_id())
        .ok_or_else(|| {
            format!(
                "Session '{}' is not daemon-managed. Use 'kild focus {}' for terminal sessions.",
                branch, branch
            )
        })?
        .to_string();

    info!(
        event = "cli.attach_connecting",
        branch = branch,
        daemon_session_id = daemon_session_id.as_str()
    );

    // 2. Connect to daemon and attach
    attach_to_daemon_session(&daemon_session_id)?;

    info!(event = "cli.attach_completed", branch = branch);
    Ok(())
}

fn attach_to_daemon_session(daemon_session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let socket_path = kild_core::daemon::socket_path();
    let mut stream = UnixStream::connect(&socket_path).map_err(|e| {
        format!(
            "Cannot connect to daemon at {}: {}",
            socket_path.display(),
            e
        )
    })?;

    // Get terminal size
    let (cols, rows) = terminal_size();

    // Send attach request
    let attach_msg = serde_json::json!({
        "id": "attach-1",
        "type": "attach",
        "session_id": daemon_session_id,
        "cols": cols,
        "rows": rows,
    });
    writeln!(stream, "{}", serde_json::to_string(&attach_msg)?)?;
    stream.flush()?;

    // Read ack response
    let mut reader = std::io::BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    std::io::BufRead::read_line(&mut reader, &mut line)?;

    let ack: serde_json::Value = serde_json::from_str(line.trim())?;
    if ack.get("type").and_then(|t| t.as_str()) == Some("error") {
        let msg = ack
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        return Err(format!("Attach failed: {}", msg).into());
    }

    // Enter raw terminal mode
    let _raw_guard = enable_raw_mode()?;

    // Spawn stdin reader thread (owned String for 'static lifetime)
    let session_id_owned = daemon_session_id.to_string();
    let mut write_stream = stream.try_clone()?;
    let stdin_handle = std::thread::spawn(move || {
        forward_stdin_to_daemon(&mut write_stream, &session_id_owned);
    });

    // Main thread: read daemon output, write to stdout
    let read_stream = reader.into_inner();
    forward_daemon_to_stdout(read_stream)?;

    // Restore terminal
    drop(_raw_guard);
    eprintln!("\r\nDetached from session.");

    let _ = stdin_handle.join();
    Ok(())
}

fn terminal_size() -> (u16, u16) {
    use nix::libc;
    unsafe {
        let mut winsize: libc::winsize = std::mem::zeroed();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut winsize) == 0 {
            (winsize.ws_col, winsize.ws_row)
        } else {
            (80, 24)
        }
    }
}

struct RawModeGuard {
    original: termios::Termios,
}

fn enable_raw_mode() -> Result<RawModeGuard, Box<dyn std::error::Error>> {
    use std::os::fd::BorrowedFd;

    let stdin_fd = unsafe { BorrowedFd::borrow_raw(0) };
    let original = termios::tcgetattr(stdin_fd).map_err(|e| format!("tcgetattr failed: {}", e))?;

    let mut raw = original.clone();
    termios::cfmakeraw(&mut raw);
    termios::tcsetattr(stdin_fd, termios::SetArg::TCSANOW, &raw)
        .map_err(|e| format!("tcsetattr failed: {}", e))?;

    Ok(RawModeGuard { original })
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        use std::os::fd::BorrowedFd;
        let stdin_fd = unsafe { BorrowedFd::borrow_raw(0) };
        let _ = termios::tcsetattr(stdin_fd, termios::SetArg::TCSANOW, &self.original);
    }
}

/// Forwards stdin bytes to the daemon over IPC, base64-encoded.
/// Supports a Ctrl+B, d detach sequence (similar to tmux's Ctrl+B prefix).
fn forward_stdin_to_daemon(stream: &mut UnixStream, session_id: &str) {
    use base64::Engine;

    let stdin = std::io::stdin();
    let mut buf = [0u8; 4096];
    let mut ctrl_b_pressed = false;

    loop {
        let n = match stdin.lock().read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };

        // Filter out detach sequence bytes (Ctrl+B + 'd') and build clean buffer
        let mut filtered = Vec::with_capacity(n);
        let mut detach = false;

        for &byte in &buf[..n] {
            if ctrl_b_pressed {
                if byte == b'd' {
                    // Detach sequence complete — send detach, don't forward anything
                    detach = true;
                    break;
                }
                // Ctrl+B was not followed by 'd', emit the held Ctrl+B
                filtered.push(0x02);
                ctrl_b_pressed = false;
            }

            if byte == 0x02 {
                // Hold Ctrl+B until we see what comes next
                ctrl_b_pressed = true;
            } else {
                filtered.push(byte);
            }
        }

        if detach {
            let detach_msg = serde_json::json!({
                "id": "detach-1",
                "type": "detach",
                "session_id": session_id,
            });
            match serde_json::to_string(&detach_msg) {
                Ok(msg) => {
                    let _ = writeln!(stream, "{}", msg);
                    let _ = stream.flush();
                }
                Err(e) => {
                    error!(event = "cli.attach.detach_serialize_failed", error = %e);
                }
            }
            return;
        }

        // Nothing to forward if all bytes were filtered out
        if filtered.is_empty() {
            continue;
        }

        // Forward filtered input to daemon
        let encoded = base64::engine::general_purpose::STANDARD.encode(&filtered);
        let input_msg = serde_json::json!({
            "id": format!("write-{}", filtered.len()),
            "type": "write_stdin",
            "session_id": session_id,
            "data": encoded,
        });
        let serialized = match serde_json::to_string(&input_msg) {
            Ok(s) => s,
            Err(e) => {
                error!(event = "cli.attach.stdin_serialize_failed", error = %e);
                continue;
            }
        };
        if let Err(e) = writeln!(stream, "{}", serialized) {
            error!(event = "cli.attach.stdin_write_failed", error = %e);
            eprintln!("\r\nConnection to daemon lost.");
            break;
        }
        if let Err(e) = stream.flush() {
            error!(event = "cli.attach.stdin_flush_failed", error = %e);
            eprintln!("\r\nConnection to daemon lost.");
            break;
        }
    }
}

fn forward_daemon_to_stdout(mut stream: UnixStream) -> Result<(), Box<dyn std::error::Error>> {
    use base64::Engine;

    let mut reader = std::io::BufReader::new(&mut stream);
    let mut line = String::new();
    let mut stdout = std::io::stdout();

    loop {
        line.clear();
        let n = std::io::BufRead::read_line(&mut reader, &mut line)?;
        if n == 0 {
            break; // EOF
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let msg: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                error!(event = "cli.attach.parse_failed", error = %e);
                eprintln!("\r\nWarning: received malformed message from daemon.");
                continue;
            }
        };

        match msg.get("type").and_then(|t| t.as_str()) {
            Some("pty_output") => {
                if let Some(data) = msg.get("data").and_then(|d| d.as_str())
                    && let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(data)
                {
                    stdout.write_all(&decoded)?;
                    stdout.flush()?;
                }
            }
            Some("pty_output_dropped") => {}

            Some("session_event") => {
                if let Some(event) = msg.get("event").and_then(|e| e.as_str())
                    && event == "stopped"
                {
                    eprintln!("\r\nSession process exited.");
                    break;
                }
            }
            _ => {
                // Ignore other messages (ack, etc.)
            }
        }
    }

    Ok(())
}
