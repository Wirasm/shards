use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;
use crate::config::Config;

pub struct TerminalLauncher;

impl TerminalLauncher {
    pub fn launch_agent(worktree_path: &Path, agent_command: &[String], config: &Config) -> Result<()> {
        let command_str = agent_command.join(" ");
        let terminal = Self::resolve_terminal(&config.terminal)?;
        
        #[cfg(target_os = "macos")]
        {
            Self::launch_macos_terminal(worktree_path, &command_str, &terminal)
        }
        
        #[cfg(target_os = "linux")]
        {
            Self::launch_linux_terminal(worktree_path, &command_str, &terminal)
        }
        
        #[cfg(target_os = "windows")]
        {
            Self::launch_windows_terminal(worktree_path, &command_str, &terminal)
        }
    }

    fn resolve_terminal(terminal_config: &str) -> Result<String> {
        if terminal_config == "auto" {
            Self::detect_terminal()
        } else {
            Ok(terminal_config.to_string())
        }
    }

    fn detect_terminal() -> Result<String> {
        #[cfg(target_os = "macos")]
        {
            // Check if Ghostty is available
            if Command::new("ghostty").arg("--version").output().is_ok() {
                return Ok("ghostty".to_string());
            }
            Ok("terminal".to_string()) // Fallback to Terminal.app
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            Ok("default".to_string())
        }
    }

    #[cfg(target_os = "macos")]
    fn launch_macos_terminal(worktree_path: &Path, command: &str, terminal: &str) -> Result<()> {
        match terminal {
            "ghostty" => {
                let worktree_str = worktree_path.to_string_lossy();
                
                // Launch Ghostty with working directory
                Command::new("open")
                    .arg("-na")
                    .arg("Ghostty.app")
                    .arg("--args")
                    .arg(&format!("--working-directory={}", worktree_str))
                    .spawn()
                    .context("Failed to launch Ghostty")?;
                
                // If there's a command, use AppleScript to type it after a brief delay
                if !command.is_empty() {
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                    
                    let script = format!(
                        r#"tell application "Ghostty"
                            activate
                            delay 0.5
                            tell application "System Events"
                                keystroke "{}"
                                keystroke return
                            end tell
                        end tell"#,
                        command
                    );
                    
                    Command::new("osascript")
                        .arg("-e")
                        .arg(&script)
                        .spawn()
                        .context("Failed to send command to Ghostty")?;
                }
            }
            _ => {
                // Default to Terminal.app
                let script = format!(
                    r#"tell application "Terminal"
                        do script "cd '{}' && {}"
                        activate
                    end tell"#,
                    worktree_path.display(),
                    command
                );

                Command::new("osascript")
                    .arg("-e")
                    .arg(&script)
                    .spawn()
                    .context("Failed to launch Terminal.app")?;
            }
        }

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn launch_linux_terminal(worktree_path: &Path, command: &str, terminal: &str) -> Result<()> {
        // Simple fallback to gnome-terminal for now
        Command::new("gnome-terminal")
            .arg("--working-directory")
            .arg(worktree_path)
            .arg("--")
            .arg("bash")
            .arg("-c")
            .arg(command)
            .spawn()
            .context("Failed to launch terminal")?;
        
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn launch_windows_terminal(worktree_path: &Path, command: &str, terminal: &str) -> Result<()> {
        Command::new("cmd")
            .arg("/k")
            .arg(&format!("cd /d \"{}\" && {}", worktree_path.display(), command))
            .spawn()
            .context("Failed to launch terminal")?;
        
        Ok(())
    }
}
