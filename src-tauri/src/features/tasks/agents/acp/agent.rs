use super::client::AcpClientHandler;
use super::command::AcpCommand;
use super::config::AcpAgentConfig;
use super::runtime::{run_acp_runtime, set_exit_code};
use super::state::{AcpAgentState, SharedAcpAgentState};
use crate::features::tasks::agents::{
    AcpAgent as AcpAgentTrait, Agent, AgentCallbacks, AgentModelCapability, GuiAgent,
    GuiSessionAgent,
};
use agent_client_protocol::{
    SessionConfigKind, SessionConfigOption, SessionConfigOptionCategory, SessionConfigSelectOptions,
};
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

    fn apply_pending_session_config(&self) -> Result<()> {
        let updates = {
            let state = self.state.lock();
            collect_pending_session_config_updates(&state.config_options, &state)
        };

        for (config_id, value) in updates {
            self.send_command(|reply| AcpCommand::SetConfigOption {
                config_id,
                value,
                reply,
            })?;
        }

        Ok(())
    }
}

impl<C: AcpAgentConfig> Agent for AcpAgent<C> {
    fn as_gui_agent(&self) -> Option<&dyn GuiAgent> {
        Some(self)
    }

    fn as_gui_agent_mut(&mut self) -> Option<&mut dyn GuiAgent> {
        Some(self)
    }

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
            state.config_options.clear();
            state.pending_permission_requests.clear();
            state.next_message_id = 0;
            state.active_message_ids.clear();
            state.session_message_ids.clear();
            state.tool_call_messages.clear();
            state.selected_model = None;
            state.selected_reasoning_effort = None;
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
        let new_session_request = self.config.new_session_request(&worktree_path)?;
        let load_session_request = self.config.load_session_request(&worktree_path)?;
        let config_title = self.config.title().to_string();
        log::info!(
            "ACP agent {} starting for worktree {}",
            config_id,
            worktree_path.display()
        );

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
                    load_session_request,
                    state,
                    callbacks,
                    control_rx,
                    startup_tx,
                    config_title,
                )
                .await
                {
                    log::warn!("ACP runtime {} failed: {:#}", config_id, error);
                }
            });
        });

        startup_rx
            .recv()
            .map_err(|_| anyhow!("ACP runtime did not finish startup"))??;
        log::info!("ACP agent {} startup completed", self.config.id());
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
}

impl<C: AcpAgentConfig> GuiAgent for AcpAgent<C> {
    fn as_gui_session_agent(&self) -> Option<&dyn GuiSessionAgent> {
        Some(self)
    }

    fn as_gui_session_agent_mut(&mut self) -> Option<&mut dyn GuiSessionAgent> {
        Some(self)
    }

    fn send_message(&mut self, content: String) -> Result<()> {
        let trimmed = content.trim().to_string();
        if trimmed.is_empty() {
            return Ok(());
        }
        self.apply_pending_session_config()?;
        log::info!(
            "ACP agent {} sending prompt ({} chars)",
            self.config.id(),
            trimmed.chars().count()
        );
        self.send_command(|reply| AcpCommand::Prompt {
            content: trimmed,
            reply,
        })
    }

    fn interrupt(&mut self) -> Result<()> {
        log::info!("ACP agent {} sending cancel request", self.config.id());
        self.send_command(|reply| AcpCommand::Cancel { reply })
    }

    fn set_model(&mut self, model: Option<String>) -> Result<()> {
        let mut state = self.state.lock();
        state.selected_model = model
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        Ok(())
    }

    fn available_models(&self) -> Vec<String> {
        let state = self.state.lock();
        model_option(&state.config_options)
            .map(config_option_values)
            .unwrap_or_default()
    }

    fn available_model_capabilities(&self) -> Vec<AgentModelCapability> {
        let state = self.state.lock();
        let efforts = thought_level_option(&state.config_options)
            .map(config_option_values)
            .unwrap_or_default();

        model_option(&state.config_options)
            .map(config_option_values)
            .unwrap_or_default()
            .into_iter()
            .map(|model| AgentModelCapability {
                model,
                reasoning_efforts: efforts.clone(),
            })
            .collect()
    }

    fn selected_model(&self) -> Option<String> {
        let state = self.state.lock();
        state
            .selected_model
            .clone()
            .or_else(|| current_config_option_value(model_option(&state.config_options)))
    }

    fn selected_reasoning_effort(&self) -> Option<String> {
        let state = self.state.lock();
        state
            .selected_reasoning_effort
            .clone()
            .or_else(|| current_config_option_value(thought_level_option(&state.config_options)))
    }

