use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    Codex,
    CodexGui,
    CopilotGui,
    Copilot,
    OpenCode,
}

impl AgentKind {
    pub const ALL: [Self; 5] = [
        Self::CodexGui,
        Self::CopilotGui,
        Self::Codex,
        Self::Copilot,
        Self::OpenCode,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex CLI",
            Self::CodexGui => "Codex GUI",
            Self::CopilotGui => "Copilot GUI",
            Self::Copilot => "Copilot CLI",
            Self::OpenCode => "OpenCode",
        }
    }

    pub fn executable(self) -> &'static str {
        match self {
            Self::Codex | Self::CodexGui => "codex",
            Self::Copilot | Self::CopilotGui => "copilot",
            Self::OpenCode => "opencode",
        }
    }

    pub fn is_installed(self) -> bool {
        command_exists(self.executable())
    }
}

fn command_exists(command: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        Command::new("wsl.exe")
            .args([
                "--",
                "bash",
                "-lc",
                &format!("command -v {command} >/dev/null 2>&1"),
            ])
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new("sh")
            .args(["-lc", &format!("command -v {command} >/dev/null 2>&1")])
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }
}
