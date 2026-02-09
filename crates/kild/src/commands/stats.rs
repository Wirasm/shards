use std::fmt;

use clap::ArgMatches;
use serde::Serialize;
use tracing::{error, info};

use kild_core::BranchHealth;
use kild_core::ConflictStatus;
use kild_core::events;
use kild_core::forge::types::CiStatus;
use kild_core::git::types::WorktreeStatus;
use kild_core::session_ops;

use super::helpers::{
    FailedOperation, format_partial_failure_error, is_valid_branch_name, load_config_with_warning,
};

/// Computed merge readiness status for a branch.
///
/// Lives in the CLI layer because it combines git metrics with forge/PR data.
/// The git module provides raw metrics; readiness is a presentation concern.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum MergeReadiness {
    /// Clean, pushed, PR open, CI passing
    Ready,
    /// Has unpushed commits
    NeedsPush,
    /// Behind base branch significantly
    NeedsRebase,
    /// Cannot merge cleanly into base
    HasConflicts,
    /// Pushed but no PR exists
    NeedsPr,
    /// PR exists but CI is failing
    CiFailing,
    /// Ready to merge locally (no remote configured)
    ReadyLocal,
}

impl fmt::Display for MergeReadiness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MergeReadiness::Ready => write!(f, "Ready"),
            MergeReadiness::NeedsPush => write!(f, "Needs push"),
            MergeReadiness::NeedsRebase => write!(f, "Needs rebase"),
            MergeReadiness::HasConflicts => write!(f, "Has conflicts"),
            MergeReadiness::NeedsPr => write!(f, "Needs PR"),
            MergeReadiness::CiFailing => write!(f, "CI failing"),
            MergeReadiness::ReadyLocal => write!(f, "Ready (local)"),
        }
    }
}

/// Compute merge readiness from git health metrics, worktree status, and optional PR info.
///
/// Priority: HasConflicts > NeedsRebase > NeedsPush > NeedsPr > CiFailing > Ready/ReadyLocal
fn compute_merge_readiness(
    health: &BranchHealth,
    worktree_status: &Option<WorktreeStatus>,
    pr_info: Option<&kild_core::PrInfo>,
) -> MergeReadiness {
    if health.conflict_status != ConflictStatus::Clean {
        return MergeReadiness::HasConflicts;
    }

    if health.drift.behind > 0 {
        return MergeReadiness::NeedsRebase;
    }

    if !health.has_remote {
        return MergeReadiness::ReadyLocal;
    }

    // Check if there are unpushed commits
    let has_unpushed = worktree_status
        .as_ref()
        .is_some_and(|ws| ws.unpushed_commit_count > 0 || !ws.has_remote_branch);

    if has_unpushed {
        return MergeReadiness::NeedsPush;
    }

    match pr_info {
        None => MergeReadiness::NeedsPr,
        Some(pr) => {
            if pr.ci_status == CiStatus::Failing {
                MergeReadiness::CiFailing
            } else {
                MergeReadiness::Ready
            }
        }
    }
}

/// Combined output for JSON: git health + computed readiness.
#[derive(Serialize)]
struct StatsOutput {
    #[serde(flatten)]
    health: BranchHealth,
    merge_readiness: MergeReadiness,
}

pub(crate) fn handle_stats_command(matches: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    if matches.get_flag("all") {
        let base_override = matches.get_one::<String>("base").cloned();
        let json_output = matches.get_flag("json");
        return handle_all_stats(base_override, json_output);
    }

    let branch = matches
        .get_one::<String>("branch")
        .ok_or("Branch argument is required (or use --all)")?;
    let json_output = matches.get_flag("json");

    if !is_valid_branch_name(branch) {
        eprintln!("Invalid branch name: {}", branch);
        error!(event = "cli.stats_invalid_branch", branch = branch);
        return Err("Invalid branch name".into());
    }

    let config = load_config_with_warning();
    let base_branch = match matches.get_one::<String>("base") {
        Some(s) => s.as_str(),
        None => config.git.base_branch(),
    };

    handle_single_stats(branch, base_branch, json_output)
}

