use super::client::AcpClientHandler;
use super::command::AcpCommand;
use super::state::SharedAcpAgentState;
use super::utils::{
    finalize_active_messages, log_acp_rpc_stream, log_acp_stderr, update_config_options,
};
use crate::features::tasks::agents::AgentCallbacks;
use crate::features::tasks::TaskStatus;
use agent_client_protocol::{
    CancelNotification, ClientSideConnection, InitializeRequest, InitializeResponse,
    LoadSessionRequest, NewSessionRequest, SessionConfigOption, SessionConfigValueId,
    SessionId, SetSessionConfigOptionRequest,
};
use anyhow::{anyhow, Context, Result};
use std::process::{Command, Stdio};
use std::sync::mpsc::SyncSender;
use std::time::Duration;
use tokio::process::Command as TokioCommand;
use tokio::sync::mpsc as tokio_mpsc;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

pub(crate) async fn run_acp_runtime(
    mut command: Command,
    client: AcpClientHandler,
    initialize_request: InitializeRequest,
    new_session_request: NewSessionRequest,
    load_session_request: Option<LoadSessionRequest>,
    state: SharedAcpAgentState,
    callbacks: AgentCallbacks,
    mut control_rx: tokio_mpsc::UnboundedReceiver<AcpCommand>,
    startup_tx: SyncSender<Result<()>>,
    title: String,
) -> Result<()> {
    let command_description = describe_command(&command);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = TokioCommand::from(command).spawn().with_context(|| {
        format!(
            "failed to start ACP agent {} with command {}",
            title, command_description
        )
    })?;
    let stdin = child
        .stdin
        .take()
        .context("failed to open ACP stdin")?
        .compat_write();
    let stdout = child
        .stdout
        .take()
        .context("failed to open ACP stdout")?
        .compat();
    if let Some(stderr) = child.stderr.take() {
        tokio::task::spawn_local(log_acp_stderr(stderr, title.clone()));
    }

    let (conn, io_task) = ClientSideConnection::new(client, stdin, stdout, |future| {
        tokio::task::spawn_local(future);
    });
    tokio::task::spawn_local(log_acp_rpc_stream(conn.subscribe(), title.clone()));
    tokio::task::spawn_local(async move {
        if let Err(error) = io_task.await {
            log::warn!("ACP IO task failed: {}", error);
        }
    });

    let initialize = agent_client_protocol::Agent::initialize(&conn, initialize_request)
        .await
        .map_err(|error| anyhow!("ACP initialize failed: {}", error))?;
    handle_initialize(&callbacks, &initialize);
    if let Some(load_request) = load_session_request {
        let session_id = load_request.session_id.clone();
        match agent_client_protocol::Agent::load_session(&conn, load_request).await {
            Ok(response) => {
                apply_session_response(&state, session_id.clone(), response.config_options);
            }
            Err(error) if should_fallback_to_new_session(&error) => {
                log::warn!(
                    "ACP runtime {} could not resume session {}; falling back to new session: {}",
                    title,
                    session_id.0,
                    error
                );
                let session = agent_client_protocol::Agent::new_session(&conn, new_session_request.clone())
                    .await
                    .map_err(|new_error| anyhow!("ACP new_session failed after load fallback: {}", new_error))?;
                let session_id = session.session_id.clone();
                apply_session_response(&state, session_id.clone(), session.config_options);
            }
            Err(error) => {
                return Err(anyhow!("ACP load_session failed: {}", error));
            }
        }
    } else {
        let session = agent_client_protocol::Agent::new_session(&conn, new_session_request.clone())
            .await
            .map_err(|error| anyhow!("ACP new_session failed: {}", error))?;
        let session_id = session.session_id.clone();
        apply_session_response(&state, session_id.clone(), session.config_options);
    }
    (callbacks.on_status)(TaskStatus::Idle);
    (callbacks.on_gui_hydrated)();
    let _ = startup_tx.send(Ok(()));

    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| anyhow!("ACP process wait failed: {}", error))?
        {
            let code = status.code().unwrap_or(1);
            finalize_active_messages(&state, &callbacks);
            cancel_pending_permission_requests(&state, true);
            set_exit_code(&state, code);
            (callbacks.on_exit)(code);
            break;
        }

        if let Ok(Some(command)) =
            tokio::time::timeout(Duration::from_millis(100), control_rx.recv()).await
        {
            match command {
                AcpCommand::Prompt { content, reply } => {
                    let result = handle_prompt(&conn, &state, &callbacks, content).await;
                    let _ = reply.send(result);
                }
                AcpCommand::SetConfigOption {
                    config_id,
                    value,
                    reply,
                } => {
                    let result = handle_set_config_option(&conn, &state, config_id, value).await;
                    let _ = reply.send(result);
                }
                AcpCommand::Cancel { reply } => {
                    let result = handle_cancel(&conn, &state).await;
                    let _ = reply.send(result);
                }
                AcpCommand::RespondUiRequest {
                    request_id,
                    response,
                    reply,
                } => {
                    let result = handle_ui_response(&state, &callbacks, request_id, response);
                    let _ = reply.send(result);
                }
                AcpCommand::NewSession { reply } => {
                    finalize_active_messages(&state, &callbacks);
                    let result =
                        handle_new_session(&conn, &state, &callbacks, &new_session_request).await;
                    let _ = reply.send(result);
                }
                AcpCommand::Shutdown => {
                    finalize_active_messages(&state, &callbacks);
                    cancel_pending_permission_requests(&state, true);
                    let _ = child.kill().await;
                }
            }
        }
    }

    Ok(())
}

