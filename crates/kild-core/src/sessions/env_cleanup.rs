//! Environment variable cleanup for spawned agent sessions.
//!
//! When kild is invoked from inside an existing agent session (e.g. Claude Code),
//! the parent's nesting-detection env vars leak into spawned terminals and cause
//! agents to refuse to start. This module defines the vars to strip.

/// Environment variables to remove when spawning agent sessions.
///
/// These are nesting-detection vars set by AI agents to prevent accidental
/// recursive launches. KILD intentionally spawns isolated sessions, so these
/// must be stripped.
pub const ENV_VARS_TO_STRIP: &[&str] = &[
    // Claude Code sets this to detect nested sessions
    "CLAUDECODE",
];
