use clap::ArgMatches;
use tracing::{error, info};

use kild_core::daemon::client;
use kild_core::events;
use kild_teams::discovery;

use super::helpers;
use crate::color;

pub(crate) fn handle_teammates_command(
    matches: &ArgMatches,
) -> Result<(), Box<dyn std::error::Error>> {
    let branch = matches
        .get_one::<String>("branch")
        .ok_or("Branch argument is required")?;
    let json_output = matches.get_flag("json");

    info!(event = "cli.teammates_started", branch = branch);

    // 1. Find session
    let session = helpers::require_session(branch, "cli.teammates_failed")?;

    // 2. Discover panes (leader + teammates) from shim registry
    let members = match discovery::discover_teammates(&session.id) {
        Ok(Some(m)) => m,
        Ok(None) => {
            if json_output {
                println!("[]");
            } else {
                println!(
                    "No agent team found for '{}'. Session has no teammates.",
                    branch
                );
                println!(
                    "  {} Create with: kild create {} --daemon",
                    color::muted("Hint:"),
                    branch
                );
            }
            info!(
                event = "cli.teammates_completed",
                branch = branch,
                count = 0
            );
            return Ok(());
        }
        Err(e) => {
            error!(event = "cli.teammates_failed", branch = branch, error = %e);
            events::log_app_error(&e);
            eprintln!("Failed to read pane registry: {}", e);
            return Err(e.into());
        }
    };

    // 3. Enrich with live daemon status
    let enriched: Vec<_> = members
        .iter()
        .map(|m| {
            let status = m
                .daemon_session_id
                .as_deref()
                .and_then(|sid| client::get_session_status(sid).ok().flatten());
            (m, status)
        })
        .collect();

    if json_output {
        let json: Vec<_> = enriched
            .iter()
            .map(|(m, status)| {
                serde_json::json!({
                    "pane_id": m.pane_id,
                    "name": m.name,
                    "role": if m.is_leader { "leader" } else { "teammate" },
                    "daemon_session_id": m.daemon_session_id,
                    "status": status.as_ref().map(|s| format!("{}", s)),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json)?);
    } else {
        println!("{}", color::bold(&format!("Agent team for '{}':", branch)));
        println!(
            "  {:<6}  {:<10}  {:<20}  {:<10}",
            color::muted("PANE"),
            color::muted("ROLE"),
            color::muted("NAME"),
            color::muted("STATUS"),
        );
        for (m, status) in &enriched {
            let role = if m.is_leader { "leader" } else { "teammate" };
            let status_str = match status {
                Some(s) => format!("{}", s),
                None => "-".to_string(),
            };
            println!(
                "  {:<6}  {:<10}  {:<20}  {:<10}",
                color::ice(&m.pane_id),
                color::muted(role),
                m.name,
                status_str,
            );
        }
        println!();
        println!(
            "{}  kild attach {} --pane <pane>  |  kild stop {} --pane <pane>",
            color::muted("Actions:"),
            branch,
            branch
        );
    }

    info!(
        event = "cli.teammates_completed",
        branch = branch,
        count = enriched.len()
    );
    Ok(())
}
