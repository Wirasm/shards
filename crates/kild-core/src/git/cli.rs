//! Centralized git CLI wrappers.
//!
//! All production `std::process::Command::new("git")` calls in kild-core live here.
//! Each function validates arguments, logs structured events, and maps errors consistently.
//!
//! **Why CLI instead of git2?** Operations like `fetch`, `rebase`, and `push --delete`
//! require authentication. The git CLI inherits the user's SSH agent and credential
//! helpers automatically, while git2 requires explicit credential callback setup.

use std::path::Path;

use tracing::{debug, error, info, warn};

use super::errors::GitError;

/// Validate a git argument to prevent injection.
///
/// Rejects values that start with `-` (option injection), contain control characters,
/// or contain `::` sequences (refspec injection).
pub fn validate_git_arg(value: &str, label: &str) -> Result<(), GitError> {
    if value.starts_with('-') {
        return Err(GitError::OperationFailed {
            message: format!("Invalid {label}: '{value}' (must not start with '-')"),
        });
    }
    if value.chars().any(|c| c.is_control()) {
        return Err(GitError::OperationFailed {
            message: format!("Invalid {label}: contains control characters"),
        });
    }
    if value.contains("::") {
        return Err(GitError::OperationFailed {
            message: format!("Invalid {label}: '::' sequences are not allowed"),
        });
    }
    Ok(())
}

/// Fetch a specific branch from a remote.
///
/// Uses `git fetch` CLI to inherit the user's SSH agent and credential helpers
/// with zero auth code in kild.
pub fn fetch(dir: &Path, remote: &str, branch: &str) -> Result<(), GitError> {
    validate_git_arg(remote, "remote name")?;
    validate_git_arg(branch, "branch name")?;

    info!(
        event = "core.git.cli.fetch_started",
        remote = remote,
        branch = branch,
        path = %dir.display()
    );

    let output = std::process::Command::new("git")
        .current_dir(dir)
        .args(["fetch", remote, branch])
        .output()
        .map_err(|e| GitError::FetchFailed {
            remote: remote.to_string(),
            message: format!("Failed to execute git: {}", e),
        })?;

    if output.status.success() {
        info!(
            event = "core.git.cli.fetch_completed",
            remote = remote,
            branch = branch
        );
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!(
            event = "core.git.cli.fetch_failed",
            remote = remote,
            branch = branch,
            stderr = %stderr.trim()
        );
        Err(GitError::FetchFailed {
            remote: remote.to_string(),
            message: stderr.trim().to_string(),
        })
    }
}

