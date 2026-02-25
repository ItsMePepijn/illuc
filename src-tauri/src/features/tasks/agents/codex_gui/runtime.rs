use super::{
    app_server_handlers, rpc, CodexGuiAgent, CodexGuiAgentState, NullMaster, NullWriter, StdChild,
};
use crate::features::tasks::agents::{AgentCallbacks, AgentRuntime};
use crate::utils::pty::{ChildHandle, MasterHandle, WriteHandle};
use anyhow::{Context, Result};
use parking_lot::Mutex;
use serde_json::Value;
use std::io::{BufRead, BufReader, BufWriter};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

pub(super) fn start(
    agent: &mut CodexGuiAgent,
    worktree_path: &Path,
    callbacks: AgentCallbacks,
) -> Result<AgentRuntime> {
    let mut command = Command::new("codex");
    command
        .arg("app-server")
        .current_dir(worktree_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .context("failed to start codex app-server")?;
    let stdin = child
        .stdin
        .take()
        .context("failed to open codex app-server stdin")?;
    let stdout = child
        .stdout
        .take()
        .context("failed to open codex app-server stdout")?;
    let stderr = child.stderr.take();
    app_server_handlers::spawn_stderr_logger(stderr);

    {
        let mut state = agent.state.lock();
        state.stdin = Some(BufWriter::new(stdin));
        state.cwd = Some(worktree_path.to_string_lossy().to_string());
        state.thread_id = None;
        state.current_turn_id = None;
        state.pending_messages.clear();
        state.thread_list_request_id = None;
        state.thread_resume_request_id = None;
        state.model_list_request_id = None;
        state.rate_limits_request_id = None;
        state.callbacks = Some(callbacks.clone());
        state.next_id = 1;
        state.model = None;
        state.reasoning_effort = None;
        state.available_models = Vec::new();
        state.available_model_capabilities = Vec::new();
        state.pending_server_requests.clear();
        state.rollback_request_id = None;
        state.rollback_history_events.clear();
        state.activity_label = None;
        state.activity_started_at = None;

        let initialize_id = rpc::next_id(&mut state);
        let initialize_payload = serde_json::json!({
            "id": initialize_id,
            "method": "initialize",
            "params": {
                "capabilities": {
                    "experimentalApi": true
                },
                "clientInfo": {
                    "name": "illuc-codex-gui",
                    "title": "Illuc Codex GUI",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        });
        rpc::send_rpc(&mut state, initialize_payload)?;
        rpc::send_rpc(
            &mut state,
            serde_json::json!({
                "method": "initialized",
                "params": {}
            }),
        )?;

        rpc::send_model_list_request(&mut state)?;
        if let Err(error) = rpc::send_rate_limits_request(&mut state) {
            log::warn!("failed to request Codex rate limits: {}", error);
        }
        rpc::send_thread_list_request(&mut state)?;
    }

    spawn_reader_loop(Arc::clone(&agent.state), stdout);

    let child_handle: Arc<Mutex<ChildHandle>> = Arc::new(Mutex::new(Box::new(StdChild {
        child: Mutex::new(child),
    })));
    let writer_handle: WriteHandle = Arc::new(Mutex::new(Box::new(NullWriter)));
    let master_handle: MasterHandle = Arc::new(Mutex::new(Box::new(NullMaster)));

    spawn_exit_watcher(child_handle.clone(), callbacks);

    Ok(AgentRuntime {
        child: child_handle,
        writer: writer_handle,
        master: master_handle,
    })
}

fn spawn_reader_loop(
    state: Arc<Mutex<CodexGuiAgentState>>,
    stdout: impl std::io::Read + Send + 'static,
) {
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(std::result::Result::ok) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let message: Value = match serde_json::from_str(trimmed) {
                Ok(value) => value,
                Err(error) => {
                    log::warn!("invalid codex app-server payload: {}", error);
                    continue;
                }
            };

            if let (Some(request_id), Some(method)) = (
                message.get("id"),
                message.get("method").and_then(|value| value.as_str()),
            ) {
                if let Err(error) =
                    app_server_handlers::handle_server_request(&state, &message, request_id, method)
                {
                    log::warn!(
                        "failed to respond to app-server request {}: {}",
                        method,
                        error
                    );
                }
                continue;
            }

            if message.get("id").is_some() {
                app_server_handlers::handle_response(&state, &message);
                continue;
            }

            if message.get("method").is_some() {
                app_server_handlers::handle_notification(&state, &message);
            }
        }
    });
}

fn spawn_exit_watcher(child: Arc<Mutex<ChildHandle>>, callbacks: AgentCallbacks) {
    std::thread::spawn(move || {
        let exit_code = loop {
            {
                let mut child_guard = child.lock();
                match child_guard.try_wait() {
                    Ok(Some(status)) => {
                        break if status.success() {
                            0
                        } else {
                            status.exit_code()
                        };
                    }
                    Ok(None) => {}
                    Err(error) => {
                        log::warn!("failed waiting for codex app-server: {}", error);
                        break 1;
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(200));
        };
        (callbacks.on_exit)(exit_code);
    });
}