pub(crate) fn handle_initialize(callbacks: &AgentCallbacks, response: &InitializeResponse) {
    let summary = format!(
        "[ACP] initialized protocol v{} with {} prompt capabilities\n",
        response.protocol_version,
        summarize_prompt_capabilities(&response.agent_capabilities.prompt_capabilities)
    );
    (callbacks.on_output)(summary);
}

pub(crate) fn cancel_pending_permission_requests(state: &SharedAcpAgentState, cancelled: bool) {
    let pending = {
        let mut state = state.lock();
        std::mem::take(&mut state.pending_permission_requests)
    };
    for request in pending.into_values() {
        let outcome = if cancelled {
            agent_client_protocol::RequestPermissionResponse::new(
                agent_client_protocol::RequestPermissionOutcome::Cancelled,
            )
        } else {
            continue;
        };
        let _ = request.reply.send(outcome);
    }
}

pub(crate) fn set_exit_code(state: &SharedAcpAgentState, exit_code: i32) {
    let mut state = state.lock();
    state.exit_code = Some(exit_code);
    state.control_tx = None;
}

pub(crate) fn describe_command(command: &Command) -> String {
    let program = command.get_program().to_string_lossy();
    let args = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let cwd = command
        .get_current_dir()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<inherit>".to_string());

    if args.is_empty() {
        format!("program={program}, cwd={cwd}")
    } else {
        format!("program={program}, args=[{}], cwd={cwd}", args.join(", "))
    }
}

fn summarize_prompt_capabilities(
    capabilities: &agent_client_protocol::PromptCapabilities,
) -> &'static str {
    if capabilities.image || capabilities.audio || capabilities.embedded_context {
        "extended"
    } else {
        "baseline"
    }
}

fn should_fallback_to_new_session(error: &agent_client_protocol::Error) -> bool {
    matches!(
        error.code,
        agent_client_protocol::ErrorCode::MethodNotFound
            | agent_client_protocol::ErrorCode::ResourceNotFound
    )
        || error.message.contains("Method not found")
        || error.message.contains("Resource not found")
}

