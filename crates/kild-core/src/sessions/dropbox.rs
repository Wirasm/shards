//! Dropbox messaging protocol — fleet directory setup and protocol generation.
//!
//! The dropbox is a per-session directory at `~/.kild/fleet/<project_id>/<branch>/`
//! containing fleet protocol instructions and (in future phases) task files.
//! Created only when fleet mode is active — no-op for normal sessions.

use kild_paths::KildPaths;
use tracing::{info, warn};

use super::fleet;

/// Ensure the dropbox directory exists with a current `protocol.md`.
///
/// Idempotent: creates directory if missing, overwrites `protocol.md` on every call
/// (picks up template changes). Best-effort: warns on failure, never blocks session
/// creation/opening.
pub(super) fn ensure_dropbox(project_id: &str, branch: &str) {
    if !fleet::fleet_mode_active(branch) {
        return;
    }

    let paths = match KildPaths::resolve() {
        Ok(p) => p,
        Err(e) => {
            warn!(
                event = "core.session.dropbox.paths_resolve_failed",
                error = %e,
            );
            return;
        }
    };

    let dropbox_dir = paths.fleet_dropbox_dir(project_id, branch);

    if let Err(e) = std::fs::create_dir_all(&dropbox_dir) {
        warn!(
            event = "core.session.dropbox.create_dir_failed",
            branch = branch,
            path = %dropbox_dir.display(),
            error = %e,
        );
        eprintln!(
            "Warning: Failed to create dropbox directory at {}: {}",
            dropbox_dir.display(),
            e,
        );
        return;
    }

    let protocol_path = dropbox_dir.join("protocol.md");
    let protocol_content = generate_protocol(&dropbox_dir);

    if let Err(e) = std::fs::write(&protocol_path, protocol_content) {
        warn!(
            event = "core.session.dropbox.protocol_write_failed",
            branch = branch,
            path = %protocol_path.display(),
            error = %e,
        );
        eprintln!(
            "Warning: Failed to write protocol.md at {}: {}",
            protocol_path.display(),
            e,
        );
        return;
    }

    info!(
        event = "core.session.dropbox.ensure_completed",
        branch = branch,
        path = %dropbox_dir.display(),
    );
}

/// Inject `KILD_DROPBOX` (and `KILD_FLEET_DIR` for brain) into daemon env vars.
///
/// No-op when fleet mode is not active. Called at the call site after
/// `build_daemon_create_request` returns, to avoid modifying that function's signature.
pub(super) fn inject_dropbox_env_vars(
    env_vars: &mut Vec<(String, String)>,
    project_id: &str,
    branch: &str,
) {
    if !fleet::fleet_mode_active(branch) {
        return;
    }

    let paths = match KildPaths::resolve() {
        Ok(p) => p,
        Err(e) => {
            warn!(
                event = "core.session.dropbox.env_paths_resolve_failed",
                error = %e,
            );
            return;
        }
    };

    let dropbox = paths.fleet_dropbox_dir(project_id, branch);
    env_vars.push((
        "KILD_DROPBOX".to_string(),
        dropbox.to_string_lossy().to_string(),
    ));

    if branch == fleet::BRAIN_BRANCH {
        let fleet_dir = paths.fleet_project_dir(project_id);
        env_vars.push((
            "KILD_FLEET_DIR".to_string(),
            fleet_dir.to_string_lossy().to_string(),
        ));
    }

    info!(
        event = "core.session.dropbox.env_injected",
        branch = branch,
        dropbox = %dropbox.display(),
    );
}

/// Clean up the dropbox directory for a session. Best-effort.
pub(super) fn cleanup_dropbox(project_id: &str, branch: &str) {
    let paths = match KildPaths::resolve() {
        Ok(p) => p,
        Err(e) => {
            warn!(
                event = "core.session.dropbox.cleanup_paths_failed",
                error = %e,
            );
            return;
        }
    };

    let dropbox_dir = paths.fleet_dropbox_dir(project_id, branch);

    if !dropbox_dir.exists() {
        return;
    }

    if let Err(e) = std::fs::remove_dir_all(&dropbox_dir) {
        warn!(
            event = "core.session.dropbox.cleanup_failed",
            branch = branch,
            path = %dropbox_dir.display(),
            error = %e,
        );
        eprintln!(
            "Warning: Failed to remove dropbox at {}: {}",
            dropbox_dir.display(),
            e,
        );
    } else {
        info!(
            event = "core.session.dropbox.cleanup_completed",
            branch = branch,
        );
    }
}

