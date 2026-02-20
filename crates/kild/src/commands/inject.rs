use std::fs;
use std::fs::OpenOptions;

use chrono::Utc;
use clap::ArgMatches;
use nix::fcntl::{Flock, FlockArg};
use serde_json::json;
use tracing::{error, info};

use kild_core::agents::{InjectMethod, get_inject_method};

use super::helpers;

/// Default team name for fleet mode. Will become a config key.
const DEFAULT_TEAM: &str = "honryu";

pub(crate) fn handle_inject_command(
    matches: &ArgMatches,
) -> Result<(), Box<dyn std::error::Error>> {
    let branch = matches
        .get_one::<String>("branch")
        .ok_or("Branch argument is required")?;
    let text = matches
        .get_one::<String>("text")
        .ok_or("Text argument is required")?;
    let force_inbox = matches.get_flag("inbox");

    info!(event = "cli.inject_started", branch = branch);

    let session = helpers::require_session(branch, "cli.inject_failed")?;

    // Determine inject method: --inbox forces inbox protocol; otherwise use agent default.
    let method = if force_inbox {
        InjectMethod::ClaudeInbox
    } else {
        get_inject_method(&session.agent)
    };

    let result = match method {
        InjectMethod::Pty => write_to_pty(&session, text),
        InjectMethod::ClaudeInbox => write_to_inbox(DEFAULT_TEAM, branch, text),
    };

    if let Err(e) = result {
        eprintln!("{}", crate::color::error(&format!("Inject failed: {}", e)));
        error!(event = "cli.inject_failed", branch = branch, error = %e);
        return Err(e);
    }

    info!(event = "cli.inject_completed", branch = branch);
    Ok(())
}

/// Write text to the agent's PTY stdin via the daemon WriteStdin IPC.
///
/// Works for all agents. The text is sent as raw bytes with a trailing newline.
/// PTY stdin is kernel-buffered — the agent reads it when its input handler is ready.
/// This is the universal inject path and works on cold start.
fn write_to_pty(
    session: &kild_core::Session,
    text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let daemon_session_id = session
        .latest_agent()
        .and_then(|a| a.daemon_session_id())
        .ok_or_else(|| {
            format!(
                "Session '{}' has no active daemon PTY. Is it a daemon session? \
                 Use `kild create --daemon` or `kild open --daemon`.",
                session.branch
            )
        })?;

    // Two separate writes: text then Enter (\r), with a brief pause between.
    // TUI agents need the text and Enter in separate read() cycles to correctly
    // submit the input rather than treating \r as a literal character.
    kild_core::daemon::client::write_stdin(daemon_session_id, text.as_bytes())
        .map_err(|e| format!("PTY write failed (text): {}", e))?;

    std::thread::sleep(std::time::Duration::from_millis(50));

    kild_core::daemon::client::write_stdin(daemon_session_id, b"\r")
        .map_err(|e| format!("PTY write failed (enter): {}", e).into())
}

/// Write a message to a Claude Code inbox file.
///
/// Claude Code polls `~/.claude/teams/<team>/inboxes/<agent>.json` every 1 second
/// and delivers unread messages as user turns. The session must have been started
/// with `--agent-id <agent>@<team> --agent-name <agent> --team-name <team>`.
///
/// Uses an exclusive flock on `<agent>.lock` to prevent concurrent writes from
/// the hook script (which fires on every worker Stop event) from racing and
/// overwriting each other's messages.
pub(crate) fn write_to_inbox(
    team: &str,
    agent: &str,
    text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let base = std::env::var("CLAUDE_CONFIG_DIR")
        .map(std::path::PathBuf::from)
        .ok()
        .or_else(|| dirs::home_dir().map(|h| h.join(".claude")))
        .ok_or("HOME directory not found — cannot locate Claude config directory")?;

    let inbox_dir = base.join("teams").join(team).join("inboxes");
    fs::create_dir_all(&inbox_dir)?;

    let inbox_path = inbox_dir.join(format!("{}.json", agent));
    let lock_path = inbox_dir.join(format!("{}.lock", agent));

    // Acquire exclusive file lock to prevent concurrent hook invocations from
    // racing on the read-modify-write below. Held until _lock drops at end of fn.
    let lock_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)?;
    let _lock = Flock::lock(lock_file, FlockArg::LockExclusive)
        .map_err(|(_, e)| format!("failed to lock inbox: {}", e))?;

    // Read existing messages (preserving history for the session).
    let mut messages: Vec<serde_json::Value> = if inbox_path.exists() {
        let raw = fs::read_to_string(&inbox_path)?;
        serde_json::from_str(&raw).map_err(|e| {
            format!(
                "inbox at {} is corrupt ({}). Delete it and retry.",
                inbox_path.display(),
                e
            )
        })?
    } else {
        Vec::new()
    };

    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    messages.push(json!({
        "from": team,
        "text": text,
        "timestamp": timestamp,
        "read": false
    }));

    fs::write(&inbox_path, serde_json::to_string_pretty(&messages)?)?;

    Ok(())
}