async fn handle_prompt(
    conn: &ClientSideConnection,
    state: &SharedAcpAgentState,
    callbacks: &AgentCallbacks,
    content: String,
) -> Result<()> {
    let session_id = {
        let state = state.lock();
        state
            .session_id
            .clone()
            .context("ACP session is not initialized")?
    };
    (callbacks.on_status)(TaskStatus::Working);
    let response = agent_client_protocol::Agent::prompt(
        conn,
        agent_client_protocol::PromptRequest::new(
            session_id,
            vec![agent_client_protocol::ContentBlock::from(content)],
        ),
    )
    .await
    .map_err(|error| anyhow!("ACP prompt failed: {}", error))?;
    let _ = response;
    finalize_active_messages(state, callbacks);
    (callbacks.on_status)(TaskStatus::Idle);
    Ok(())
}

async fn handle_cancel(conn: &ClientSideConnection, state: &SharedAcpAgentState) -> Result<()> {
    let session_id = {
        let state = state.lock();
        state
            .session_id
            .clone()
            .context("ACP session is not initialized")?
    };
    cancel_pending_permission_requests(state, true);
    agent_client_protocol::Agent::cancel(conn, CancelNotification::new(session_id))
        .await
        .map_err(|error| anyhow!("ACP cancel failed: {}", error))
}

async fn handle_set_config_option(
    conn: &ClientSideConnection,
    state: &SharedAcpAgentState,
    config_id: String,
    value: String,
) -> Result<()> {
    let session_id = {
        let state = state.lock();
        state
            .session_id
            .clone()
            .context("ACP session is not initialized")?
    };
    let response = agent_client_protocol::Agent::set_session_config_option(
        conn,
        SetSessionConfigOptionRequest::new(session_id, config_id, SessionConfigValueId::new(value)),
    )
    .await
    .map_err(|error| anyhow!("ACP set_session_config_option failed: {}", error))?;
    let mut state = state.lock();
    state.config_options = response.config_options;
    Ok(())
}

async fn handle_new_session(
    conn: &ClientSideConnection,
    state: &SharedAcpAgentState,
    callbacks: &AgentCallbacks,
    request: &NewSessionRequest,
) -> Result<()> {
    cancel_pending_permission_requests(state, true);
    let response = agent_client_protocol::Agent::new_session(conn, request.clone())
        .await
        .map_err(|error| anyhow!("ACP new_session failed: {}", error))?;
    apply_session_response(state, response.session_id, response.config_options);
    (callbacks.on_status)(TaskStatus::Idle);
    (callbacks.on_gui_hydrated)();
    Ok(())
}

fn apply_session_response(
    state: &SharedAcpAgentState,
    session_id: SessionId,
    config_options: Option<Vec<SessionConfigOption>>,
) {
    let config_options = config_options.unwrap_or_default();
    if !config_options.is_empty() {
        update_config_options(state, config_options.clone());
    }
    {
        let mut state = state.lock();
        state.session_id = Some(session_id);
        state.active_message_ids.clear();
        state.session_message_ids.clear();
        state.tool_call_messages.clear();
        state.config_options = config_options;
    }
}

fn handle_ui_response(
    state: &SharedAcpAgentState,
    callbacks: &AgentCallbacks,
    request_id: String,
    response: serde_json::Value,
) -> Result<()> {
    let (reply, parsed_response) = {
        let state = state.lock();
        let pending = state
            .pending_permission_requests
            .get(&request_id)
            .context("ACP request is no longer pending")?;
        let parsed_response = parse_permission_response(&pending.options, response)?;
        (pending.reply.clone(), parsed_response)
    };
    {
        let mut state = state.lock();
        state.pending_permission_requests.remove(&request_id);
    }
    reply
        .send(parsed_response)
        .map_err(|_| anyhow!("failed delivering ACP request response"))?;
    (callbacks.on_gui_request)(crate::features::tasks::agents::GuiRequestEvent::Cleared);
    Ok(())
}

