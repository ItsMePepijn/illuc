mod agents;
mod diff_watcher;
pub(crate) mod events;
pub mod git;
pub mod management;
pub mod models;
mod repo;
pub mod review;
mod task_manager;
mod worktree;

pub(crate) use diff_watcher::DiffWatcher;
pub use task_manager::TaskManager;
pub(crate) use task_manager::{TaskRecord, TaskRuntime};

pub use models::diff_payload::{DiffLine, DiffLineType};
pub use models::TerminalKind;
pub use models::{AgentKind, BaseRepoInfo, DiffPayload, TaskStatus, TaskSummary};
pub use repo::handle_select_base_repo;

use crate::features::tasks::agents::codex::CodexAgent;
use crate::features::tasks::agents::copilot::CopilotAgent;
use crate::features::tasks::agents::Agent;
use portable_pty::CommandBuilder;
use std::path::Path;

pub(crate) const DEFAULT_SCREEN_ROWS: usize = 40;
pub(crate) const DEFAULT_SCREEN_COLS: usize = 120;
pub(crate) const DEFAULT_PTY_ROWS: u16 = 40;
pub(crate) const DEFAULT_PTY_COLS: u16 = 80;

pub(crate) fn build_agent(agent_kind: AgentKind) -> Box<dyn Agent> {
    match agent_kind {
        AgentKind::Codex => Box::new(CodexAgent::default()),
        AgentKind::Copilot => Box::new(CopilotAgent::default()),
    }
}

pub(crate) fn agent_label(agent_kind: AgentKind) -> &'static str {
    match agent_kind {
        AgentKind::Codex => "Codex",
        AgentKind::Copilot => "Copilot CLI",
    }
}

pub(crate) fn build_worktree_shell_command(worktree_path: &Path) -> CommandBuilder {
    #[cfg(target_os = "windows")]
    {
        let mut command = CommandBuilder::new("powershell.exe");
        command.arg("-NoLogo");
        command.cwd(worktree_path);
        command
    }
    #[cfg(not(target_os = "windows"))]
    {
        let shell = std::env::var("SHELL")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "bash".to_string());
        let mut command = CommandBuilder::new(shell);
        command.cwd(worktree_path);
        command
    }
}
