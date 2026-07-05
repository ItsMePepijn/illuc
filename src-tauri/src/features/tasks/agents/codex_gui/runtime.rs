use super::{event_handlers, rpc, CodexGuiAgent, CodexGuiAgentState, StdChild};
#[cfg(not(target_os = "windows"))]
use crate::features::tasks::agent_command::{apply_process_env, resolve_command};
use crate::features::tasks::agents::AgentCallbacks;
use crate::utils::pty::ChildHandle;
#[cfg(target_os = "windows")]
use crate::utils::windows::{build_wsl_process_command, to_wsl_path};
use anyhow::{Context, Result};
use parking_lot::Mutex;
use serde_json::Value;
use std::io::{BufRead, BufReader, BufWriter};
use std::path::Path;
#[cfg(not(target_os = "windows"))]
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

pub(super) fn start(
    agent: &mut CodexGuiAgent,
    worktree_path: &Path,
    callbacks: AgentCallbacks,
) -> Result<()> {
    #[cfg(target_os = "windows")]
    let mut command = build_wsl_process_command(worktree_path, "codex", &["app-server"]);
    #[cfg(not(target_os = "windows"))]
    let mut command = {
        let mut command = Command::new(resolve_command("codex"));
        apply_process_env(&mut command);
        command.arg("app-server").current_dir(worktree_path);
        command
    };
    command
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
    event_handlers::spawn_stderr_logger(stderr);

    {
        let mut state = agent.state.lock();
        state.stdin = Some(BufWriter::new(stdin));
        #[cfg(target_os = "windows")]
        {
            state.cwd = Some(resolve_windows_cwd(worktree_path));
        }
        #[cfg(not(target_os = "windows"))]
        {
            state.cwd = Some(worktree_path.to_string_lossy().to_string());
        }
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
        state.service_tier = None;
        state.available_models = Vec::new();
        state.available_model_capabilities = Vec::new();
        state.pending_server_requests.clear();
        state.rollback_request_id = None;
        state.rollback_history_events.clear();
        state.activity_label = None;
        state.activity_started_at = None;
        state.child = None;

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
    {
        let mut state = agent.state.lock();
        state.child = Some(child_handle.clone());
    }

    spawn_exit_watcher(Arc::clone(&agent.state), child_handle, callbacks);

    Ok(())
}

#[cfg(target_os = "windows")]
fn resolve_windows_cwd(worktree_path: &Path) -> String {
    std::fs::canonicalize(worktree_path)
        .ok()
        .as_deref()
        .and_then(to_wsl_path)
        .or_else(|| to_wsl_path(worktree_path))
        .unwrap_or_else(|| worktree_path.to_string_lossy().to_string())
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
                    event_handlers::handle_server_request(&state, &message, request_id, method)
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
                event_handlers::handle_response(&state, &message);
                continue;
            }

            if message.get("method").is_some() {
                event_handlers::handle_notification(&state, &message);
            }
        }
    });
}

fn spawn_exit_watcher(
    state: Arc<Mutex<CodexGuiAgentState>>,
    child: Arc<Mutex<ChildHandle>>,
    callbacks: AgentCallbacks,
) {
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
        let mut state = state.lock();
        state.stdin = None;
        state.child = None;
        (callbacks.on_exit)(exit_code);
    });
}