/// Delete a branch from a remote.
///
/// Uses `git push --delete` CLI because push operations require authentication
/// that the CLI inherits from the user's credential helpers.
///
/// Treats "branch already deleted" as success (idempotent).
pub fn delete_remote_branch(dir: &Path, remote: &str, branch: &str) -> Result<(), GitError> {
    validate_git_arg(remote, "remote name")?;
    validate_git_arg(branch, "branch name")?;

    info!(
        event = "core.git.cli.delete_remote_branch_started",
        remote = remote,
        branch = branch,
        path = %dir.display()
    );

    let output = std::process::Command::new("git")
        .current_dir(dir)
        .args(["push", remote, "--delete", branch])
        .output()
        .map_err(|e| GitError::RemoteBranchDeleteFailed {
            branch: branch.to_string(),
            message: format!("Failed to execute git in {}: {}", dir.display(), e),
        })?;

    if output.status.success() {
        info!(
            event = "core.git.cli.delete_remote_branch_completed",
            remote = remote,
            branch = branch
        );
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Common patterns for "branch doesn't exist" across git versions.
    // These indicate the branch is already gone — treat as success.
    let benign_patterns = [
        "remote ref does not exist",
        "unable to delete",
        "does not exist",
    ];

    let is_already_deleted = benign_patterns
        .iter()
        .any(|pattern| stderr.to_lowercase().contains(pattern));

    if is_already_deleted {
        info!(
            event = "core.git.cli.delete_remote_branch_already_deleted",
            remote = remote,
            branch = branch
        );
        Ok(())
    } else {
        debug!(
            event = "core.git.cli.delete_remote_branch_failed",
            remote = remote,
            branch = branch,
            stderr = %stderr.trim()
        );
        Err(GitError::RemoteBranchDeleteFailed {
            branch: branch.to_string(),
            message: stderr.trim().to_string(),
        })
    }
}

/// Rebase the current branch onto a base branch.
///
/// Uses `git rebase` CLI because rebase may trigger fetches or other operations
/// that require the user's credential helpers.
///
/// On conflict, auto-aborts the rebase to leave the worktree clean,
/// then returns `GitError::RebaseConflict` so the user can resolve manually.
pub fn rebase(dir: &Path, base_branch: &str) -> Result<(), GitError> {
    validate_git_arg(base_branch, "base branch")?;

    info!(
        event = "core.git.cli.rebase_started",
        base = base_branch,
        path = %dir.display()
    );

    let output = std::process::Command::new("git")
        .current_dir(dir)
        .args(["rebase", base_branch])
        .output()
        .map_err(|e| GitError::OperationFailed {
            message: format!("Failed to execute git rebase: {}", e),
        })?;

    if output.status.success() {
        info!(
            event = "core.git.cli.rebase_completed",
            base = base_branch,
            path = %dir.display()
        );
        return Ok(());
    }

    let code = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Detect conflicts: exit code 1 with conflict markers in stderr
    let is_conflict = code == 1
        && (stderr.contains("CONFLICT")
            || stderr.contains("failed to merge")
            || stderr.contains("could not apply"));

    if is_conflict {
        // Auto-abort to leave worktree clean
        let abort_result = std::process::Command::new("git")
            .current_dir(dir)
            .args(["rebase", "--abort"])
            .output();

        match abort_result {
            Ok(abort_output) if abort_output.status.success() => {
                info!(
                    event = "core.git.cli.rebase_abort_completed",
                    base = base_branch,
                    path = %dir.display()
                );
            }
            Ok(abort_output) => {
                let abort_stderr = String::from_utf8_lossy(&abort_output.stderr);
                error!(
                    event = "core.git.cli.rebase_abort_failed",
                    base = base_branch,
                    path = %dir.display(),
                    stderr = %abort_stderr.trim()
                );
                return Err(GitError::RebaseAbortFailed {
                    base_branch: base_branch.to_string(),
                    worktree_path: dir.to_path_buf(),
                    message: abort_stderr.trim().to_string(),
                });
            }
            Err(e) => {
                error!(
                    event = "core.git.cli.rebase_abort_failed",
                    base = base_branch,
                    path = %dir.display(),
                    error = %e
                );
                return Err(GitError::RebaseAbortFailed {
                    base_branch: base_branch.to_string(),
                    worktree_path: dir.to_path_buf(),
                    message: e.to_string(),
                });
            }
        }

        warn!(
            event = "core.git.cli.rebase_conflicts",
            base = base_branch,
            path = %dir.display()
        );
        return Err(GitError::RebaseConflict {
            base_branch: base_branch.to_string(),
            worktree_path: dir.to_path_buf(),
        });
    }

    // Non-conflict failure
    error!(
        event = "core.git.cli.rebase_failed",
        base = base_branch,
        path = %dir.display(),
        code = code,
        stderr = %stderr.trim()
    );
    Err(GitError::OperationFailed {
        message: format!("git rebase failed (exit {}): {}", code, stderr.trim()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_git_arg_rejects_dash_prefix() {
        let result = validate_git_arg("--evil", "test");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("must not start with '-'"));
    }

    #[test]
    fn test_validate_git_arg_rejects_control_chars() {
        let result = validate_git_arg("hello\x00world", "test");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("control characters"));
    }

    #[test]
    fn test_validate_git_arg_rejects_double_colon() {
        let result = validate_git_arg("refs::heads", "test");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("'::'"));
    }

    #[test]
    fn test_validate_git_arg_accepts_valid_values() {
        assert!(validate_git_arg("origin", "remote").is_ok());
        assert!(validate_git_arg("main", "branch").is_ok());
        assert!(validate_git_arg("kild/feature-auth", "branch").is_ok());
        assert!(validate_git_arg("refs/heads/main", "refspec").is_ok());
    }
}
