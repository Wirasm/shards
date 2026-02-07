//! Forge backend trait definition.

use std::path::Path;

use crate::forge::types::{PrCheckResult, PrInfo};

/// Trait defining the interface for forge (code hosting) backends.
///
/// Each supported forge (GitHub, GitLab, etc.) implements this trait
/// to provide platform-specific PR/MR operations.
pub trait ForgeBackend: Send + Sync {
    /// The canonical name of this forge (e.g., "github", "gitlab").
    fn name(&self) -> &'static str;

    /// The user-facing display name (e.g., "GitHub", "GitLab").
    fn display_name(&self) -> &'static str;

    /// Whether this forge's CLI tooling is available on the system.
    fn is_available(&self) -> bool;

    /// Check if a merged PR/MR exists for the given branch.
    ///
    /// Returns true if PR exists and is merged, false otherwise (including errors).
    fn is_pr_merged(&self, worktree_path: &Path, branch: &str) -> bool;

    /// Check if any PR/MR exists for the given branch.
    fn check_pr_exists(&self, worktree_path: &Path, branch: &str) -> PrCheckResult;

    /// Fetch rich PR/MR info (number, URL, state, CI, reviews).
    fn fetch_pr_info(&self, worktree_path: &Path, branch: &str) -> Option<PrInfo>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockForge;

    impl ForgeBackend for MockForge {
        fn name(&self) -> &'static str {
            "mock"
        }

        fn display_name(&self) -> &'static str {
            "Mock Forge"
        }

        fn is_available(&self) -> bool {
            true
        }

        fn is_pr_merged(&self, _worktree_path: &Path, _branch: &str) -> bool {
            false
        }

        fn check_pr_exists(&self, _worktree_path: &Path, _branch: &str) -> PrCheckResult {
            PrCheckResult::Unavailable
        }

        fn fetch_pr_info(&self, _worktree_path: &Path, _branch: &str) -> Option<PrInfo> {
            None
        }
    }

    #[test]
    fn test_forge_backend_basic_methods() {
        let backend = MockForge;
        assert_eq!(backend.name(), "mock");
        assert_eq!(backend.display_name(), "Mock Forge");
        assert!(backend.is_available());
    }

    #[test]
    fn test_forge_backend_pr_methods() {
        let backend = MockForge;
        let path = Path::new("/tmp");
        assert!(!backend.is_pr_merged(path, "test"));
        assert!(backend.check_pr_exists(path, "test").is_unavailable());
        assert!(backend.fetch_pr_info(path, "test").is_none());
    }
}
