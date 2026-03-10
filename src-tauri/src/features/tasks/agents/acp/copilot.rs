use super::config::AcpAgentConfig;
use crate::features::tasks::agents::copilot::{find_latest_session_id, resolve_session_cwd};
#[cfg(target_os = "windows")]
use crate::utils::windows::build_wsl_process_command;
use anyhow::Result;
use agent_client_protocol::{LoadSessionRequest, NewSessionRequest};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct CopilotAcpConfig;

impl AcpAgentConfig for CopilotAcpConfig {
    fn id(&self) -> &'static str {
        "copilot_gui"
    }

    fn title(&self) -> &'static str {
        "Copilot GUI"
    }

    fn build_command(&self, worktree_path: &Path) -> Command {
        let args = [
            "--acp",
            "--stdio",
            "--allow-all-tools",
            "--deny-tool",
            "shell(git push)",
        ];

        #[cfg(target_os = "windows")]
        {
            build_wsl_process_command(worktree_path, "copilot", &args)
        }

        #[cfg(not(target_os = "windows"))]
        {
            let mut command = Command::new("copilot");
            command.args(args);
            command.current_dir(worktree_path);
            command
        }
    }

    fn new_session_request(&self, worktree_path: &Path) -> Result<NewSessionRequest> {
        let session_cwd = session_cwd_path(worktree_path)?;
        Ok(NewSessionRequest::new(&session_cwd))
    }

    fn load_session_request(&self, worktree_path: &Path) -> Result<Option<LoadSessionRequest>> {
        let Some(session_id) = find_latest_session_id(worktree_path)? else {
            return Ok(None);
        };
        let session_cwd = session_cwd_path(worktree_path)?;
        Ok(Some(LoadSessionRequest::new(session_id, &session_cwd)))
    }
}

fn session_cwd_path(worktree_path: &Path) -> Result<PathBuf> {
    Ok(PathBuf::from(resolve_session_cwd(worktree_path)?))
}