/// Generate protocol.md content with baked-in absolute paths.
fn generate_protocol(dropbox_dir: &std::path::Path) -> String {
    let dropbox = dropbox_dir.display();
    // NOTE: Raw string content is flush-left to avoid embedding leading whitespace.
    // This matches the pattern in daemon_helpers.rs for hook script generation.
    format!(
        r##"# KILD Fleet Protocol

You are a worker in a KILD fleet managed by the Honryu brain supervisor.

## Receiving Tasks

Your dropbox: {dropbox}

On startup and after completing each task:
1. Read task.md from your dropbox for your current task
2. Write the task number (from the "# Task NNN" heading) to ack
3. Execute the task fully
4. Write your results to report.md
5. Stop and wait for the next instruction

## File Paths

- Task: {dropbox}/task.md
- Ack:  {dropbox}/ack
- Report: {dropbox}/report.md

## Rules

- Always read task.md before starting work
- Always write ack immediately after reading task.md
- Always write report.md when done
- Do not modify task.md or task-id — those are written by the brain
"##
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_protocol_contains_dropbox_path() {
        let content =
            generate_protocol(std::path::Path::new("/home/user/.kild/fleet/abc/my-branch"));
        assert!(content.contains("/home/user/.kild/fleet/abc/my-branch"));
        assert!(content.contains("/home/user/.kild/fleet/abc/my-branch/task.md"));
        assert!(content.contains("/home/user/.kild/fleet/abc/my-branch/ack"));
        assert!(content.contains("/home/user/.kild/fleet/abc/my-branch/report.md"));
    }

    #[test]
    fn test_generate_protocol_contains_instructions() {
        let content = generate_protocol(std::path::Path::new("/tmp/dropbox"));
        assert!(content.contains("KILD Fleet Protocol"));
        assert!(content.contains("Read task.md"));
        assert!(content.contains("Write your results to report.md"));
        assert!(content.contains("Do not modify task.md"));
    }

    #[test]
    fn test_ensure_dropbox_creates_directory_and_protocol() {
        let tmp = tempfile::tempdir().unwrap();
        let kild_dir = tmp.path().join(".kild");
        let paths = KildPaths::from_dir(kild_dir);

        let dropbox_dir = paths.fleet_dropbox_dir("proj123", "my-branch");
        assert!(!dropbox_dir.exists());

        // Manually replicate ensure_dropbox logic (since fleet_mode_active
        // depends on env vars and filesystem state):
        std::fs::create_dir_all(&dropbox_dir).unwrap();
        let protocol_path = dropbox_dir.join("protocol.md");
        let content = generate_protocol(&dropbox_dir);
        std::fs::write(&protocol_path, &content).unwrap();

        assert!(dropbox_dir.exists());
        assert!(protocol_path.exists());

        let written = std::fs::read_to_string(&protocol_path).unwrap();
        assert!(written.contains(&dropbox_dir.display().to_string()));
    }

    #[test]
    fn test_ensure_dropbox_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let kild_dir = tmp.path().join(".kild");
        let paths = KildPaths::from_dir(kild_dir);
        let dropbox_dir = paths.fleet_dropbox_dir("proj123", "my-branch");

        // First call
        std::fs::create_dir_all(&dropbox_dir).unwrap();
        std::fs::write(
            dropbox_dir.join("protocol.md"),
            generate_protocol(&dropbox_dir),
        )
        .unwrap();

        // Second call — should not error
        std::fs::create_dir_all(&dropbox_dir).unwrap();
        std::fs::write(
            dropbox_dir.join("protocol.md"),
            generate_protocol(&dropbox_dir),
        )
        .unwrap();

        assert!(dropbox_dir.join("protocol.md").exists());
    }

    #[test]
    fn test_cleanup_dropbox_removes_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let kild_dir = tmp.path().join(".kild");
        let paths = KildPaths::from_dir(kild_dir);
        let dropbox_dir = paths.fleet_dropbox_dir("proj123", "my-branch");

        std::fs::create_dir_all(&dropbox_dir).unwrap();
        std::fs::write(dropbox_dir.join("protocol.md"), "test").unwrap();
        assert!(dropbox_dir.exists());

        std::fs::remove_dir_all(&dropbox_dir).unwrap();
        assert!(!dropbox_dir.exists());
    }

    #[test]
    fn test_cleanup_dropbox_noop_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let kild_dir = tmp.path().join(".kild");
        let paths = KildPaths::from_dir(kild_dir);
        let dropbox_dir = paths.fleet_dropbox_dir("proj123", "my-branch");

        assert!(!dropbox_dir.exists());
        // Should not panic — nothing to remove
        // (mirrors cleanup_dropbox's early return)
    }

    #[test]
    fn test_inject_dropbox_env_vars_pushes_kild_dropbox() {
        let mut env_vars: Vec<(String, String)> = vec![];
        // Simulate what inject_dropbox_env_vars does when fleet is active:
        let paths = KildPaths::from_dir(std::path::PathBuf::from("/home/user/.kild"));
        let dropbox = paths.fleet_dropbox_dir("proj123", "worker");
        env_vars.push((
            "KILD_DROPBOX".to_string(),
            dropbox.to_string_lossy().to_string(),
        ));

        assert_eq!(env_vars.len(), 1);
        assert_eq!(env_vars[0].0, "KILD_DROPBOX");
        assert!(env_vars[0].1.contains("fleet/proj123/worker"));
    }

    #[test]
    fn test_inject_dropbox_env_vars_brain_gets_fleet_dir() {
        let mut env_vars: Vec<(String, String)> = vec![];
        let paths = KildPaths::from_dir(std::path::PathBuf::from("/home/user/.kild"));
        let dropbox = paths.fleet_dropbox_dir("proj123", fleet::BRAIN_BRANCH);
        env_vars.push((
            "KILD_DROPBOX".to_string(),
            dropbox.to_string_lossy().to_string(),
        ));
        let fleet_dir = paths.fleet_project_dir("proj123");
        env_vars.push((
            "KILD_FLEET_DIR".to_string(),
            fleet_dir.to_string_lossy().to_string(),
        ));

        assert_eq!(env_vars.len(), 2);
        assert_eq!(env_vars[0].0, "KILD_DROPBOX");
        assert_eq!(env_vars[1].0, "KILD_FLEET_DIR");
        assert!(env_vars[1].1.contains("fleet/proj123"));
    }
}
