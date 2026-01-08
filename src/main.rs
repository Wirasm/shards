mod git;
mod terminal;
mod registry;
mod config;

use clap::{Parser, Subcommand};
use anyhow::Result;
use git::GitManager;
use terminal::TerminalLauncher;
use registry::{SessionRegistry, ShardSession, SessionStatus};
use config::Config;
use std::time::SystemTime;
use dialoguer::Select;

#[derive(Parser)]
#[command(name = "shards")]
#[command(about = "Manage parallel AI development agents in isolated Git worktrees")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a new shard with an AI agent
    Start {
        /// Name of the shard
        name: String,
        /// Agent command to run (optional if using profiles)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        agent_command: Vec<String>,
        /// Use a specific agent profile
        #[arg(long)]
        agent: Option<String>,
        /// Override terminal to use (auto, terminal, ghostty)
        #[arg(long)]
        terminal: Option<String>,
    },
    /// List active shards
    List,
    /// Stop a shard
    Stop {
        /// Name of the shard to stop
        name: String,
    },
    /// Clean up orphaned shards and worktrees
    Cleanup,
    /// Show detailed information about a shard
    Info {
        /// Name of the shard
        name: String,
    },
}

fn resolve_agent_command(
    agent_command: Vec<String>,
    agent_profile: Option<String>,
    config: &Config,
) -> Result<Vec<String>> {
    // Priority: CLI command > --agent profile > default_agent > prompt user
    
    if !agent_command.is_empty() {
        return Ok(agent_command);
    }
    
    if let Some(profile) = agent_profile {
        if let Some(command) = config.agents.get(&profile) {
            return Ok(command.split_whitespace().map(|s| s.to_string()).collect());
        } else {
            anyhow::bail!("Agent profile '{}' not found", profile);
        }
    }
    
    if !config.default_agent.is_empty() {
        return Ok(config.default_agent.split_whitespace().map(|s| s.to_string()).collect());
    }
    
    // Prompt user to choose
    let mut options: Vec<String> = config.agents.keys().cloned().collect();
    options.sort();
    
    if options.is_empty() {
        anyhow::bail!("No agent profiles configured. Please specify a command or configure agents in ~/.shards/config.toml");
    }
    
    println!("No agent specified. Choose an agent:");
    let selection = Select::new()
        .items(&options)
        .default(0)
        .interact()?;
    
    let selected_profile = &options[selection];
    let command = config.agents.get(selected_profile).unwrap();
    Ok(command.split_whitespace().map(|s| s.to_string()).collect())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load()?;

    match cli.command {
        Commands::Start { name, agent_command, agent, terminal } => {
            let git_manager = GitManager::new()?;
            let registry = SessionRegistry::new()?;
            
            // Check if shard already exists
            if registry.get_session(&name)?.is_some() {
                anyhow::bail!("Shard '{}' already exists", name);
            }

            let resolved_command = resolve_agent_command(agent_command, agent, &config)?;
            let worktree_path = git_manager.create_worktree(&name)?;
            
            // Create session record
            let session = ShardSession {
                name: name.clone(),
                worktree_path: worktree_path.clone(),
                agent_command: resolved_command.clone(),
                created_at: SystemTime::now(),
                status: SessionStatus::Active,
            };
            
            registry.add_session(session)?;
            
            println!("Created worktree for shard '{}' at: {}", name, worktree_path.display());
            println!("Launching agent with command: {:?}", resolved_command);
            
            let mut config = config;
            if let Some(terminal_override) = terminal {
                config.terminal = terminal_override;
            }
            
            TerminalLauncher::launch_agent(&worktree_path, &resolved_command, &config)?;
            println!("Agent launched in new terminal window");
            
            Ok(())
        }
        Commands::List => {
            let registry = SessionRegistry::new()?;
            let sessions = registry.list_sessions()?;
            
            if sessions.is_empty() {
                println!("No active shards");
                return Ok(());
            }
            
            println!("Active shards:");
            for session in sessions {
                let status = match session.status {
                    SessionStatus::Active => "🟢 Active",
                    SessionStatus::Stopped => "🔴 Stopped",
                };
                
                println!("  {} - {} ({})", 
                    session.name, 
                    status,
                    session.agent_command.join(" ")
                );
                println!("    Path: {}", session.worktree_path.display());
            }
            
            Ok(())
        }
        Commands::Stop { name } => {
            let git_manager = GitManager::new()?;
            let registry = SessionRegistry::new()?;
            
            // Check if session exists
            if registry.get_session(&name)?.is_none() {
                anyhow::bail!("Shard '{}' not found", name);
            }
            
            // Update session status
            registry.update_session_status(&name, SessionStatus::Stopped)?;
            
            // Clean up worktree
            git_manager.cleanup_worktree(&name)?;
            
            // Remove from registry
            registry.remove_session(&name)?;
            
            println!("Stopped and cleaned up shard '{}'", name);
            Ok(())
        }
        Commands::Cleanup => {
            let git_manager = GitManager::new()?;
            let registry = SessionRegistry::new()?;
            let sessions = registry.list_sessions()?;
            
            let mut cleaned_count = 0;
            
            for session in sessions {
                // Check if worktree directory still exists
                if !session.worktree_path.exists() {
                    println!("Cleaning up orphaned session: {}", session.name);
                    registry.remove_session(&session.name)?;
                    cleaned_count += 1;
                }
            }
            
            // Also clean up any worktree directories without registry entries
            let shards_dir = std::env::current_dir()?.join(".shards");
            if shards_dir.exists() {
                for entry in std::fs::read_dir(&shards_dir)? {
                    let entry = entry?;
                    if entry.file_type()?.is_dir() {
                        let dir_name = entry.file_name().to_string_lossy().to_string();
                        if registry.get_session(&dir_name)?.is_none() {
                            println!("Cleaning up orphaned worktree: {}", dir_name);
                            git_manager.cleanup_worktree(&dir_name)?;
                            cleaned_count += 1;
                        }
                    }
                }
            }
            
            if cleaned_count == 0 {
                println!("No orphaned shards found");
            } else {
                println!("Cleaned up {} orphaned shard(s)", cleaned_count);
            }
            
            Ok(())
        }
        Commands::Info { name } => {
            let registry = SessionRegistry::new()?;
            
            if let Some(session) = registry.get_session(&name)? {
                println!("Shard: {}", session.name);
                println!("Status: {:?}", session.status);
                println!("Command: {}", session.agent_command.join(" "));
                println!("Worktree: {}", session.worktree_path.display());
                println!("Created: {:?}", session.created_at);
                
                // Check if worktree still exists
                if session.worktree_path.exists() {
                    println!("Worktree exists: ✅");
                } else {
                    println!("Worktree exists: ❌ (orphaned)");
                }
            } else {
                anyhow::bail!("Shard '{}' not found", name);
            }
            
            Ok(())
        }
    }
}
