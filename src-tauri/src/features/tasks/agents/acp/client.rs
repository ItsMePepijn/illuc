use super::state::{PendingPermissionRequest, SharedAcpAgentState};
use super::terminal::TerminalStore;
use super::utils::{
    anyhow_to_acp_error, apply_tool_call_update, available_commands_summary, emit_chunk,
    emit_session_note, io_to_acp_error, select_lines, update_config_options,
    upsert_tool_call_message,
};
use crate::features::tasks::agents::agent_gui::types::{
    GuiMessageRole, GuiPlanEvent, GuiPlanStep, GuiRequestEvent, GuiRequestQuestion,
    GuiRequestQuestionOption,
};
use crate::features::tasks::agents::AgentCallbacks;
use crate::features::tasks::TaskStatus;
use agent_client_protocol::{
    Client as AcpClient, RequestPermissionOutcome, RequestPermissionResponse, SessionNotification,
    SessionUpdate,
};
use anyhow::{Context, Result as AnyhowResult};
use async_trait::async_trait;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct AcpClientHandler {
    state: SharedAcpAgentState,
    callbacks: AgentCallbacks,
    terminals: Arc<TerminalStore>,
    worktree_path: PathBuf,
    title: String,
}

impl AcpClientHandler {
    pub(crate) fn new(
        state: SharedAcpAgentState,
        callbacks: AgentCallbacks,
        worktree_path: PathBuf,
        title: String,
    ) -> Self {
        Self {
            state,
            callbacks,
            terminals: Arc::new(TerminalStore::default()),
            worktree_path,
            title,
        }
    }
}

#[async_trait(?Send)]
impl AcpClient for AcpClientHandler {
    async fn request_permission(
        &self,
        args: agent_client_protocol::RequestPermissionRequest,
    ) -> agent_client_protocol::Result<RequestPermissionResponse> {
        let request_id = format!("acp-{}", Uuid::new_v4());
        let (reply_tx, reply_rx) = mpsc::sync_channel(1);
        {
            let mut state = self.state.lock();
            state.pending_permission_requests.insert(
                request_id.clone(),
                PendingPermissionRequest {
                    options: args.options.clone(),
                    reply: reply_tx,
                },
            );
        }

        let summary = args
            .tool_call
            .fields
            .title
            .clone()
            .unwrap_or_else(|| "Allow this ACP tool call?".to_string());
        log::info!(
            "ACP {} requested permission for tool call {}: {}",
            self.title,
            args.tool_call.tool_call_id.0,
            summary
        );
        (self.callbacks.on_status)(TaskStatus::AwaitingApproval);
        (self.callbacks.on_gui_request)(GuiRequestEvent::UserInput {
            request_id: request_id.clone(),
            item_id: args.tool_call.tool_call_id.0.to_string(),
            questions: vec![GuiRequestQuestion {
                id: "permission".to_string(),
                header: "Approve".to_string(),
                question: summary,
                is_other: false,
                is_secret: false,
                options: args
                    .options
                    .iter()
                    .map(|option| GuiRequestQuestionOption {
                        label: option.option_id.0.to_string(),
                        description: option.name.clone(),
                    })
                    .collect(),
            }],
        });

        let response = tokio::task::spawn_blocking(move || reply_rx.recv())
            .await
            .map_err(|error| {
                agent_client_protocol::Error::internal_error().data(error.to_string())
            })?
            .unwrap_or_else(|_| {
                RequestPermissionResponse::new(RequestPermissionOutcome::Cancelled)
            });
        log::info!(
            "ACP {} permission request {} resolved",
            self.title,
            request_id
        );
        (self.callbacks.on_status)(TaskStatus::Working);
        Ok(response)
    }

