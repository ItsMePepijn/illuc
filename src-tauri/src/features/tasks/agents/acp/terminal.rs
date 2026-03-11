use super::utils::{spawn_terminal_reader, spawn_terminal_waiter};
#[cfg(target_os = "windows")]
use crate::utils::windows::build_wsl_process_command;
use agent_client_protocol::{CreateTerminalRequest, TerminalExitStatus};
use anyhow::{Context, Result};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::Path;
use std::process::{Child, Stdio};
#[cfg(not(target_os = "windows"))]
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

#[derive(Default)]
pub(crate) struct TerminalStore {
    terminals: Mutex<HashMap<String, LocalTerminal>>,
}

impl TerminalStore {
    pub(crate) fn create(
        &self,
        worktree_path: &Path,
        request: agent_client_protocol::CreateTerminalRequest,
    ) -> Result<agent_client_protocol::TerminalId> {
        let terminal_id = format!("terminal-{}", Uuid::new_v4());
        let terminal = LocalTerminal::spawn(worktree_path, request)?;
        self.terminals.lock().insert(terminal_id.clone(), terminal);
        Ok(agent_client_protocol::TerminalId::new(terminal_id))
    }

    pub(crate) fn output(
        &self,
        terminal_id: &agent_client_protocol::TerminalId,
    ) -> Result<agent_client_protocol::TerminalOutputResponse> {
        let terminals = self.terminals.lock();
        let terminal = terminals
            .get(terminal_id.0.as_ref())
            .context("terminal not found")?;
        Ok(terminal.output())
    }

    pub(crate) fn kill(&self, terminal_id: &agent_client_protocol::TerminalId) -> Result<()> {
        let terminals = self.terminals.lock();
        let terminal = terminals
            .get(terminal_id.0.as_ref())
            .context("terminal not found")?;
        terminal.kill()
    }

    pub(crate) fn release(&self, terminal_id: &agent_client_protocol::TerminalId) -> Result<()> {
        let terminal = self
            .terminals
            .lock()
            .remove(terminal_id.0.as_ref())
            .context("terminal not found")?;
        let _ = terminal.kill();
        Ok(())
    }

    pub(crate) async fn wait_for_exit(
        &self,
        terminal_id: &agent_client_protocol::TerminalId,
    ) -> Result<TerminalExitStatus> {
        loop {
            if let Some(exit_status) = {
                let terminals = self.terminals.lock();
                let terminal = terminals
                    .get(terminal_id.0.as_ref())
                    .context("terminal not found")?;
                terminal.exit_status()
            } {
                return Ok(exit_status);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

pub(crate) struct LocalTerminal {
    child: Arc<Mutex<Child>>,
    output: Arc<Mutex<String>>,
    output_byte_limit: Option<usize>,
    exit_status: Arc<Mutex<Option<TerminalExitStatus>>>,
}

impl LocalTerminal {
    pub(crate) fn spawn(worktree_path: &Path, request: CreateTerminalRequest) -> Result<Self> {
        #[cfg(target_os = "windows")]
        let mut command = {
            let arg_refs: Vec<&str> = request.args.iter().map(String::as_str).collect();
            build_wsl_process_command(
                request.cwd.as_deref().unwrap_or(worktree_path),
                &request.command,
                &arg_refs,
            )
        };
        #[cfg(not(target_os = "windows"))]
        let mut command = {
            let mut command = Command::new(&request.command);
            command.args(&request.args);
            command
        };

        #[cfg(not(target_os = "windows"))]
        {
            command.current_dir(request.cwd.as_deref().unwrap_or(worktree_path));
            command.envs(
                request
                    .env
                    .into_iter()
                    .map(|entry| (entry.name, entry.value)),
            );
        }
        command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        let mut child = command
            .spawn()
            .with_context(|| format!("failed to spawn terminal command {}", request.command))?;
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let output = Arc::new(Mutex::new(String::new()));
        let exit_status = Arc::new(Mutex::new(None));
        let output_byte_limit = request.output_byte_limit.map(|value| value as usize);

        if let Some(stdout) = stdout {
            spawn_terminal_reader(stdout, Arc::clone(&output), output_byte_limit);
        }
        if let Some(stderr) = stderr {
            spawn_terminal_reader(stderr, Arc::clone(&output), output_byte_limit);
        }

        let child = Arc::new(Mutex::new(child));
        spawn_terminal_waiter(Arc::clone(&child), Arc::clone(&exit_status));

        Ok(Self {
            child,
            output,
            output_byte_limit,
            exit_status,
        })
    }

    pub(crate) fn output(&self) -> agent_client_protocol::TerminalOutputResponse {
        let output = self.output.lock().clone();
        let truncated = self
            .output_byte_limit
            .map(|limit| output.len() >= limit)
            .unwrap_or(false);
        let mut response = agent_client_protocol::TerminalOutputResponse::new(output, truncated);
        if let Some(exit_status) = self.exit_status() {
            response = response.exit_status(exit_status);
        }
        response
    }

    pub(crate) fn exit_status(&self) -> Option<TerminalExitStatus> {
        self.exit_status.lock().clone()
    }

    pub(crate) fn kill(&self) -> Result<()> {
        self.child
            .lock()
            .kill()
            .context("failed to kill ACP terminal command")
    }
}
