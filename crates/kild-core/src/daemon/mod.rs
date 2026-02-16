pub mod autostart;
pub mod client;
pub mod errors;

pub use autostart::ensure_daemon_running;
pub use errors::DaemonAutoStartError;

use std::path::PathBuf;

/// Default socket path for the daemon.
pub fn socket_path() -> PathBuf {
    kild_paths::KildPaths::resolve()
        .expect("HOME not set — cannot determine daemon socket path")
        .daemon_socket()
}

/// PID file path for the daemon process.
pub fn pid_file_path() -> PathBuf {
    kild_paths::KildPaths::resolve()
        .expect("HOME not set — cannot determine daemon PID path")
        .daemon_pid_file()
}

/// Find a sibling binary next to the currently running executable.
///
/// Looks for `binary_name` in the same directory as `std::env::current_exe()`.
/// Returns the full path if found, or a descriptive error if not.
pub fn find_sibling_binary(binary_name: &str) -> Result<PathBuf, String> {
    let our_binary =
        std::env::current_exe().map_err(|e| format!("could not determine binary path: {}", e))?;
    let bin_dir = our_binary
        .parent()
        .ok_or_else(|| format!("binary has no parent directory: {}", our_binary.display()))?;
    let sibling = bin_dir.join(binary_name);
    if !sibling.exists() {
        return Err(format!(
            "{} binary not found at {}. Run 'cargo build --all' to build it.",
            binary_name,
            sibling.display()
        ));
    }
    Ok(sibling)
}
