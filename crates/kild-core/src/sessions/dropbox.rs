//! Dropbox messaging protocol — fleet directory setup and protocol generation.
//!
//! The dropbox is a per-session directory at `~/.kild/fleet/<project_id>/<branch>/`
//! (where `<branch>` has `/` replaced with `_` for filesystem safety) containing
//! fleet protocol instructions and (in future phases) task files.
//! Created only for fleet-capable agents (claude) when fleet mode is active —
//! no-op for normal sessions and non-claude agents.

use kild_paths::KildPaths;
use tracing::{info, warn};

use super::fleet;

/// Ensure the dropbox directory exists with a current `protocol.md`.
///
/// Idempotent: creates directory if missing, overwrites `protocol.md` on every call
/// (picks up template changes). Best-effort: warns on failure, never blocks session
/// creation/opening. No-op for non-fleet-capable agents (mirrors `ensure_fleet_member`).
pub(super) fn ensure_dropbox(project_id: &str, branch: &str, agent: &str) {
    if !fleet::is_fleet_capable_agent(agent) || !fleet::fleet_mode_active(branch) {
        return;
    }

    let paths = match KildPaths::resolve() {
        Ok(p) => p,
        Err(e) => {
            warn!(
                event = "core.session.dropbox.paths_resolve_failed",
                error = %e,
            );
            eprintln!(
                "Warning: Failed to resolve kild paths — dropbox will not be created for '{}': {}",
                branch, e,
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
/// No-op for non-fleet-capable agents or when fleet mode is not active.
/// Best-effort: warns and skips injection if path resolution fails.
/// Called at the call site after `build_daemon_create_request` returns,
/// to avoid modifying that function's signature.
pub(super) fn inject_dropbox_env_vars(
    env_vars: &mut Vec<(String, String)>,
    project_id: &str,
    branch: &str,
    agent: &str,
) {
    if !fleet::is_fleet_capable_agent(agent) || !fleet::fleet_mode_active(branch) {
        return;
    }

    let paths = match KildPaths::resolve() {
        Ok(p) => p,
        Err(e) => {
            warn!(
                event = "core.session.dropbox.env_paths_resolve_failed",
                error = %e,
            );
            eprintln!(
                "Warning: Failed to resolve kild paths — KILD_DROPBOX will not be set for '{}': {}",
                branch, e,
            );
            return;
        }
    };

    let dropbox = paths.fleet_dropbox_dir(project_id, branch);
    let dropbox_str = match dropbox.to_str() {
        Some(s) => s.to_string(),
        None => {
            warn!(
                event = "core.session.dropbox.env_path_not_utf8",
                branch = branch,
                path = %dropbox.display(),
            );
            eprintln!(
                "Warning: Dropbox path is not valid UTF-8, KILD_DROPBOX will not be set: {}",
                dropbox.display(),
            );
            return;
        }
    };
    env_vars.push(("KILD_DROPBOX".to_string(), dropbox_str));

    if branch == fleet::BRAIN_BRANCH {
        let fleet_dir = paths.fleet_project_dir(project_id);
        let fleet_dir_str = match fleet_dir.to_str() {
            Some(s) => s.to_string(),
            None => {
                warn!(
                    event = "core.session.dropbox.env_fleet_dir_not_utf8",
                    branch = branch,
                    path = %fleet_dir.display(),
                );
                return;
            }
        };
        env_vars.push(("KILD_FLEET_DIR".to_string(), fleet_dir_str));
    }

    info!(
        event = "core.session.dropbox.env_injected",
        branch = branch,
        dropbox = %dropbox.display(),
    );
}

/// Clean up the dropbox directory for a session. Best-effort.
///
/// Always called — not gated on fleet mode or agent type. Returns immediately
/// if the directory does not exist (normal case for non-fleet sessions).
pub(super) fn cleanup_dropbox(project_id: &str, branch: &str) {
    let paths = match KildPaths::resolve() {
        Ok(p) => p,
        Err(e) => {
            warn!(
                event = "core.session.dropbox.cleanup_paths_failed",
                error = %e,
            );
            eprintln!(
                "Warning: Failed to resolve kild paths — dropbox for '{}' was not cleaned up: {}",
                branch, e,
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
- Do not modify task.md — it is written by the brain
"##
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    /// Serialize tests that mutate HOME and CLAUDE_CONFIG_DIR — env vars are process-global.
    static DROPBOX_ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Set up temp dirs with fleet mode active (team dir present + HOME overridden).
    ///
    /// Sets `CLAUDE_CONFIG_DIR` so `fleet_mode_active` returns true for non-brain branches,
    /// and `HOME` so `KildPaths::resolve()` returns a temp-based path. The callback receives
    /// the HOME dir; the dropbox will be created at `<home>/.kild/fleet/...`.
    fn with_fleet_env(test_name: &str, f: impl FnOnce(&std::path::Path)) {
        let _lock = DROPBOX_ENV_LOCK.lock().unwrap();
        let base = std::env::temp_dir().join(format!(
            "kild_dropbox_test_{}_{}",
            test_name,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);

        // Create the team dir so fleet_mode_active returns true.
        let claude_dir = base.join("claude_config");
        let team_dir = claude_dir.join("teams").join(fleet::BRAIN_BRANCH);
        std::fs::create_dir_all(&team_dir).unwrap();

        let home_dir = base.join("home");
        std::fs::create_dir_all(&home_dir).unwrap();

        // SAFETY: DROPBOX_ENV_LOCK serializes all env mutations in this module.
        unsafe {
            std::env::set_var("CLAUDE_CONFIG_DIR", &claude_dir);
            std::env::set_var("HOME", &home_dir);
        }
        f(&home_dir);
        let _ = std::fs::remove_dir_all(&base);
        // SAFETY: restoring env; lock still held.
        unsafe {
            std::env::remove_var("CLAUDE_CONFIG_DIR");
            std::env::remove_var("HOME");
        }
    }

    /// Set up temp dirs WITHOUT fleet mode active (no team dir).
    fn without_fleet_env(test_name: &str, f: impl FnOnce(&std::path::Path)) {
        let _lock = DROPBOX_ENV_LOCK.lock().unwrap();
        let base = std::env::temp_dir().join(format!(
            "kild_dropbox_no_fleet_{}_{}",
            test_name,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);

        let claude_dir = base.join("claude_config");
        std::fs::create_dir_all(&claude_dir).unwrap();

        let home_dir = base.join("home");
        std::fs::create_dir_all(&home_dir).unwrap();

        // SAFETY: DROPBOX_ENV_LOCK serializes all env mutations in this module.
        unsafe {
            std::env::set_var("CLAUDE_CONFIG_DIR", &claude_dir);
            std::env::set_var("HOME", &home_dir);
        }
        f(&home_dir);
        let _ = std::fs::remove_dir_all(&base);
        // SAFETY: restoring env; lock still held.
        unsafe {
            std::env::remove_var("CLAUDE_CONFIG_DIR");
            std::env::remove_var("HOME");
        }
    }

    // --- generate_protocol ---

    #[test]
    fn generate_protocol_contains_baked_absolute_paths() {
        let content =
            generate_protocol(std::path::Path::new("/home/user/.kild/fleet/abc/my-branch"));
        assert!(content.contains("/home/user/.kild/fleet/abc/my-branch"));
        assert!(content.contains("/home/user/.kild/fleet/abc/my-branch/task.md"));
        assert!(content.contains("/home/user/.kild/fleet/abc/my-branch/ack"));
        assert!(content.contains("/home/user/.kild/fleet/abc/my-branch/report.md"));
    }

    #[test]
    fn generate_protocol_contains_instructions() {
        let content = generate_protocol(std::path::Path::new("/tmp/dropbox"));
        assert!(content.contains("KILD Fleet Protocol"));
        assert!(content.contains("Read task.md"));
        assert!(content.contains("Write your results to report.md"));
        assert!(content.contains("Do not modify task.md"));
    }

    // --- ensure_dropbox ---

    #[test]
    fn ensure_dropbox_creates_directory_and_protocol() {
        with_fleet_env("creates_dir", |home| {
            let paths = KildPaths::resolve().unwrap();
            let dropbox_dir = paths.fleet_dropbox_dir("proj123", "my-branch");
            assert!(!dropbox_dir.exists());

            ensure_dropbox("proj123", "my-branch", "claude");

            assert!(dropbox_dir.exists());
            let protocol_path = dropbox_dir.join("protocol.md");
            assert!(protocol_path.exists());

            let written = std::fs::read_to_string(&protocol_path).unwrap();
            assert!(
                written.contains(&dropbox_dir.display().to_string()),
                "protocol.md should contain baked-in absolute paths under {}/",
                home.display(),
            );
        });
    }

    #[test]
    fn ensure_dropbox_is_idempotent() {
        with_fleet_env("idempotent", |_| {
            ensure_dropbox("proj123", "my-branch", "claude");
            ensure_dropbox("proj123", "my-branch", "claude");

            let paths = KildPaths::resolve().unwrap();
            let dropbox_dir = paths.fleet_dropbox_dir("proj123", "my-branch");
            assert!(dropbox_dir.join("protocol.md").exists());
        });
    }

    #[test]
    fn ensure_dropbox_noop_for_non_claude_agent() {
        with_fleet_env("non_claude", |_| {
            ensure_dropbox("proj123", "my-branch", "codex");

            let paths = KildPaths::resolve().unwrap();
            let dropbox_dir = paths.fleet_dropbox_dir("proj123", "my-branch");
            assert!(
                !dropbox_dir.exists(),
                "non-claude agent should not create dropbox"
            );
        });
    }

    #[test]
    fn ensure_dropbox_noop_when_fleet_not_active() {
        without_fleet_env("no_fleet", |_| {
            ensure_dropbox("proj123", "my-branch", "claude");

            let paths = KildPaths::resolve().unwrap();
            let dropbox_dir = paths.fleet_dropbox_dir("proj123", "my-branch");
            assert!(
                !dropbox_dir.exists(),
                "should not create dropbox when fleet is not active"
            );
        });
    }

    // --- cleanup_dropbox ---

    #[test]
    fn cleanup_dropbox_removes_existing_directory() {
        with_fleet_env("cleanup_removes", |_| {
            // Create the dropbox first
            ensure_dropbox("proj123", "my-branch", "claude");

            let paths = KildPaths::resolve().unwrap();
            let dropbox_dir = paths.fleet_dropbox_dir("proj123", "my-branch");
            assert!(dropbox_dir.exists());

            cleanup_dropbox("proj123", "my-branch");
            assert!(
                !dropbox_dir.exists(),
                "cleanup should remove the dropbox directory"
            );
        });
    }

    #[test]
    fn cleanup_dropbox_noop_when_missing() {
        with_fleet_env("cleanup_noop", |_| {
            // Call cleanup on a session that never had a dropbox — should not panic
            cleanup_dropbox("proj123", "never-existed");
        });
    }

    // --- inject_dropbox_env_vars ---

    #[test]
    fn inject_env_vars_pushes_kild_dropbox_for_worker() {
        with_fleet_env("inject_worker", |_| {
            let mut env_vars: Vec<(String, String)> = vec![];
            inject_dropbox_env_vars(&mut env_vars, "proj123", "worker", "claude");

            let keys: Vec<&str> = env_vars.iter().map(|(k, _)| k.as_str()).collect();
            assert!(
                keys.contains(&"KILD_DROPBOX"),
                "KILD_DROPBOX must be injected for worker"
            );
            assert!(
                !keys.contains(&"KILD_FLEET_DIR"),
                "KILD_FLEET_DIR must NOT be injected for non-brain worker"
            );
            assert!(env_vars[0].1.contains("fleet/proj123/worker"));
        });
    }

    #[test]
    fn inject_env_vars_brain_gets_fleet_dir() {
        with_fleet_env("inject_brain", |_| {
            let mut env_vars: Vec<(String, String)> = vec![];
            inject_dropbox_env_vars(&mut env_vars, "proj123", fleet::BRAIN_BRANCH, "claude");

            let keys: Vec<&str> = env_vars.iter().map(|(k, _)| k.as_str()).collect();
            assert!(keys.contains(&"KILD_DROPBOX"));
            assert!(
                keys.contains(&"KILD_FLEET_DIR"),
                "brain must get KILD_FLEET_DIR"
            );
            assert!(env_vars[1].1.contains("fleet/proj123"));
        });
    }

    #[test]
    fn inject_env_vars_noop_for_non_claude_agent() {
        with_fleet_env("inject_non_claude", |_| {
            let mut env_vars: Vec<(String, String)> = vec![];
            inject_dropbox_env_vars(&mut env_vars, "proj123", "worker", "codex");

            assert!(
                env_vars.is_empty(),
                "non-claude agent should not get dropbox env vars"
            );
        });
    }

    #[test]
    fn inject_env_vars_noop_when_fleet_not_active() {
        without_fleet_env("inject_no_fleet", |_| {
            let mut env_vars: Vec<(String, String)> = vec![];
            inject_dropbox_env_vars(&mut env_vars, "proj123", "worker", "claude");

            assert!(
                env_vars.is_empty(),
                "should not inject env vars when fleet is not active"
            );
        });
    }
}
