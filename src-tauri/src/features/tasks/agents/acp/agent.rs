use super::client::AcpClientHandler;
use super::command::AcpCommand;
use super::config::AcpAgentConfig;
use super::runtime::{run_acp_runtime, set_exit_code};
use super::state::{AcpAgentState, SharedAcpAgentState};
use crate::features::tasks::agents::{AcpAgent as AcpAgentTrait, Agent, AgentCallbacks};
use anyhow::{anyhow, Context, Result};
use parking_lot::Mutex;
use serde_json::Value;
use std::sync::mpsc::{self, SyncSender};
use std::sync::Arc;
use tokio::runtime::Builder;
use tokio::sync::mpsc as tokio_mpsc;
use tokio::task::LocalSet;

pub struct AcpAgent<C: AcpAgentConfig> {
    config: C,
    state: SharedAcpAgentState,
}

impl<C: AcpAgentConfig> AcpAgent<C> {
    pub fn new(config: C) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(AcpAgentState::default())),
        }
    }

    fn send_command<T>(
        &self,
        build: impl FnOnce(SyncSender<Result<T>>) -> AcpCommand,
    ) -> Result<T> {
        let tx = {
            let state = self.state.lock();
            state
                .control_tx
                .clone()
                .context("ACP agent is not running")?
        };
        let (reply_tx, reply_rx) = mpsc::sync_channel(1);
        tx.send(build(reply_tx))
            .map_err(|_| anyhow!("ACP runtime command channel closed"))?;
        reply_rx
            .recv()
            .map_err(|_| anyhow!("ACP runtime did not reply"))?
    }
}

impl<C: AcpAgentConfig> Agent for AcpAgent<C> {
    fn start(&mut self, worktree_path: &std::path::Path, callbacks: AgentCallbacks) -> Result<()> {
        let worktree_path = worktree_path.to_path_buf();
        let state = Arc::clone(&self.state);
        let (startup_tx, startup_rx) = mpsc::sync_channel(1);
        let (control_tx, control_rx) = tokio_mpsc::unbounded_channel();

        {
            let mut state = state.lock();
            state.control_tx = Some(control_tx.clone());
            state.exit_code = None;
            state.session_id = None;
            state.pending_permission_requests.clear();
            state.next_message_id = 0;
        }

        let config_id = self.config.id().to_string();
        let command = self.config.build_command(&worktree_path);
        let client = AcpClientHandler::new(
            Arc::clone(&state),
            callbacks.clone(),
            worktree_path.clone(),
            self.config.title().to_string(),
        );
        let initialize_request = self.config.initialize_request();
        let new_session_request = self.config.new_session_request(&worktree_path);
        let config_title = self.config.title().to_string();

        std::thread::spawn(move || {
            let runtime = match Builder::new_current_thread().enable_all().build() {
                Ok(runtime) => runtime,
                Err(error) => {
                    let _ = startup_tx.send(Err(anyhow!(error)));
                    set_exit_code(&state, 1);
                    return;
                }
            };
            let local = LocalSet::new();
            local.block_on(&runtime, async move {
                if let Err(error) = run_acp_runtime(
                    command,
                    client,
                    initialize_request,
                    new_session_request,
                    state,
                    callbacks,
                    control_rx,
                    startup_tx,
                    config_title,
                )
                .await
                {
                    log::warn!("ACP runtime {} failed: {}", config_id, error);
                }
            });
        });

        startup_rx
            .recv()
            .map_err(|_| anyhow!("ACP runtime did not finish startup"))??;
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.state.lock().control_tx.is_some()
    }

    fn stop(&mut self) -> Result<()> {
        let tx = {
            let state = self.state.lock();
            state.control_tx.clone()
        }
        .context("ACP agent is not running")?;
        tx.send(AcpCommand::Shutdown)
            .map_err(|_| anyhow!("ACP runtime command channel closed"))
    }

    fn send_message(&mut self, content: String) -> Result<()> {
        let trimmed = content.trim().to_string();
        if trimmed.is_empty() {
            return Ok(());
        }
        self.send_command(|reply| AcpCommand::Prompt {
            content: trimmed,
            reply,
        })
    }

    fn interrupt(&mut self) -> Result<()> {
        self.send_command(|reply| AcpCommand::Cancel { reply })
    }

    fn respond_ui_request(&mut self, request_id: String, response: Value) -> Result<()> {
        self.send_command(|reply| AcpCommand::RespondUiRequest {
            request_id,
            response,
            reply,
        })
    }

    fn start_new_thread(&mut self) -> Result<()> {
        self.send_command(|reply| AcpCommand::NewSession { reply })
    }
}

impl<C: AcpAgentConfig> AcpAgentTrait for AcpAgent<C> {}