    async fn session_notification(
        &self,
        args: SessionNotification,
    ) -> agent_client_protocol::Result<()> {
        log::debug!(
            "ACP {} session update: {}",
            self.title,
            summarize_session_update(&args.update)
        );
        match args.update {
            SessionUpdate::UserMessageChunk(chunk) => {
                emit_chunk(
                    &self.state,
                    &self.callbacks,
                    GuiMessageRole::User,
                    chunk.content,
                );
            }
            SessionUpdate::AgentMessageChunk(chunk) => {
                emit_chunk(
                    &self.state,
                    &self.callbacks,
                    GuiMessageRole::Assistant,
                    chunk.content,
                );
            }
            SessionUpdate::AgentThoughtChunk(chunk) => {
                emit_chunk(
                    &self.state,
                    &self.callbacks,
                    GuiMessageRole::Reasoning,
                    chunk.content,
                );
            }
            SessionUpdate::Plan(plan) => {
                (self.callbacks.on_gui_plan)(GuiPlanEvent {
                    explanation: Some(format!("{} updated its plan", self.title)),
                    plan: plan
                        .entries
                        .into_iter()
                        .map(|entry| GuiPlanStep {
                            step: entry.content,
                            status: match entry.status {
                                agent_client_protocol::PlanEntryStatus::Pending => "pending",
                                agent_client_protocol::PlanEntryStatus::InProgress => "in_progress",
                                agent_client_protocol::PlanEntryStatus::Completed => "completed",
                                _ => "pending",
                            }
                            .to_string(),
                        })
                        .collect(),
                });
            }
            SessionUpdate::ToolCall(tool_call) => {
                upsert_tool_call_message(&self.state, &self.callbacks, tool_call);
            }
            SessionUpdate::ToolCallUpdate(update) => {
                apply_tool_call_update(&self.state, &self.callbacks, update);
            }
            SessionUpdate::AvailableCommandsUpdate(update) => {
                emit_session_note(
                    &self.state,
                    &self.callbacks,
                    "available-commands",
                    "ACP Commands",
                    available_commands_summary(&update.available_commands),
                );
            }
            SessionUpdate::CurrentModeUpdate(update) => {
                emit_session_note(
                    &self.state,
                    &self.callbacks,
                    "current-mode",
                    "ACP Mode",
                    format!("Current mode: `{}`", update.current_mode_id.0),
                );
            }
            SessionUpdate::ConfigOptionUpdate(update) => {
                update_config_options(&self.state, update.config_options.clone());
            }
            _ => {}
        }
        Ok(())
    }

    async fn write_text_file(
        &self,
        args: agent_client_protocol::WriteTextFileRequest,
    ) -> agent_client_protocol::Result<agent_client_protocol::WriteTextFileResponse> {
        let validated_path = validate_worktree_path(&self.worktree_path, &args.path, true)
            .map_err(anyhow_to_acp_error)?;
        if let Some(parent) = validated_path.parent() {
            fs::create_dir_all(parent).map_err(io_to_acp_error)?;
        }
        fs::write(validated_path, args.content).map_err(io_to_acp_error)?;
        Ok(agent_client_protocol::WriteTextFileResponse::new())
    }

    async fn read_text_file(
        &self,
        args: agent_client_protocol::ReadTextFileRequest,
    ) -> agent_client_protocol::Result<agent_client_protocol::ReadTextFileResponse> {
        let validated_path = validate_worktree_path(&self.worktree_path, &args.path, false)
            .map_err(anyhow_to_acp_error)?;
        let content = fs::read_to_string(validated_path).map_err(io_to_acp_error)?;
        Ok(agent_client_protocol::ReadTextFileResponse::new(
            select_lines(content, args.line, args.limit),
        ))
    }

    async fn create_terminal(
        &self,
        mut args: agent_client_protocol::CreateTerminalRequest,
    ) -> agent_client_protocol::Result<agent_client_protocol::CreateTerminalResponse> {
        args.cwd = Some(
            args.cwd
                .as_ref()
                .map(|cwd| validate_worktree_path(&self.worktree_path, cwd, false))
                .transpose()
                .map_err(anyhow_to_acp_error)?
                .unwrap_or_else(|| self.worktree_path.clone()),
        );
        let terminal_id = self
            .terminals
            .create(&self.worktree_path, args)
            .map_err(anyhow_to_acp_error)?;
        Ok(agent_client_protocol::CreateTerminalResponse::new(
            terminal_id,
        ))
    }

    async fn terminal_output(
        &self,
        args: agent_client_protocol::TerminalOutputRequest,
    ) -> agent_client_protocol::Result<agent_client_protocol::TerminalOutputResponse> {
        self.terminals
            .output(&args.terminal_id)
            .map_err(anyhow_to_acp_error)
    }

    async fn release_terminal(
        &self,
        args: agent_client_protocol::ReleaseTerminalRequest,
    ) -> agent_client_protocol::Result<agent_client_protocol::ReleaseTerminalResponse> {
        self.terminals
            .release(&args.terminal_id)
            .map_err(anyhow_to_acp_error)?;
        Ok(agent_client_protocol::ReleaseTerminalResponse::new())
    }

