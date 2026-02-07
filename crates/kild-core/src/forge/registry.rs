//! Forge registry for managing and looking up forge backends.

use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

use tracing::debug;

use super::backends::GitHubBackend;
use super::traits::ForgeBackend;
use super::types::ForgeType;

/// Global registry of all supported forge backends.
static REGISTRY: LazyLock<ForgeRegistry> = LazyLock::new(ForgeRegistry::new);

/// Registry that manages all forge backend implementations.
struct ForgeRegistry {
    backends: HashMap<ForgeType, Box<dyn ForgeBackend>>,
}

impl ForgeRegistry {
    fn new() -> Self {
        let mut backends: HashMap<ForgeType, Box<dyn ForgeBackend>> = HashMap::new();
        backends.insert(ForgeType::GitHub, Box::new(GitHubBackend));
        Self { backends }
    }

    fn get(&self, forge_type: &ForgeType) -> Option<&dyn ForgeBackend> {
        self.backends.get(forge_type).map(|b| b.as_ref())
    }
}

/// Get a reference to a forge backend by type.
pub fn get_backend(forge_type: &ForgeType) -> Option<&'static dyn ForgeBackend> {
    REGISTRY.get(forge_type)
}

/// Detect the forge type from the git remote URL.
///
/// Opens the repository at `worktree_path`, reads the "origin" remote URL,
/// and matches known forge hosts. Returns `None` for unknown hosts.
pub fn detect_forge(worktree_path: &Path) -> Option<ForgeType> {
    let repo = match git2::Repository::open(worktree_path) {
        Ok(r) => r,
        Err(e) => {
            debug!(
                event = "core.forge.detect_repo_open_failed",
                path = %worktree_path.display(),
                error = %e
            );
            return None;
        }
    };

    let remote = match repo.find_remote("origin") {
        Ok(r) => r,
        Err(e) => {
            debug!(
                event = "core.forge.detect_no_origin",
                path = %worktree_path.display(),
                error = %e
            );
            return None;
        }
    };

    let url = remote.url().unwrap_or("");

    if url.contains("github.com") {
        debug!(event = "core.forge.detected", forge = "github", url = url);
        Some(ForgeType::GitHub)
    } else {
        debug!(event = "core.forge.detect_unknown_host", url = url);
        None
    }
}

/// Convenience function: detect the forge for a repo and return its backend.
///
/// Combines `detect_forge()` + `get_backend()` + `is_available()` check.
/// Returns `None` if no forge detected, backend not registered, or CLI not available.
pub fn get_forge_backend(worktree_path: &Path) -> Option<&'static dyn ForgeBackend> {
    let forge_type = detect_forge(worktree_path)?;
    let backend = get_backend(&forge_type)?;
    if backend.is_available() {
        Some(backend)
    } else {
        debug!(
            event = "core.forge.cli_not_available",
            forge = backend.name()
        );
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_backend_github() {
        let backend = get_backend(&ForgeType::GitHub);
        assert!(backend.is_some());
        assert_eq!(backend.unwrap().name(), "github");
    }

    #[test]
    fn test_registry_contains_github() {
        let backend = get_backend(&ForgeType::GitHub);
        assert!(backend.is_some(), "Registry should contain GitHub backend");
    }

    #[test]
    fn test_all_registered_backends_have_correct_names() {
        let checks = [(ForgeType::GitHub, "github")];
        for (forge_type, expected_name) in checks {
            let backend = get_backend(&forge_type).unwrap();
            assert_eq!(
                backend.name(),
                expected_name,
                "Backend for {:?} should have name '{}'",
                forge_type,
                expected_name
            );
        }
    }

    #[test]
    fn test_detect_forge_nonexistent_path() {
        let result = detect_forge(Path::new("/nonexistent/path/that/does/not/exist"));
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_forge_does_not_panic() {
        // Should never panic regardless of input
        let _ = detect_forge(Path::new("/tmp"));
    }
}
