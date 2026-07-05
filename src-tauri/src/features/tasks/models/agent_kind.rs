use crate::features::tasks::agent_command::command_exists;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    Codex,
    CodexGui,
    CopilotGui,
    Copilot,
    OpenCode,
    ClaudeCode,
}

impl AgentKind {
    pub const ALL: [Self; 6] = [
        Self::CodexGui,
        Self::CopilotGui,
        Self::Codex,
        Self::Copilot,
        Self::ClaudeCode,
        Self::OpenCode,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex CLI",
            Self::CodexGui => "Codex GUI",
            Self::CopilotGui => "Copilot GUI",
            Self::Copilot => "Copilot CLI",
            Self::OpenCode => "OpenCode",
            Self::ClaudeCode => "Claude Code",
        }
    }

    pub fn executable(self) -> &'static str {
        match self {
            Self::Codex | Self::CodexGui => "codex",
            Self::Copilot | Self::CopilotGui => "copilot",
            Self::OpenCode => "opencode",
            Self::ClaudeCode => "claude",
        }
    }

    pub fn is_installed(self) -> bool {
        command_exists(self.executable())
    }
}
