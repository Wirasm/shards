use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

pub struct TerminalLauncher;

impl TerminalLauncher {
    pub fn launch_agent(worktree_path: &Path, agent_command: &[String]) -> Result<()> {
        let command_str = agent_command.join(" ");
        
        #[cfg(target_os = "macos")]
        {
            Self::launch_macos_terminal(worktree_path, &command_str)
        }
        
        #[cfg(target_os = "linux")]
        {
            Self::launch_linux_terminal(worktree_path, &command_str)
        }
        
        #[cfg(target_os = "windows")]
        {
            Self::launch_windows_terminal(worktree_path, &command_str)
        }
    }

    #[cfg(target_os = "macos")]
    fn launch_macos_terminal(worktree_path: &Path, command: &str) -> Result<()> {
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

        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn launch_linux_terminal(worktree_path: &Path, command: &str) -> Result<()> {
        // Try common Linux terminal emulators
        let terminals = ["gnome-terminal", "konsole", "xterm", "alacritty", "kitty"];
        
        for terminal in &terminals {
            if Command::new("which").arg(terminal).output().is_ok() {
                match terminal {
                    &"gnome-terminal" => {
                        Command::new(terminal)
                            .arg("--working-directory")
                            .arg(worktree_path)
                            .arg("--")
                            .arg("bash")
                            .arg("-c")
                            .arg(command)
                            .spawn()?;
                    }
                    &"konsole" => {
                        Command::new(terminal)
                            .arg("--workdir")
                            .arg(worktree_path)
                            .arg("-e")
                            .arg("bash")
                            .arg("-c")
                            .arg(command)
                            .spawn()?;
                    }
                    _ => {
                        Command::new(terminal)
                            .arg("-e")
                            .arg("bash")
                            .arg("-c")
                            .arg(&format!("cd '{}' && {}", worktree_path.display(), command))
                            .spawn()?;
                    }
                }
                return Ok(());
            }
        }
        
        anyhow::bail!("No supported terminal emulator found");
    }

    #[cfg(target_os = "windows")]
    fn launch_windows_terminal(worktree_path: &Path, command: &str) -> Result<()> {
        // Try Windows Terminal first, then fall back to cmd
        if Command::new("wt").arg("--version").output().is_ok() {
            Command::new("wt")
                .arg("-d")
                .arg(worktree_path)
                .arg("cmd")
                .arg("/k")
                .arg(command)
                .spawn()?;
        } else {
            Command::new("cmd")
                .arg("/k")
                .arg(&format!("cd /d \"{}\" && {}", worktree_path.display(), command))
                .spawn()?;
        }
        
        Ok(())
    }
}