fn parse_permission_response(
    options: &[agent_client_protocol::PermissionOption],
    response: serde_json::Value,
) -> Result<agent_client_protocol::RequestPermissionResponse> {
    let selected = response
        .as_str()
        .map(ToOwned::to_owned)
        .or_else(|| {
            response
                .get("optionId")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
        })
        .or_else(|| {
            response
                .get("option_id")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
        })
        .or_else(|| {
            response
                .get("answer")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
        })
        .or_else(|| {
            response
                .get("answers")
                .and_then(|value| value.get("permission"))
                .and_then(|value| value.get("answers"))
                .and_then(serde_json::Value::as_array)
                .and_then(|answers| answers.first())
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
        });

    let Some(selected) = selected else {
        return Ok(agent_client_protocol::RequestPermissionResponse::new(
            agent_client_protocol::RequestPermissionOutcome::Cancelled,
        ));
    };

    let option = options
        .iter()
        .find(|option| option.option_id.0.as_ref() == selected || option.name == selected)
        .context("invalid ACP permission option")?;

    Ok(agent_client_protocol::RequestPermissionResponse::new(
        agent_client_protocol::RequestPermissionOutcome::Selected(
            agent_client_protocol::SelectedPermissionOutcome::new(option.option_id.clone()),
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::{handle_ui_response, parse_permission_response};
    use crate::features::tasks::agents::{AgentCallbacks, GuiRequestEvent};
    use crate::features::tasks::TaskStatus;
    use agent_client_protocol::{PermissionOption, PermissionOptionKind, RequestPermissionOutcome};
    use parking_lot::Mutex;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{mpsc, Arc};

    use super::super::state::{AcpAgentState, PendingPermissionRequest, SharedAcpAgentState};

    fn callbacks(cleared_count: Arc<AtomicUsize>) -> AgentCallbacks {
        AgentCallbacks {
            on_output: Arc::new(|_| {}),
            on_status: Arc::new(|_: TaskStatus| {}),
            on_exit: Arc::new(|_| {}),
            on_gui_event: Arc::new(|_| {}),
            on_gui_history: Arc::new(|_| {}),
            on_gui_activity: Arc::new(|_| {}),
            on_gui_plan: Arc::new(|_| {}),
            on_gui_token_usage: Arc::new(|_| {}),
            on_gui_request: Arc::new(move |event| {
                if matches!(event, GuiRequestEvent::Cleared) {
                    cleared_count.fetch_add(1, Ordering::Relaxed);
                }
            }),
            on_gui_hydrated: Arc::new(|| {}),
        }
    }

    #[test]
    fn parses_nested_user_input_answers() {
        let options = vec![PermissionOption::new(
            "allow-once",
            "Allow once",
            PermissionOptionKind::AllowOnce,
        )];
        let response = serde_json::json!({
            "answers": {
                "permission": {
                    "answers": ["allow-once"]
                }
            }
        });

        let parsed = parse_permission_response(&options, response).unwrap();

        match parsed.outcome {
            RequestPermissionOutcome::Selected(selected) => {
                assert_eq!(selected.option_id.0.as_ref(), "allow-once");
            }
            RequestPermissionOutcome::Cancelled => panic!("expected selected outcome"),
            _ => panic!("unexpected permission outcome"),
        }
    }

    #[test]
    fn invalid_ui_response_keeps_pending_request() {
        let (reply_tx, _reply_rx) = mpsc::sync_channel(1);
        let state: SharedAcpAgentState = Arc::new(Mutex::new(AcpAgentState {
            pending_permission_requests: HashMap::from([(
                "req-1".to_string(),
                PendingPermissionRequest {
                    options: vec![PermissionOption::new(
                        "allow-once",
                        "Allow once",
                        PermissionOptionKind::AllowOnce,
                    )],
                    reply: reply_tx,
                },
            )]),
            ..AcpAgentState::default()
        }));
        let callbacks = callbacks(Arc::new(AtomicUsize::new(0)));

        let result = handle_ui_response(
            &state,
            &callbacks,
            "req-1".to_string(),
            serde_json::json!({ "optionId": "unknown" }),
        );

        assert!(result.is_err());
        assert!(state
            .lock()
            .pending_permission_requests
            .contains_key("req-1"));
    }
}