    async fn wait_for_terminal_exit(
        &self,
        args: agent_client_protocol::WaitForTerminalExitRequest,
    ) -> agent_client_protocol::Result<agent_client_protocol::WaitForTerminalExitResponse> {
        let exit_status = self
            .terminals
            .wait_for_exit(&args.terminal_id)
            .await
            .map_err(anyhow_to_acp_error)?;
        Ok(agent_client_protocol::WaitForTerminalExitResponse::new(
            exit_status,
        ))
    }

    async fn kill_terminal_command(
        &self,
        args: agent_client_protocol::KillTerminalCommandRequest,
    ) -> agent_client_protocol::Result<agent_client_protocol::KillTerminalCommandResponse> {
        self.terminals
            .kill(&args.terminal_id)
            .map_err(anyhow_to_acp_error)?;
        Ok(agent_client_protocol::KillTerminalCommandResponse::new())
    }
}

fn summarize_session_update(update: &SessionUpdate) -> String {
    match update {
        SessionUpdate::UserMessageChunk(_) => "user_message_chunk".to_string(),
        SessionUpdate::AgentMessageChunk(_) => "agent_message_chunk".to_string(),
        SessionUpdate::AgentThoughtChunk(_) => "agent_thought_chunk".to_string(),
        SessionUpdate::Plan(plan) => format!("plan({} entries)", plan.entries.len()),
        SessionUpdate::ToolCall(tool_call) => format!(
            "tool_call(id={}, status={:?})",
            tool_call.tool_call_id.0, tool_call.status
        ),
        SessionUpdate::ToolCallUpdate(update) => {
            format!("tool_call_update(id={})", update.tool_call_id.0)
        }
        SessionUpdate::AvailableCommandsUpdate(update) => format!(
            "available_commands_update({} commands)",
            update.available_commands.len()
        ),
        SessionUpdate::CurrentModeUpdate(update) => {
            format!("current_mode_update({})", update.current_mode_id.0)
        }
        SessionUpdate::ConfigOptionUpdate(update) => format!(
            "config_option_update({} options)",
            update.config_options.len()
        ),
        other => format!("{other:?}"),
    }
}

fn validate_worktree_path(
    worktree_path: &PathBuf,
    requested_path: &PathBuf,
    allow_create: bool,
) -> AnyhowResult<PathBuf> {
    let worktree_root = fs::canonicalize(worktree_path)
        .with_context(|| format!("failed to resolve worktree {}", worktree_path.display()))?;

    let candidate = if allow_create && !requested_path.exists() {
        let parent = requested_path
            .parent()
            .context("requested ACP path must have a parent directory")?;
        let resolved_parent = fs::canonicalize(parent)
            .with_context(|| format!("failed to resolve parent directory {}", parent.display()))?;
        resolved_parent.join(
            requested_path
                .file_name()
                .context("requested ACP path must include a file name")?,
        )
    } else {
        fs::canonicalize(requested_path)
            .with_context(|| format!("failed to resolve ACP path {}", requested_path.display()))?
    };

    if candidate.starts_with(&worktree_root) {
        Ok(candidate)
    } else {
        anyhow::bail!(
            "ACP path {} is outside the task worktree {}",
            requested_path.display(),
            worktree_root.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::validate_worktree_path;
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn rejects_paths_outside_worktree() {
        let base = std::env::temp_dir().join(format!("illuc-acp-{}", Uuid::new_v4()));
        let worktree = base.join("worktree");
        let outside = base.join("outside.txt");
        fs::create_dir_all(&worktree).unwrap();
        fs::write(&outside, "secret").unwrap();

        let result = validate_worktree_path(&worktree, &outside, false);
        assert!(result.is_err());

        let _ = fs::remove_file(&outside);
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn allows_new_files_inside_worktree() {
        let base = std::env::temp_dir().join(format!("illuc-acp-{}", Uuid::new_v4()));
        let worktree = base.join("worktree");
        let nested = worktree.join("nested");
        let target = nested.join("file.txt");
        fs::create_dir_all(&nested).unwrap();

        let result = validate_worktree_path(&worktree, &target, true).unwrap();
        assert!(result.ends_with("file.txt"));

        let _ = fs::remove_dir_all(&base);
    }
}
