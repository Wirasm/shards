use std::path::Path;

use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::git;
use crate::sessions::{errors::SessionError, persistence, types::*};

/// Completes a kild by checking PR status, optionally deleting remote branch, and destroying the session.
///
/// # Arguments
/// * `name` - Branch name or kild identifier
///
/// # Returns
/// * `Ok(CompleteResult::RemoteDeleted)` - PR was merged and remote branch was deleted
/// * `Ok(CompleteResult::RemoteDeleteFailed)` - PR was merged but remote deletion failed (non-fatal)
/// * `Ok(CompleteResult::PrNotMerged)` - PR not merged, remote preserved for future merge
///
/// # Errors
/// Returns `SessionError::NotFound` if the session doesn't exist.
/// Returns `SessionError::UncommittedChanges` if the worktree has uncommitted changes.
/// Propagates errors from `destroy_session`.
/// Remote branch deletion errors are logged but do not fail the operation.
///
/// # Workflow Detection
/// - If PR is merged: attempts to delete remote branch (since gh merge --delete-branch would have failed due to worktree)
/// - If PR not merged: just destroys the local session, allowing user's subsequent merge to handle remote cleanup
pub fn complete_session(name: &str) -> Result<CompleteResult, SessionError> {
    info!(event = "core.session.complete_started", name = name);

    let config = Config::new();

    // 1. Find session by name to get branch info
    let session =
        persistence::find_session_by_name(&config.sessions_dir(), name)?.ok_or_else(|| {
            SessionError::NotFound {
                name: name.to_string(),
            }
        })?;

    let kild_branch = git::operations::kild_branch_name(name);

    // 2. Check if PR was merged (determines if we need to delete remote)
    // Skip PR check entirely for repos without a remote configured
    let pr_merged = if super::destroy::has_remote_configured(&session.worktree_path) {
        crate::forge::get_forge_backend(&session.worktree_path)
            .map(|backend| backend.is_pr_merged(&session.worktree_path, &kild_branch))
            .unwrap_or(false)
    } else {
        debug!(
            event = "core.session.complete_no_remote",
            branch = name,
            "No remote configured — skipping PR check"
        );
        false
    };

    info!(
        event = "core.session.complete_pr_status",
        branch = name,
        pr_merged = pr_merged
    );

    // 3. Determine the result based on PR status and remote deletion outcome
    let result = if !pr_merged {
        CompleteResult::PrNotMerged
    } else {
        match crate::git::cli::delete_remote_branch(&session.worktree_path, "origin", &kild_branch)
        {
            Ok(()) => {
                info!(
                    event = "core.session.complete_remote_deleted",
                    branch = kild_branch
                );
                CompleteResult::RemoteDeleted
            }
            Err(e) => {
                // Non-fatal: remote might already be deleted, not exist, or deletion failed
                warn!(
                    event = "core.session.complete_remote_delete_failed",
                    branch = kild_branch,
                    worktree_path = %session.worktree_path.display(),
                    error = %e
                );
                CompleteResult::RemoteDeleteFailed
            }
        }
    };

    // 4. Safety check: always block on uncommitted changes (no --force bypass for complete)
    let safety_info = super::destroy::get_destroy_safety_info(name)?;
    if safety_info.should_block() {
        error!(
            event = "core.session.complete_blocked",
            name = name,
            reason = "uncommitted_changes"
        );
        return Err(SessionError::UncommittedChanges {
            name: name.to_string(),
        });
    }

    // 5. Destroy the session (reuse existing logic, always non-force since we already
    //    verified the worktree is clean above)
    super::destroy::destroy_session(name, false)?;

    info!(
        event = "core.session.complete_completed",
        name = name,
        result = ?result
    );

    Ok(result)
}

/// Fetch rich PR info via the forge backend.
///
/// Delegates to `forge::get_forge_backend()` to determine the correct forge
/// and calls its `fetch_pr_info()` method.
///
/// Returns `None` if no forge detected, CLI unavailable, no PR, or parse error.
pub fn fetch_pr_info(worktree_path: &Path, branch: &str) -> Option<crate::forge::types::PrInfo> {
    crate::forge::get_forge_backend(worktree_path)
        .and_then(|backend| backend.fetch_pr_info(worktree_path, branch))
}

/// Read PR info for a session from the sidecar file.
///
/// Returns `None` if no PR info has been cached yet.
pub fn read_pr_info(session_id: &str) -> Option<crate::forge::types::PrInfo> {
    let config = Config::new();
    persistence::read_pr_info(&config.sessions_dir(), session_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complete_session_not_found() {
        let result = complete_session("non-existent");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SessionError::NotFound { .. }));
    }
}
