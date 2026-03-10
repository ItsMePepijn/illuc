use super::state::SharedAcpAgentState;
use crate::features::tasks::agents::{AgentCallbacks, GuiMessageEvent, GuiMessageRole};
use agent_client_protocol::{ContentBlock, TerminalExitStatus, ToolCallStatus};
use anyhow::anyhow;
use std::io::Read;
use std::process::Child;
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn emit_chunk(
    state: &SharedAcpAgentState,
    callbacks: &AgentCallbacks,
    role: GuiMessageRole,
    content: ContentBlock,
) {
    let content = render_content_block(content);
    let message_id = {
        let mut state = state.lock();
        let key = role.as_str().to_string();
        if let Some(existing) = state.active_message_ids.get(&key) {
            existing.clone()
        } else {
            state.next_message_id += 1;
            let message_id = format!("acp-message-{}", state.next_message_id);
            state.active_message_ids.insert(key, message_id.clone());
            message_id
        }
    };
    (callbacks.on_gui_event)(GuiMessageEvent {
        message_id,
        role,
        content,
        is_delta: true,
        is_final: false,
    });
}

pub(crate) fn finalize_active_messages(state: &SharedAcpAgentState, callbacks: &AgentCallbacks) {
    let active_messages = {
        let mut state = state.lock();
        std::mem::take(&mut state.active_message_ids)
    };

    for (role, message_id) in active_messages {
        let role = match role.as_str() {
            "user" => GuiMessageRole::User,
            "assistant" => GuiMessageRole::Assistant,
            "system" => GuiMessageRole::System,
            "reasoning" => GuiMessageRole::Reasoning,
            _ => GuiMessageRole::Assistant,
        };
        (callbacks.on_gui_event)(GuiMessageEvent {
            message_id,
            role,
            content: String::new(),
            is_delta: true,
            is_final: true,
        });
    }
}

pub(crate) fn render_content_block(content: ContentBlock) -> String {
    match content {
        ContentBlock::Text(text) => text.text,
        other => serde_json::to_string(&other).unwrap_or_default(),
    }
}

pub(crate) fn format_tool_status(status: ToolCallStatus) -> &'static str {
    match status {
        ToolCallStatus::Pending => "pending",
        ToolCallStatus::InProgress => "running",
        ToolCallStatus::Completed => "completed",
        ToolCallStatus::Failed => "failed",
        _ => "pending",
    }
}

pub(crate) fn select_lines(content: String, start_line: Option<u32>, limit: Option<u32>) -> String {
    let start = start_line.unwrap_or(1).saturating_sub(1) as usize;
    let limit = limit.unwrap_or(u32::MAX) as usize;
    content
        .lines()
        .skip(start)
        .take(limit)
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) async fn log_acp_stderr(stderr: tokio::process::ChildStderr, title: String) {
    use tokio::io::{AsyncBufReadExt, BufReader};

    let mut lines = BufReader::new(stderr).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if !line.trim().is_empty() {
            log::warn!("{} stderr: {}", title, line);
        }
    }
}

pub(crate) fn io_to_acp_error(error: std::io::Error) -> agent_client_protocol::Error {
    anyhow_to_acp_error(anyhow!(error))
}

pub(crate) fn anyhow_to_acp_error(error: anyhow::Error) -> agent_client_protocol::Error {
    agent_client_protocol::Error::internal_error().data(error.to_string())
}

pub(crate) fn spawn_terminal_reader(
    mut reader: impl Read + Send + 'static,
    output: Arc<parking_lot::Mutex<String>>,
    output_byte_limit: Option<usize>,
) {
    std::thread::spawn(move || {
        let mut buffer = [0u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(size) => append_terminal_output(
                    &output,
                    &String::from_utf8_lossy(&buffer[..size]),
                    output_byte_limit,
                ),
                Err(error) => {
                    log::warn!("ACP terminal read failed: {}", error);
                    break;
                }
            }
        }
    });
}

pub(crate) fn spawn_terminal_waiter(
    child: Arc<parking_lot::Mutex<Child>>,
    exit_status: Arc<parking_lot::Mutex<Option<TerminalExitStatus>>>,
) {
    std::thread::spawn(move || {
        let status = child.lock().wait();
        let terminal_status = match status {
            Ok(status) => {
                TerminalExitStatus::new().exit_code(status.code().map(|code| code as u32))
            }
            Err(error) => {
                log::warn!("ACP terminal wait failed: {}", error);
                TerminalExitStatus::new().exit_code(1)
            }
        };
        *exit_status.lock() = Some(terminal_status);
    });
}

pub(crate) fn append_terminal_output(
    output: &Arc<parking_lot::Mutex<String>>,
    chunk: &str,
    limit: Option<usize>,
) {
    let mut output = output.lock();
    output.push_str(chunk);
    if let Some(limit) = limit {
        while output.len() > limit {
            let drain = output
                .char_indices()
                .nth(output.len().saturating_sub(limit))
                .map(|(index, _)| index)
                .unwrap_or(output.len().saturating_sub(limit));
            output.drain(..drain.max(1));
        }
    }
}

pub(crate) async fn poll_runtime_cycle() {
    tokio::time::sleep(Duration::from_millis(100)).await;
}
