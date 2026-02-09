use clap::ArgMatches;
use tracing::{error, info};

use kild_core::session_ops;

use super::helpers::{format_partial_failure_error, load_config_with_warning};

pub(crate) fn handle_overlaps_command(
    matches: &ArgMatches,
) -> Result<(), Box<dyn std::error::Error>> {
    let json_output = matches.get_flag("json");
    let config = load_config_with_warning();
    let base_branch = match matches.get_one::<String>("base") {
        Some(s) => s.as_str(),
        None => config.git.base_branch(),
    };

    info!(
        event = "cli.overlaps_started",
        base = base_branch,
        json_output = json_output
    );

    let sessions = session_ops::list_sessions()?;

    if sessions.is_empty() {
        println!("No kilds found.");
        info!(event = "cli.overlaps_completed", overlap_count = 0);
        return Ok(());
    }

    if sessions.len() < 2 {
        println!("Only 1 kild active. Overlaps require at least 2 kilds.");
        info!(event = "cli.overlaps_completed", overlap_count = 0);
        return Ok(());
    }

    let (report, errors) =
        kild_core::git::operations::collect_file_overlaps(&sessions, base_branch);

    info!(
        event = "cli.overlaps_completed",
        overlap_count = report.overlapping_files.len(),
        clean_count = report.clean_kilds.len(),
        errors = errors.len()
    );

    if json_output {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_overlap_report(&report);
    }

    if !errors.is_empty() {
        let total = sessions.len();
        eprintln!();
        for (branch, msg) in &errors {
            eprintln!("  Warning: {} — {}", branch, msg);
        }
        error!(
            event = "cli.overlaps_failed",
            failed = errors.len(),
            total = total
        );
        return Err(format_partial_failure_error("compute overlaps", errors.len(), total).into());
    }

    Ok(())
}

fn print_overlap_report(report: &kild_core::OverlapReport) {
    if report.overlapping_files.is_empty() {
        println!("No file overlaps detected across kilds.");
        if !report.clean_kilds.is_empty() {
            println!();
            for (branch, file_count) in &report.clean_kilds {
                println!("  {} ({} files changed)", branch, file_count);
            }
        }
        return;
    }

    println!("Overlapping files across kilds:");
    println!();
    for overlap in &report.overlapping_files {
        println!("  {}", overlap.file.display());
        println!("    modified by: {}", overlap.branches.join(", "));
    }

    if !report.clean_kilds.is_empty() {
        println!();
        println!("No overlaps:");
        for (branch, file_count) in &report.clean_kilds {
            println!(
                "  {} ({} files changed, no shared files with other kilds)",
                branch, file_count
            );
        }
    }
}
