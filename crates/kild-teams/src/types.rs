//! Domain types for agent team state.
//!
//! Our own types decoupled from Claude Code's JSON format. Raw serde types
//! for Claude Code config are in `parser.rs`.

use serde::{Deserialize, Serialize};

/// Color assigned to a team member by Claude Code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TeamColor {
    Red,
    Blue,
    Green,
    Yellow,
    Purple,
    Orange,
    Pink,
    Cyan,
    Unknown,
}

impl TeamColor {
    /// Parse a color from Claude Code's string format (e.g., "blue", "magenta").
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "red" => Self::Red,
            "blue" => Self::Blue,
            "green" => Self::Green,
            "yellow" => Self::Yellow,
            "purple" | "magenta" => Self::Purple,
            "orange" | "colour208" => Self::Orange,
            "pink" | "colour205" => Self::Pink,
            "cyan" => Self::Cyan,
            _ => Self::Unknown,
        }
    }

    /// Parse from a tmux border_style string like "fg=blue".
    pub fn from_border_style(style: &str) -> Self {
        let color = style
            .split(',')
            .find_map(|part| part.strip_prefix("fg="))
            .unwrap_or("");
        Self::parse(color)
    }
}

/// A single member of an agent team.
#[derive(Debug, Clone)]
pub struct TeamMember {
    /// Display name (e.g., "researcher").
    pub name: String,
    /// Unique agent ID (e.g., "researcher@my-team"). `None` when discovered via shim registry.
    pub agent_id: Option<String>,
    /// Agent type (e.g., "general-purpose"). `None` when discovered via shim registry.
    pub agent_type: Option<String>,
    /// Color assigned by Claude Code.
    pub color: TeamColor,
    /// Tmux pane ID (e.g., "%1"). Empty string or "%0" identifies the leader.
    pub pane_id: String,
    /// Daemon session ID resolved from shim registry.
    pub daemon_session_id: Option<String>,
    /// Whether the member is currently active.
    pub is_active: bool,
}

impl TeamMember {
    /// Whether this member is the team leader (pane %0 or empty pane ID).
    pub fn is_leader(&self) -> bool {
        self.pane_id.is_empty() || self.pane_id == "%0"
    }
}

/// State of an agent team associated with a kild session.
#[derive(Debug, Clone)]
pub struct TeamState {
    /// Team name (directory name under `~/.claude/teams/`).
    pub team_name: String,
    /// Kild session ID this team belongs to (resolved from shim).
    pub kild_session_id: Option<String>,
    /// Team members including the leader.
    pub members: Vec<TeamMember>,
}

impl TeamState {
    /// Get all non-leader members (teammates).
    pub fn teammates(&self) -> impl Iterator<Item = &TeamMember> {
        self.members.iter().filter(|m| !m.is_leader())
    }

    /// Get the leader, if any.
    pub fn leader(&self) -> Option<&TeamMember> {
        self.members.iter().find(|m| m.is_leader())
    }
}

/// Events emitted when team state changes.
#[derive(Debug, Clone)]
pub enum TeamEvent {
    /// A team was created or updated.
    TeamUpdated { team_name: String, state: TeamState },
    /// A team was removed (config deleted).
    TeamRemoved { team_name: String },
}