fn handle_single_stats(
    branch: &str,
    base_branch: &str,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    info!(
        event = "cli.stats_started",
        branch = branch,
        base = base_branch,
        json_output = json_output
    );

    let session = match session_ops::get_session(branch) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to find kild '{}': {}", branch, e);
            error!(event = "cli.stats_failed", branch = branch, error = %e);
            events::log_app_error(&e);
            return Err(e.into());
        }
    };

    let health = kild_core::git::operations::collect_branch_health(
        &session.worktree_path,
        branch,
        base_branch,
        &session.created_at,
    );

    match health {
        Ok(h) => {
            // Compose: git health + worktree status + PR info → readiness
            let worktree_status =
                kild_core::git::operations::get_worktree_status(&session.worktree_path).ok();
            let pr_info = session_ops::read_pr_info(&session.id);
            let readiness = compute_merge_readiness(&h, &worktree_status, pr_info.as_ref());

            info!(
                event = "cli.stats_completed",
                branch = branch,
                readiness = %readiness
            );

            if json_output {
                let output = StatsOutput {
                    health: h,
                    merge_readiness: readiness,
                };
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                print_single_health(branch, &h, &readiness);
            }
            Ok(())
        }
        Err(msg) => {
            eprintln!("Could not compute branch health for '{}': {}", branch, msg);
            error!(
                event = "cli.stats_failed",
                branch = branch,
                reason = "health_unavailable"
            );
            Err(format!("Branch health unavailable for '{}'", branch).into())
        }
    }
}

