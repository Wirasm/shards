use clap::ArgMatches;
use tracing::{error, info};

use kild_core::events;
use kild_core::session_ops;

use super::helpers::is_valid_branch_name;

pub(crate) fn handle_complete_command(
    matches: &ArgMatches,
) -> Result<(), Box<dyn std::error::Error>> {
    let branch = matches
        .get_one::<String>("branch")
        .ok_or("Branch argument is required")?;

    if !is_valid_branch_name(branch) {
        eprintln!("Invalid branch name: {}", branch);
        error!(event = "cli.complete_invalid_branch", branch = branch);
        return Err("Invalid branch name".into());
    }

    info!(event = "cli.complete_started", branch = branch);

    match session_ops::complete_session(branch) {
        Ok(result) => {
            use kild_core::CompleteResult;

            println!("\u{2705} KILD '{}' completed!", branch);
            match result {
                CompleteResult::RemoteDeleted => {
                    println!("   Remote branch deleted (PR was merged)");
                }
                CompleteResult::RemoteDeleteFailed => {
                    println!("   Remote branch deletion failed (PR was merged, check logs)");
                }
                CompleteResult::PrNotMerged => {
                    println!("   Remote branch preserved (merge will delete it)");
                }
                CompleteResult::PrCheckUnavailable => {
                    println!(
                        "   Could not verify PR merge status \u{2014} remote branch preserved"
                    );
                }
            }

            info!(
                event = "cli.complete_completed",
                branch = branch,
                result = ?result
            );

            Ok(())
        }
        Err(e) => {
            eprintln!("\u{274c} {}", e);

            error!(
                event = "cli.complete_failed",
                branch = branch,
                error = %e
            );

            events::log_app_error(&e);
            Err(e.into())
        }
    }
}
