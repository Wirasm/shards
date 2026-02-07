//! AppleScript execution utilities for terminal backends.

use crate::terminal::errors::TerminalError;
#[cfg(target_os = "macos")]
use tracing::{debug, warn};

/// Execute an AppleScript and return the stdout as window ID.
#[cfg(target_os = "macos")]
pub fn execute_spawn_script(
    script: &str,
    terminal_name: &str,
) -> Result<Option<String>, TerminalError> {
    debug!(
        event = "core.terminal.applescript_executing",
        terminal = terminal_name
    );

    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|e| TerminalError::AppleScriptExecution {
            message: format!("Failed to execute osascript for {}: {}", terminal_name, e),
        })?;

    if !output.status.success() {
        let stderr = super::helpers::stderr_lossy(&output);
        return Err(TerminalError::SpawnFailed {
            message: format!("{} AppleScript failed: {}", terminal_name, stderr),
        });
    }

    let window_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

    debug!(
        event = "core.terminal.applescript_completed",
        terminal = terminal_name,
        window_id = %window_id
    );

    if window_id.is_empty() {
        Ok(None)
    } else {
        Ok(Some(window_id))
    }
}

/// Close a window via AppleScript (fire-and-forget, errors logged).
#[cfg(target_os = "macos")]
pub fn close_applescript_window(script: &str, terminal_name: &str, window_id: &str) {
    debug!(
        event = "core.terminal.close_started",
        terminal = terminal_name,
        window_id = %window_id
    );

    match std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
    {
        Ok(output) if output.status.success() => {
            debug!(
                event = "core.terminal.close_completed",
                terminal = terminal_name,
                window_id = %window_id
            );
        }
        Ok(output) => {
            let stderr = super::helpers::stderr_lossy(&output);
            warn!(
                event = "core.terminal.close_failed",
                terminal = terminal_name,
                window_id = %window_id,
                stderr = %stderr,
                message = "AppleScript close failed - window may remain open"
            );
        }
        Err(e) => {
            warn!(
                event = "core.terminal.close_failed",
                terminal = terminal_name,
                window_id = %window_id,
                error = %e,
                message = "Failed to execute osascript"
            );
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn execute_spawn_script(
    _script: &str,
    _terminal_name: &str,
) -> Result<Option<String>, TerminalError> {
    Ok(None)
}

#[cfg(not(target_os = "macos"))]
pub fn close_applescript_window(_script: &str, _terminal_name: &str, _window_id: &str) {}

/// Action type for AppleScript window operations that return Result.
pub enum AppleScriptAction {
    Focus,
    Hide,
}

/// Execute an AppleScript window action (focus or hide) with shared logging and error handling.
///
/// This unifies the near-identical implementations of `focus_applescript_window`
/// and `hide_applescript_window` into a single execution path.
#[cfg(target_os = "macos")]
fn execute_applescript_action(
    script: &str,
    terminal_name: &str,
    window_id: &str,
    action: AppleScriptAction,
) -> Result<(), TerminalError> {
    use tracing::{error, info};

    let (action_name, make_error): (&str, fn(String) -> TerminalError) = match action {
        AppleScriptAction::Focus => ("focus", |msg| TerminalError::FocusFailed { message: msg }),
        AppleScriptAction::Hide => ("hide", |msg| TerminalError::HideFailed { message: msg }),
    };

    let event_started = format!("core.terminal.{action_name}_started");
    debug!(
        event = %event_started,
        terminal = terminal_name,
        window_id = %window_id
    );

    match std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
    {
        Ok(output) if output.status.success() => {
            let event_completed = format!("core.terminal.{action_name}_completed");
            info!(
                event = %event_completed,
                terminal = terminal_name,
                window_id = %window_id
            );
            Ok(())
        }
        Ok(output) => {
            let stderr = super::helpers::stderr_lossy(&output);
            let event_failed = format!("core.terminal.{action_name}_failed");
            warn!(
                event = %event_failed,
                terminal = terminal_name,
                window_id = %window_id,
                stderr = %stderr
            );
            Err(make_error(format!(
                "{} {} failed for window {}: {}",
                terminal_name, action_name, window_id, stderr
            )))
        }
        Err(e) => {
            let event_failed = format!("core.terminal.{action_name}_failed");
            error!(
                event = %event_failed,
                terminal = terminal_name,
                window_id = %window_id,
                error = %e
            );
            Err(make_error(format!(
                "Failed to execute osascript for {} {}: {}",
                terminal_name, action_name, e
            )))
        }
    }
}

/// Focus a window via AppleScript.
///
/// Unlike `close_applescript_window` which is fire-and-forget, this returns a Result
/// so callers can report focus failures to the user.
#[cfg(target_os = "macos")]
pub fn focus_applescript_window(
    script: &str,
    terminal_name: &str,
    window_id: &str,
) -> Result<(), TerminalError> {
    execute_applescript_action(script, terminal_name, window_id, AppleScriptAction::Focus)
}

#[cfg(not(target_os = "macos"))]
pub fn focus_applescript_window(
    _script: &str,
    _terminal_name: &str,
    _window_id: &str,
) -> Result<(), TerminalError> {
    Err(TerminalError::FocusFailed {
        message: "Focus not supported on this platform".to_string(),
    })
}

/// Hide/minimize a window via AppleScript.
///
/// Like `focus_applescript_window`, this returns a Result so callers can report
/// hide failures to the user.
#[cfg(target_os = "macos")]
pub fn hide_applescript_window(
    script: &str,
    terminal_name: &str,
    window_id: &str,
) -> Result<(), TerminalError> {
    execute_applescript_action(script, terminal_name, window_id, AppleScriptAction::Hide)
}

#[cfg(not(target_os = "macos"))]
pub fn hide_applescript_window(
    _script: &str,
    _terminal_name: &str,
    _window_id: &str,
) -> Result<(), TerminalError> {
    Err(TerminalError::HideFailed {
        message: "Hide not supported on this platform".to_string(),
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_execute_spawn_script_non_macos() {
        #[cfg(not(target_os = "macos"))]
        {
            use super::execute_spawn_script;
            let result = execute_spawn_script("test script", "test_terminal");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), None);
        }
    }

    #[test]
    fn test_close_applescript_window_non_macos_does_not_panic() {
        #[cfg(not(target_os = "macos"))]
        {
            use super::close_applescript_window;
            // Should not panic on non-macOS
            close_applescript_window("test script", "test_terminal", "123");
        }
    }
}