fn handle_all_stats(
    base_override: Option<String>,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    info!(event = "cli.stats_all_started", base_override = ?base_override);

    let config = load_config_with_warning();
    let base_branch = match base_override.as_deref() {
        Some(base) => base,
        None => config.git.base_branch(),
    };

    let sessions = session_ops::list_sessions()?;

    if sessions.is_empty() {
        println!("No kilds found.");
        info!(event = "cli.stats_all_completed", count = 0);
        return Ok(());
    }

    let mut results: Vec<(BranchHealth, MergeReadiness)> = Vec::new();
    let mut errors: Vec<FailedOperation> = Vec::new();

    for session in &sessions {
        match kild_core::git::operations::collect_branch_health(
            &session.worktree_path,
            &session.branch,
            base_branch,
            &session.created_at,
        ) {
            Ok(h) => {
                let worktree_status =
                    kild_core::git::operations::get_worktree_status(&session.worktree_path).ok();
                let pr_info = session_ops::read_pr_info(&session.id);
                let readiness = compute_merge_readiness(&h, &worktree_status, pr_info.as_ref());
                results.push((h, readiness));
            }
            Err(msg) => {
                errors.push((session.branch.clone(), msg));
            }
        }
    }

    let result_count = results.len();

    info!(
        event = "cli.stats_all_completed",
        count = result_count,
        failed = errors.len()
    );

    if json_output {
        let output: Vec<StatsOutput> = results
            .into_iter()
            .map(|(health, merge_readiness)| StatsOutput {
                health,
                merge_readiness,
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_fleet_table(&results);
    }

    if !errors.is_empty() {
        let total = result_count + errors.len();
        return Err(format_partial_failure_error("compute stats", errors.len(), total).into());
    }

    Ok(())
}

fn print_single_health(branch: &str, h: &BranchHealth, readiness: &MergeReadiness) {
    let kild_branch = kild_core::git::operations::kild_branch_name(branch);

    println!("Branch:       {} ({})", branch, kild_branch);
    println!("Created:      {}", h.created_at);

    // Last commit + commits since base
    let last_commit = h
        .commit_activity
        .last_commit_time
        .as_deref()
        .unwrap_or("none");
    println!(
        "Last commit:  {} ({} commits since {})",
        last_commit, h.commit_activity.commits_since_base, h.drift.base_branch
    );

    // Diff vs base
    if let Some(ref diff) = h.diff_vs_base {
        println!(
            "Diff vs base: +{} -{} ({} files)",
            diff.insertions, diff.deletions, diff.files_changed
        );
    } else {
        println!("Diff vs base: unavailable");
    }

    // Position
    println!(
        "Position:     {} ahead, {} behind {}",
        h.drift.ahead, h.drift.behind, h.drift.base_branch
    );

    // Merge status
    match h.conflict_status {
        ConflictStatus::Clean => println!("Merge:        Clean (no conflicts)"),
        ConflictStatus::Conflicts => println!("Merge:        Conflicts detected"),
        ConflictStatus::Unknown => println!("Merge:        Unknown (check failed)"),
    }

    // Readiness
    let readiness_detail = match readiness {
        MergeReadiness::NeedsRebase => {
            format!(
                "{} ({} behind {})",
                readiness, h.drift.behind, h.drift.base_branch
            )
        }
        _ => readiness.to_string(),
    };
    println!("Readiness:    {}", readiness_detail);
}

fn print_fleet_table(results: &[(BranchHealth, MergeReadiness)]) {
    // Dynamic column widths
    let branch_w = results
        .iter()
        .map(|(h, _)| h.branch.len())
        .max()
        .unwrap_or(6)
        .clamp(6, 30);
    let commits_w = 7;
    let diff_w = 12;
    let behind_w = 6;
    let conflicts_w = 9;
    let readiness_w = 18;

    // Header
    println!(
        "┌{}┬{}┬{}┬{}┬{}┬{}┐",
        "─".repeat(branch_w + 2),
        "─".repeat(commits_w + 2),
        "─".repeat(diff_w + 2),
        "─".repeat(behind_w + 2),
        "─".repeat(conflicts_w + 2),
        "─".repeat(readiness_w + 2),
    );
    println!(
        "│ {:<branch_w$} │ {:<commits_w$} │ {:<diff_w$} │ {:<behind_w$} │ {:<conflicts_w$} │ {:<readiness_w$} │",
        "Branch", "Commits", "Diff", "Behind", "Conflicts", "Readiness",
    );
    println!(
        "├{}┼{}┼{}┼{}┼{}┼{}┤",
        "─".repeat(branch_w + 2),
        "─".repeat(commits_w + 2),
        "─".repeat(diff_w + 2),
        "─".repeat(behind_w + 2),
        "─".repeat(conflicts_w + 2),
        "─".repeat(readiness_w + 2),
    );

    // Rows
    for (h, readiness) in results {
        let diff_str = h.diff_vs_base.as_ref().map_or_else(
            || "-".to_string(),
            |d| format!("+{} -{}", d.insertions, d.deletions),
        );
        let conflicts_str = match h.conflict_status {
            ConflictStatus::Clean => "Clean",
            ConflictStatus::Conflicts => "Yes",
            ConflictStatus::Unknown => "Unknown",
        };

        println!(
            "│ {:<branch_w$} │ {:<commits_w$} │ {:<diff_w$} │ {:<behind_w$} │ {:<conflicts_w$} │ {:<readiness_w$} │",
            truncate_str(&h.branch, branch_w),
            h.commit_activity.commits_since_base,
            truncate_str(&diff_str, diff_w),
            h.drift.behind,
            conflicts_str,
            truncate_str(&readiness.to_string(), readiness_w),
        );
    }

    // Footer
    println!(
        "└{}┴{}┴{}┴{}┴{}┴{}┘",
        "─".repeat(branch_w + 2),
        "─".repeat(commits_w + 2),
        "─".repeat(diff_w + 2),
        "─".repeat(behind_w + 2),
        "─".repeat(conflicts_w + 2),
        "─".repeat(readiness_w + 2),
    );
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kild_core::git::types::{BaseBranchDrift, CommitActivity, DiffStats};

    fn make_health(
        conflict_status: ConflictStatus,
        behind: usize,
        has_remote: bool,
    ) -> BranchHealth {
        BranchHealth {
            branch: "test".to_string(),
            created_at: "2026-02-09T10:00:00Z".to_string(),
            commit_activity: CommitActivity {
                commits_since_base: 3,
                last_commit_time: None,
            },
            drift: BaseBranchDrift {
                ahead: 3,
                behind,
                base_branch: "main".to_string(),
            },
            diff_vs_base: Some(DiffStats {
                insertions: 10,
                deletions: 2,
                files_changed: 1,
            }),
            conflict_status,
            has_remote,
        }
    }

    #[test]
    fn test_readiness_has_conflicts() {
        let h = make_health(ConflictStatus::Conflicts, 0, true);
        assert_eq!(
            compute_merge_readiness(&h, &None, None),
            MergeReadiness::HasConflicts
        );
    }

    #[test]
    fn test_readiness_conflict_check_failed() {
        let h = make_health(ConflictStatus::Unknown, 0, true);
        assert_eq!(
            compute_merge_readiness(&h, &None, None),
            MergeReadiness::HasConflicts
        );
    }

    #[test]
    fn test_readiness_needs_rebase() {
        let h = make_health(ConflictStatus::Clean, 5, true);
        assert_eq!(
            compute_merge_readiness(&h, &None, None),
            MergeReadiness::NeedsRebase
        );
    }

    #[test]
    fn test_readiness_ready_local() {
        let h = make_health(ConflictStatus::Clean, 0, false);
        assert_eq!(
            compute_merge_readiness(&h, &None, None),
            MergeReadiness::ReadyLocal
        );
    }

    #[test]
    fn test_readiness_needs_push() {
        let h = make_health(ConflictStatus::Clean, 0, true);
        let ws = WorktreeStatus {
            unpushed_commit_count: 3,
            has_remote_branch: true,
            ..Default::default()
        };
        assert_eq!(
            compute_merge_readiness(&h, &Some(ws), None),
            MergeReadiness::NeedsPush
        );
    }

    #[test]
    fn test_readiness_needs_push_never_pushed() {
        let h = make_health(ConflictStatus::Clean, 0, true);
        let ws = WorktreeStatus {
            unpushed_commit_count: 0,
            has_remote_branch: false,
            ..Default::default()
        };
        assert_eq!(
            compute_merge_readiness(&h, &Some(ws), None),
            MergeReadiness::NeedsPush
        );
    }

    #[test]
    fn test_readiness_needs_pr() {
        let h = make_health(ConflictStatus::Clean, 0, true);
        let ws = WorktreeStatus {
            unpushed_commit_count: 0,
            has_remote_branch: true,
            ..Default::default()
        };
        assert_eq!(
            compute_merge_readiness(&h, &Some(ws), None),
            MergeReadiness::NeedsPr
        );
    }

    #[test]
    fn test_readiness_ci_failing() {
        use kild_core::forge::types::{PrInfo, PrState, ReviewStatus};
        let h = make_health(ConflictStatus::Clean, 0, true);
        let ws = WorktreeStatus {
            unpushed_commit_count: 0,
            has_remote_branch: true,
            ..Default::default()
        };
        let pr = PrInfo {
            number: 1,
            url: "https://example.com/pull/1".to_string(),
            state: PrState::Open,
            ci_status: CiStatus::Failing,
            ci_summary: None,
            review_status: ReviewStatus::Unknown,
            review_summary: None,
            updated_at: "2026-02-09T12:00:00Z".to_string(),
        };
        assert_eq!(
            compute_merge_readiness(&h, &Some(ws), Some(&pr)),
            MergeReadiness::CiFailing
        );
    }

    #[test]
    fn test_readiness_ready() {
        use kild_core::forge::types::{PrInfo, PrState, ReviewStatus};
        let h = make_health(ConflictStatus::Clean, 0, true);
        let ws = WorktreeStatus {
            unpushed_commit_count: 0,
            has_remote_branch: true,
            ..Default::default()
        };
        let pr = PrInfo {
            number: 1,
            url: "https://example.com/pull/1".to_string(),
            state: PrState::Open,
            ci_status: CiStatus::Passing,
            ci_summary: None,
            review_status: ReviewStatus::Approved,
            review_summary: None,
            updated_at: "2026-02-09T12:00:00Z".to_string(),
        };
        assert_eq!(
            compute_merge_readiness(&h, &Some(ws), Some(&pr)),
            MergeReadiness::Ready
        );
    }

    #[test]
    fn test_readiness_display() {
        assert_eq!(MergeReadiness::Ready.to_string(), "Ready");
        assert_eq!(MergeReadiness::NeedsPush.to_string(), "Needs push");
        assert_eq!(MergeReadiness::NeedsRebase.to_string(), "Needs rebase");
        assert_eq!(MergeReadiness::HasConflicts.to_string(), "Has conflicts");
        assert_eq!(MergeReadiness::NeedsPr.to_string(), "Needs PR");
        assert_eq!(MergeReadiness::CiFailing.to_string(), "CI failing");
        assert_eq!(MergeReadiness::ReadyLocal.to_string(), "Ready (local)");
    }

    #[test]
    fn test_readiness_serde() {
        let json = serde_json::to_string(&MergeReadiness::NeedsRebase).unwrap();
        assert_eq!(json, "\"needs_rebase\"");

        let json = serde_json::to_string(&MergeReadiness::HasConflicts).unwrap();
        assert_eq!(json, "\"has_conflicts\"");

        let json = serde_json::to_string(&MergeReadiness::ReadyLocal).unwrap();
        assert_eq!(json, "\"ready_local\"");
    }

    #[test]
    fn test_truncate_str_exact_length() {
        assert_eq!(truncate_str("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_str_shorter() {
        assert_eq!(truncate_str("hi", 5), "hi");
    }

    #[test]
    fn test_truncate_str_longer() {
        assert_eq!(truncate_str("hello world", 8), "hello...");
    }

    #[test]
    fn test_truncate_str_zero_max() {
        assert_eq!(truncate_str("hello", 0), "...");
    }
}
