use clap::ArgMatches;
use tracing::{error, info};

use kild_core::{daemon, events};

use super::helpers;

pub(crate) fn handle_inject_command(
    matches: &ArgMatches,
) -> Result<(), Box<dyn std::error::Error>> {
    let branch = matches
        .get_one::<String>("branch")
        .ok_or("Branch argument is required")?;
    let text = matches
        .get_one::<String>("text")
        .ok_or("Text argument is required")?;

    info!(event = "cli.inject_started", branch = branch);

    let session = helpers::require_session(branch, "cli.inject_failed")?;

    let daemon_session_id = session
        .latest_agent()
        .and_then(|a| a.daemon_session_id())
        .ok_or_else(|| {
            let msg = format!(
                "kild '{}' has no active daemon session — use `kild create --daemon` or `kild open`",
                branch
            );
            eprintln!("{}", crate::color::error(&msg));
            error!(
                event = "cli.inject_failed",
                branch = branch,
                reason = "no_daemon_session"
            );
            msg
        })?;

    // Append carriage return to submit the message in Claude Code's raw-mode TUI.
    // Raw mode uses \r (CR, 0x0D) for Enter, not \n (LF, 0x0A).
    // \n would insert a newline into the multi-line input buffer without submitting.
    let mut payload = text.clone();
    payload.push('\r');

    if let Err(e) = daemon::client::write_stdin(daemon_session_id, payload.as_bytes()) {
        eprintln!("{}", crate::color::error(&format!("Inject failed: {}", e)));
        error!(event = "cli.inject_failed", branch = branch, error = %e);
        events::log_app_error(&e);
        return Err(e.into());
    }

    info!(event = "cli.inject_completed", branch = branch);
    Ok(())
}