    fn respond_ui_request(&mut self, request_id: String, response: Value) -> Result<()> {
        log::info!(
            "ACP agent {} responding to UI request {}",
            self.config.id(),
            request_id
        );
        self.send_command(|reply| AcpCommand::RespondUiRequest {
            request_id,
            response,
            reply,
        })
    }

    fn set_reasoning_effort(&mut self, effort: Option<String>) -> Result<()> {
        let mut state = self.state.lock();
        state.selected_reasoning_effort = effort
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        Ok(())
    }
}

impl<C: AcpAgentConfig> GuiSessionAgent for AcpAgent<C> {
    fn start_new_thread(&mut self) -> Result<()> {
        log::info!("ACP agent {} starting new session", self.config.id());
        self.send_command(|reply| AcpCommand::NewSession { reply })
    }
}

impl<C: AcpAgentConfig> AcpAgentTrait for AcpAgent<C> {}

fn collect_pending_session_config_updates(
    config_options: &[SessionConfigOption],
    state: &AcpAgentState,
) -> Vec<(String, String)> {
    let mut updates = Vec::new();

    if let Some(config_option) = model_option(config_options) {
        if let Some(selected_model) = state.selected_model.as_ref() {
            if let Some(value) = config_option_value_id(config_option, selected_model) {
                let current = current_config_option_value(Some(config_option));
                if current.as_ref() != Some(selected_model) {
                    updates.push((config_option.id.0.to_string(), value));
                }
            }
        }
    }

    if let Some(config_option) = thought_level_option(config_options) {
        if let Some(selected_effort) = state.selected_reasoning_effort.as_ref() {
            if let Some(value) = config_option_value_id(config_option, selected_effort) {
                let current = current_config_option_value(Some(config_option));
                if current.as_ref() != Some(selected_effort) {
                    updates.push((config_option.id.0.to_string(), value));
                }
            }
        }
    }

    updates
}

fn model_option(config_options: &[SessionConfigOption]) -> Option<&SessionConfigOption> {
    config_options.iter().find(|option| {
        option.category == Some(SessionConfigOptionCategory::Model)
            || matches_common_model_option(option)
    })
}

fn thought_level_option(config_options: &[SessionConfigOption]) -> Option<&SessionConfigOption> {
    config_options.iter().find(|option| {
        option.category == Some(SessionConfigOptionCategory::ThoughtLevel)
            || matches_common_thought_level_option(option)
    })
}

fn config_option_values(config_option: &SessionConfigOption) -> Vec<String> {
    match &config_option.kind {
        SessionConfigKind::Select(select) => match &select.options {
            SessionConfigSelectOptions::Ungrouped(options) => {
                options.iter().map(|option| option.name.clone()).collect()
            }
            SessionConfigSelectOptions::Grouped(groups) => groups
                .iter()
                .flat_map(|group| group.options.iter().map(|option| option.name.clone()))
                .collect(),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

fn current_config_option_value(config_option: Option<&SessionConfigOption>) -> Option<String> {
    let config_option = config_option?;
    match &config_option.kind {
        SessionConfigKind::Select(select) => match &select.options {
            SessionConfigSelectOptions::Ungrouped(options) => options
                .iter()
                .find(|option| option.value == select.current_value)
                .map(|option| option.name.clone()),
            SessionConfigSelectOptions::Grouped(groups) => groups
                .iter()
                .flat_map(|group| group.options.iter())
                .find(|option| option.value == select.current_value)
                .map(|option| option.name.clone()),
            _ => None,
        },
        _ => None,
    }
}

fn config_option_value_id(
    config_option: &SessionConfigOption,
    selected_name: &str,
) -> Option<String> {
    match &config_option.kind {
        SessionConfigKind::Select(select) => match &select.options {
            SessionConfigSelectOptions::Ungrouped(options) => options
                .iter()
                .find(|option| option.name == selected_name)
                .map(|option| option.value.0.to_string()),
            SessionConfigSelectOptions::Grouped(groups) => groups
                .iter()
                .flat_map(|group| group.options.iter())
                .find(|option| option.name == selected_name)
                .map(|option| option.value.0.to_string()),
            _ => None,
        },
        _ => None,
    }
}

fn matches_common_model_option(option: &SessionConfigOption) -> bool {
    let id = option.id.0.to_ascii_lowercase();
    let name = option.name.to_ascii_lowercase();
    id == "model" || name == "model" || name.contains("model")
}

fn matches_common_thought_level_option(option: &SessionConfigOption) -> bool {
    let id = option.id.0.to_ascii_lowercase();
    let name = option.name.to_ascii_lowercase();
    id.contains("reason")
        || id.contains("thought")
        || id.contains("effort")
        || name.contains("reason")
        || name.contains("thought")
        || name.contains("effort")
}
