use super::client::AcpClientHandler;
use super::command::AcpCommand;
use super::state::SharedAcpAgentState;
use super::utils::{finalize_active_messages, log_acp_stderr};
use crate::features::tasks::agents::AgentCallbacks;
use crate::features::tasks::TaskStatus;
use agent_client_protocol::{
    CancelNotification, ClientSideConnection, InitializeRequest, InitializeResponse,
    NewSessionRequest,
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
    state: SharedAcpAgentState,
    callbacks: AgentCallbacks,
    mut control_rx: tokio_mpsc::UnboundedReceiver<AcpCommand>,
    startup_tx: SyncSender<Result<()>>,
    title: String,
) -> Result<()> {
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = TokioCommand::from(command)
        .spawn()
        .with_context(|| format!("failed to start ACP agent {}", title))?;
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
    tokio::task::spawn_local(async move {
        if let Err(error) = io_task.await {
            log::warn!("ACP IO task failed: {}", error);
        }
    });

    let initialize = agent_client_protocol::Agent::initialize(&conn, initialize_request)
        .await
        .map_err(|error| anyhow!("ACP initialize failed: {}", error))?;
    handle_initialize(&callbacks, &initialize);
    let session = agent_client_protocol::Agent::new_session(&conn, new_session_request.clone())
        .await
        .map_err(|error| anyhow!("ACP new_session failed: {}", error))?;
    {
        let mut agent_state = state.lock();
        agent_state.session_id = Some(session.session_id.clone());
        agent_state.active_message_ids.clear();
    }
    (callbacks.on_status)(TaskStatus::Idle);
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

fn summarize_prompt_capabilities(
    capabilities: &agent_client_protocol::PromptCapabilities,
) -> &'static str {
    if capabilities.image || capabilities.audio || capabilities.embedded_context {
        "extended"
    } else {
        "baseline"
    }
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
    {
        let mut state = state.lock();
        state.session_id = Some(response.session_id);
    }
    (callbacks.on_status)(TaskStatus::Idle);
    Ok(())
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
