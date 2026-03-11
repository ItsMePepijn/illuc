use super::state::{AcpAgentState, SharedAcpAgentState, TrackedToolCall};
use crate::features::tasks::agents::agent_gui::types::{
    GuiMessageEvent, GuiMessagePresentation, GuiMessagePresentationKind, GuiMessageRole,
    GuiMessageTextFormat, GuiToolRow, GuiToolRowKind,
};
use crate::features::tasks::agents::AgentCallbacks;
use agent_client_protocol::{
    AvailableCommand, AvailableCommandInput, ContentBlock, SessionConfigKind, SessionConfigOption,
    SessionConfigSelectOptions, StreamMessageContent, StreamMessageDirection, StreamReceiver,
    TerminalExitStatus, ToolCall, ToolCallContent, ToolCallLocation, ToolCallStatus,
    ToolCallUpdate, ToolKind,
};
use anyhow::anyhow;
use std::collections::{HashMap, HashSet};
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
            let message_id = next_message_id(&mut state, "acp-message");
            state.active_message_ids.insert(key, message_id.clone());
            message_id
        }
    };
    (callbacks.on_gui_event)(GuiMessageEvent {
        message_id,
        role,
        content,
        presentation: text_presentation_for(role, GuiMessageTextFormat::Markdown),
        is_delta: true,
        is_final: false,
    });
}

pub(crate) fn finalize_active_messages(state: &SharedAcpAgentState, callbacks: &AgentCallbacks) {
    let (active_messages, active_tool_calls) = {
        let mut state = state.lock();
        let active_messages = std::mem::take(&mut state.active_message_ids);
        let active_tool_calls = state
            .tool_call_messages
            .values_mut()
            .filter_map(|tracked| {
                if !tool_call_is_running(tracked.tool_call.status) {
                    return None;
                }
                tracked.tool_call.status = ToolCallStatus::Completed;
                Some((tracked.message_id.clone(), tracked.tool_call.clone()))
            })
            .collect::<Vec<_>>();
        (active_messages, active_tool_calls)
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
            presentation: text_presentation_for(role, GuiMessageTextFormat::Markdown),
            is_delta: true,
            is_final: true,
        });
    }

    for (message_id, tool_call) in active_tool_calls {
        emit_tool_call_message(callbacks, message_id, tool_call);
    }
}

pub(crate) fn emit_session_note(
    state: &SharedAcpAgentState,
    callbacks: &AgentCallbacks,
    key: &str,
    title: &str,
    body: String,
) {
    let body = body.trim().to_string();
    if body.is_empty() {
        return;
    }

    let message_id = {
        let mut state = state.lock();
        if let Some(existing) = state.session_message_ids.get(key) {
            existing.clone()
        } else {
            let message_id = next_message_id(&mut state, "acp-note");
            state
                .session_message_ids
                .insert(key.to_string(), message_id.clone());
            message_id
        }
    };
    let content = format!("**{title}**\n\n{body}");
    (callbacks.on_gui_event)(GuiMessageEvent {
        message_id,
        role: GuiMessageRole::System,
        content: content.clone(),
        presentation: text_presentation_for(GuiMessageRole::System, GuiMessageTextFormat::Markdown)
            .with_text(content.clone()),
        is_delta: false,
        is_final: true,
    });
}

pub(crate) fn update_config_options(
    state: &SharedAcpAgentState,
    config_options: Vec<SessionConfigOption>,
) {
    let mut state = state.lock();
    state.config_options = config_options;
}

pub(crate) fn upsert_tool_call_message(
    state: &SharedAcpAgentState,
    callbacks: &AgentCallbacks,
    tool_call: ToolCall,
) {
    let (message_id, tool_call) = {
        let mut state = state.lock();
        let key = tool_call.tool_call_id.0.to_string();
        if let Some(existing) = state.tool_call_messages.get_mut(&key) {
            existing.tool_call = tool_call;
            (existing.message_id.clone(), existing.tool_call.clone())
        } else {
            let message_id = next_message_id(&mut state, "acp-tool");
            let tracked = TrackedToolCall {
                message_id: message_id.clone(),
                tool_call,
            };
            state.tool_call_messages.insert(key, tracked.clone());
            (tracked.message_id, tracked.tool_call)
        }
    };

    emit_tool_call_message(callbacks, message_id, tool_call);
}

pub(crate) fn apply_tool_call_update(
    state: &SharedAcpAgentState,
    callbacks: &AgentCallbacks,
    update: ToolCallUpdate,
) {
    let maybe_message = {
        let mut state = state.lock();
        let key = update.tool_call_id.0.to_string();
        if let Some(existing) = state.tool_call_messages.get_mut(&key) {
            existing.tool_call.update(update.fields);
            Some((existing.message_id.clone(), existing.tool_call.clone()))
        } else if let Ok(tool_call) = ToolCall::try_from(update) {
            let message_id = next_message_id(&mut state, "acp-tool");
            state.tool_call_messages.insert(
                key,
                TrackedToolCall {
                    message_id: message_id.clone(),
                    tool_call,
                },
            );
            state
                .tool_call_messages
                .values()
                .find(|tracked| tracked.message_id == message_id)
                .map(|tracked| (tracked.message_id.clone(), tracked.tool_call.clone()))
        } else {
            None
        }
    };

    if let Some((message_id, tool_call)) = maybe_message {
        emit_tool_call_message(callbacks, message_id, tool_call);
    }
}

pub(crate) fn available_commands_summary(commands: &[AvailableCommand]) -> String {
    if commands.is_empty() {
        return "No ACP commands are currently exposed.".to_string();
    }

    commands
        .iter()
        .map(|command| {
            let input_hint = match &command.input {
                Some(AvailableCommandInput::Unstructured(input)) => {
                    format!(" {}", truncate_inline(&input.hint, 60))
                }
                Some(_) | None => String::new(),
            };
            format!(
                "- `/{}`{}: {}",
                command.name,
                input_hint,
                command.description.trim()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn config_options_summary(config_options: &[SessionConfigOption]) -> String {
    if config_options.is_empty() {
        return "No ACP session configuration options are currently exposed.".to_string();
    }

    config_options
        .iter()
        .map(|option| {
            let value = selected_config_option_label(option).unwrap_or_else(|| "unset".to_string());
            format!("- **{}**: {}", option.name.trim(), value)
        })
        .collect::<Vec<_>>()
        .join("\n")
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

pub(crate) async fn log_acp_rpc_stream(mut receiver: StreamReceiver, title: String) {
    let mut pending_methods: HashMap<String, String> = HashMap::new();

    loop {
        let message = match receiver.recv().await {
            Ok(message) => message,
            Err(_) => break,
        };

        match (message.direction, message.message) {
            (
                StreamMessageDirection::Outgoing,
                StreamMessageContent::Request { id, method, .. },
            ) => {
                let id = format!("{id:?}");
                let method_name = method.to_string();
                pending_methods.insert(id.clone(), method_name.clone());
            }
            (StreamMessageDirection::Incoming, StreamMessageContent::Response { id, result }) => {
                let id = format!("{id:?}");
                let method_name = pending_methods
                    .remove(&id)
                    .unwrap_or_else(|| "<unknown>".to_string());
                if let Err(error) = result {
                    log::warn!(
                        "ACP {} <- response {} {} error: {:?}",
                        title,
                        id,
                        method_name,
                        error
                    );
                }
            }
            (StreamMessageDirection::Incoming, StreamMessageContent::Notification { .. }) => {}
            _ => {}
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

fn next_message_id(state: &mut AcpAgentState, prefix: &str) -> String {
    state.next_message_id += 1;
    format!("{prefix}-{}", state.next_message_id)
}

fn emit_tool_call_message(callbacks: &AgentCallbacks, message_id: String, tool_call: ToolCall) {
    let content = normalized_tool_call_title(&tool_call);
    (callbacks.on_gui_event)(GuiMessageEvent {
        message_id,
        role: GuiMessageRole::Assistant,
        content: content.clone(),
        presentation: tool_presentation_for(&tool_call),
        is_delta: false,
        is_final: !tool_call_is_running(tool_call.status),
    });
}

fn text_presentation_for(
    role: GuiMessageRole,
    text_format: GuiMessageTextFormat,
) -> GuiMessagePresentation {
    let kind = match role {
        GuiMessageRole::User => GuiMessagePresentationKind::User,
        GuiMessageRole::Assistant | GuiMessageRole::System => GuiMessagePresentationKind::Standard,
        GuiMessageRole::Reasoning => GuiMessagePresentationKind::Reasoning,
    };
    GuiMessagePresentation {
        kind,
        text: None,
        text_format: Some(text_format),
        tool_rows: Vec::new(),
        tool_status_label: None,
        is_tool_running: false,
    }
}

fn tool_presentation_for(tool_call: &ToolCall) -> GuiMessagePresentation {
    GuiMessagePresentation {
        kind: GuiMessagePresentationKind::Tool,
        text: Some(normalized_tool_call_title(tool_call)),
        text_format: Some(GuiMessageTextFormat::Plain),
        tool_rows: tool_rows_for(tool_call),
        tool_status_label: visible_tool_status_label(tool_call.status),
        is_tool_running: tool_call_is_running(tool_call.status),
    }
}

fn tool_rows_for(tool_call: &ToolCall) -> Vec<GuiToolRow> {
    let mut rows = Vec::new();
    let mut seen = HashSet::new();

    push_row(&mut rows, &mut seen, primary_tool_row(tool_call));

    for location in additional_location_rows(tool_call) {
        push_row(&mut rows, &mut seen, location);
    }

    for row in content_rows(tool_call) {
        push_row(&mut rows, &mut seen, row);
    }

    if rows.is_empty() {
        rows.push(GuiToolRow {
            kind: GuiToolRowKind::Text,
            label: "Tool".to_string(),
            value: Some(tool_call.title.clone()),
            path: None,
            added: None,
            removed: None,
        });
    }

    rows
}

fn normalized_tool_call_title(tool_call: &ToolCall) -> String {
    if matches!(tool_call.kind, ToolKind::Edit) {
        let trimmed = tool_call.title.trim();
        if let Some(rest) = trimmed.strip_prefix("Change ") {
            return format!("Edit {rest}");
        }
        if trimmed == "Change" {
            return "Edit".to_string();
        }
    }

    tool_call.title.clone()
}

fn primary_tool_row(tool_call: &ToolCall) -> GuiToolRow {
    let primary_location = tool_call.locations.first();
    let primary_value = primary_tool_value(tool_call);

    match tool_call.kind {
        ToolKind::Execute => GuiToolRow {
            kind: GuiToolRowKind::Command,
            label: "Command".to_string(),
            value: Some(primary_value),
            path: None,
            added: None,
            removed: None,
        },
        ToolKind::Search => GuiToolRow {
            kind: GuiToolRowKind::Search,
            label: "Search".to_string(),
            value: Some(primary_value),
            path: None,
            added: None,
            removed: None,
        },
        ToolKind::Read => location_row(
            "Read",
            GuiToolRowKind::Read,
            primary_location,
            primary_value,
        ),
        ToolKind::Edit | ToolKind::Delete | ToolKind::Move => file_change_primary_row(
            tool_call,
            primary_location,
            primary_value,
        ),
        ToolKind::Think => text_row("Thought", primary_value),
        ToolKind::Fetch => text_row("Fetch", primary_value),
        ToolKind::SwitchMode => text_row("Mode", primary_value),
        ToolKind::Other | _ => text_row("Tool", primary_value),
    }
}

fn additional_location_rows(tool_call: &ToolCall) -> Vec<GuiToolRow> {
    let mut rows = Vec::new();
    let kind = match tool_call.kind {
        ToolKind::Search => GuiToolRowKind::Search,
        ToolKind::Read => GuiToolRowKind::Read,
        ToolKind::Edit | ToolKind::Delete | ToolKind::Move => GuiToolRowKind::Change,
        _ => GuiToolRowKind::Text,
    };

    for location in tool_call.locations.iter().skip(1) {
        rows.push(location_row(
            kind_label(tool_call.kind),
            kind.clone(),
            Some(location),
            String::new(),
        ));
    }

    rows
}

fn content_rows(tool_call: &ToolCall) -> Vec<GuiToolRow> {
    let mut rows = Vec::new();

    for content in &tool_call.content {
        match content {
            ToolCallContent::Diff(diff) => {
                let (added, removed) = diff_line_counts(diff.old_text.as_deref(), &diff.new_text);
                rows.push(GuiToolRow {
                    kind: GuiToolRowKind::Change,
                    label: "Change".to_string(),
                    value: None,
                    path: Some(diff.path.display().to_string()),
                    added,
                    removed,
                });
            }
            ToolCallContent::Content(content) => {
                match &content.content {
                    ContentBlock::Text(text) => {
                        let rendered = text.text.trim();
                        if rendered.is_empty() || !is_file_change_summary(rendered) {
                            continue;
                        }
                    }
                    _ => {
                        let rendered = render_content_block(content.content.clone());
                        if rendered.trim().is_empty() {
                            continue;
                        }
                        rows.push(text_row(
                            content_label(&content.content),
                            truncate_inline(&rendered, 240),
                        ));
                    }
                }
            }
            ToolCallContent::Terminal(terminal) => {
                rows.push(text_row("Terminal", terminal.terminal_id.0.to_string()));
            }
            _ => {}
        }
    }

    rows
}

fn push_row(rows: &mut Vec<GuiToolRow>, seen: &mut HashSet<String>, row: GuiToolRow) {
    let key = format!(
        "{:?}\u{0}{}\u{0}{}\u{0}{}\u{0}{}\u{0}{}",
        row.kind,
        row.label,
        row.value.clone().unwrap_or_default(),
        row.path.clone().unwrap_or_default(),
        row.added.map(|value| value.to_string()).unwrap_or_default(),
        row.removed
            .map(|value| value.to_string())
            .unwrap_or_default(),
    );
    if seen.insert(key) {
        rows.push(row);
    }
}

fn location_row(
    label: &str,
    kind: GuiToolRowKind,
    location: Option<&ToolCallLocation>,
    fallback_value: String,
) -> GuiToolRow {
    if let Some(location) = location {
        let path = location.path.display().to_string();
        let value = location.line.map(|line| format!("{path}:{line}"));
        GuiToolRow {
            kind,
            label: label.to_string(),
            value,
            path: Some(path),
            added: None,
            removed: None,
        }
    } else {
        GuiToolRow {
            kind,
            label: label.to_string(),
            value: non_empty_value(fallback_value),
            path: None,
            added: None,
            removed: None,
        }
    }
}

fn file_change_primary_row(
    tool_call: &ToolCall,
    primary_location: Option<&ToolCallLocation>,
    fallback_value: String,
) -> GuiToolRow {
    if let Some(row) = tool_call_content_change_row(tool_call) {
        return row;
    }

    location_row(
        kind_label(tool_call.kind),
        GuiToolRowKind::Change,
        primary_location,
        fallback_value,
    )
}

fn text_row(label: &str, value: String) -> GuiToolRow {
    GuiToolRow {
        kind: GuiToolRowKind::Text,
        label: label.to_string(),
        value: non_empty_value(value),
        path: None,
        added: None,
        removed: None,
    }
}

fn primary_tool_value(tool_call: &ToolCall) -> String {
    compact_json_string(tool_call.raw_input.as_ref())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| tool_call.title.trim().to_string())
}

fn compact_json_string(value: Option<&serde_json::Value>) -> Option<String> {
    let value = value?;
    if let Some(text) = value.as_str() {
        return Some(truncate_inline(text, 240));
    }
    value
        .as_object()
        .and_then(|object| {
            for key in ["command", "cmd", "query", "path", "file", "input"] {
                if let Some(text) = object.get(key).and_then(serde_json::Value::as_str) {
                    return Some(truncate_inline(text, 240));
                }
            }
            None
        })
        .or_else(|| Some(truncate_inline(&value.to_string(), 240)))
}

fn tool_call_content_change_row(tool_call: &ToolCall) -> Option<GuiToolRow> {
    tool_call.content.iter().find_map(|content| match content {
        ToolCallContent::Content(content) => match &content.content {
            ContentBlock::Text(text) => file_change_row_from_text(&text.text),
            _ => None,
        },
        _ => None,
    })
}

fn file_change_row_from_text(text: &str) -> Option<GuiToolRow> {
    let trimmed = text.trim();
    let (label, remainder) = if let Some(rest) = trimmed.strip_prefix("Created file ") {
        ("Created", rest)
    } else if let Some(rest) = trimmed.strip_prefix("Updated file ") {
        ("Edited", rest)
    } else if let Some(rest) = trimmed.strip_prefix("Changed file ") {
        ("Edited", rest)
    } else if let Some(rest) = trimmed.strip_prefix("Edited file ") {
        ("Edited", rest)
    } else if let Some(rest) = trimmed.strip_prefix("Deleted file ") {
        ("Deleted", rest)
    } else if let Some(rest) = trimmed.strip_prefix("Removed file ") {
        ("Deleted", rest)
    } else if let Some(rest) = trimmed.strip_prefix("Renamed file ") {
        ("Renamed", rest)
    } else {
        return None;
    };

    let path = extract_file_change_path(remainder)?;
    Some(GuiToolRow {
        kind: GuiToolRowKind::Change,
        label: label.to_string(),
        value: None,
        path: Some(path),
        added: None,
        removed: None,
    })
}

fn extract_file_change_path(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    for delimiter in [" with ", " using ", " containing ", " (" , "\n"] {
        if let Some((path, _)) = trimmed.split_once(delimiter) {
            let path = path.trim();
            if !path.is_empty() {
                return Some(path.to_string());
            }
        }
    }

    Some(trimmed.to_string())
}

fn is_file_change_summary(text: &str) -> bool {
    file_change_row_from_text(text).is_some()
}

fn truncate_inline(value: &str, limit: usize) -> String {
    let trimmed = value.trim().replace('\n', " ");
    if trimmed.chars().count() <= limit {
        trimmed
    } else {
        let truncated = trimmed
            .chars()
            .take(limit.saturating_sub(1))
            .collect::<String>();
        format!("{truncated}…")
    }
}

fn non_empty_value(value: String) -> Option<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn tool_call_is_running(status: ToolCallStatus) -> bool {
    matches!(status, ToolCallStatus::Pending | ToolCallStatus::InProgress)
}

fn visible_tool_status_label(status: ToolCallStatus) -> Option<String> {
    match status {
        ToolCallStatus::Failed => Some(format_tool_status(status).to_string()),
        ToolCallStatus::Pending | ToolCallStatus::InProgress | ToolCallStatus::Completed => None,
        _ => None,
    }
}

fn kind_label(kind: ToolKind) -> &'static str {
    match kind {
        ToolKind::Read => "Read",
        ToolKind::Edit => "Change",
        ToolKind::Delete => "Delete",
        ToolKind::Move => "Move",
        ToolKind::Search => "Search",
        ToolKind::Execute => "Command",
        ToolKind::Think => "Thought",
        ToolKind::Fetch => "Fetch",
        ToolKind::SwitchMode => "Mode",
        ToolKind::Other | _ => "Tool",
    }
}

fn content_label(content: &ContentBlock) -> &'static str {
    match content {
        ContentBlock::Text(_) => "Output",
        ContentBlock::Image(_) => "Image",
        ContentBlock::Audio(_) => "Audio",
        ContentBlock::ResourceLink(_) | ContentBlock::Resource(_) | _ => "Resource",
    }
}

fn diff_line_counts(old_text: Option<&str>, new_text: &str) -> (Option<u32>, Option<u32>) {
    let old_lines = old_text.map(line_count).unwrap_or(0);
    let new_lines = line_count(new_text);
    (
        Some(new_lines.saturating_sub(old_lines)),
        Some(old_lines.saturating_sub(new_lines)),
    )
}

fn line_count(text: &str) -> u32 {
    text.lines().count().max(1) as u32
}

fn selected_config_option_label(option: &SessionConfigOption) -> Option<String> {
    match &option.kind {
        SessionConfigKind::Select(select) => match &select.options {
            SessionConfigSelectOptions::Ungrouped(options) => options
                .iter()
                .find(|candidate| candidate.value == select.current_value)
                .map(|candidate| candidate.name.clone()),
            SessionConfigSelectOptions::Grouped(groups) => groups
                .iter()
                .flat_map(|group| group.options.iter())
                .find(|candidate| candidate.value == select.current_value)
                .map(|candidate| candidate.name.clone()),
            _ => None,
        },
        _ => None,
    }
}

trait GuiMessagePresentationExt {
    fn with_text(self, text: String) -> Self;
}

impl GuiMessagePresentationExt for GuiMessagePresentation {
    fn with_text(mut self, text: String) -> Self {
        self.text = Some(text);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{
        available_commands_summary, config_options_summary, finalize_active_messages,
        normalized_tool_call_title, tool_rows_for, upsert_tool_call_message,
    };
    use crate::features::tasks::agents::agent_gui::types::GuiMessageEvent;
    use crate::features::tasks::agents::AgentCallbacks;
    use crate::features::tasks::agents::acp::state::AcpAgentState;
    use crate::features::tasks::TaskStatus;
    use agent_client_protocol::{
        AvailableCommand, Content, ContentBlock, Diff, SessionConfigOption, ToolCall,
        ToolCallStatus, ToolKind, ToolKind::Execute,
    };
    use parking_lot::Mutex;
    use std::sync::Arc;

    #[test]
    fn tool_rows_map_diff_content_into_change_rows() {
        let tool_call = ToolCall::new("tool-1", "Edit file")
            .kind(ToolKind::Edit)
            .content(vec![agent_client_protocol::ToolCallContent::Diff(
                Diff::new("/tmp/example.rs", "new line").old_text("old line"),
            )]);

        let rows = tool_rows_for(&tool_call);
        assert!(rows
            .iter()
            .any(|row| row.path.as_deref() == Some("/tmp/example.rs")));
        assert!(rows.iter().any(|row| matches!(
            row.kind,
            crate::features::tasks::agents::agent_gui::types::GuiToolRowKind::Change
        )));
    }

    #[test]
    fn tool_rows_map_execute_input_into_command_rows() {
        let tool_call = ToolCall::new("tool-1", "Run cargo check")
            .kind(Execute)
            .raw_input(serde_json::json!({ "command": "cargo check" }));

        let rows = tool_rows_for(&tool_call);
        assert!(rows
            .iter()
            .any(|row| row.value.as_deref() == Some("cargo check")));
    }

    #[test]
    fn available_commands_summary_uses_existing_chat_markdown_shape() {
        let summary = available_commands_summary(&[AvailableCommand::new(
            "explain",
            "Explain the current code",
        )]);

        assert!(summary.contains("`/explain`"));
        assert!(summary.contains("Explain the current code"));
    }

    #[test]
    fn config_options_summary_lists_current_selections() {
        let config_option = SessionConfigOption::select(
            "model",
            "Model",
            "gpt-4.1",
            vec![
                agent_client_protocol::SessionConfigSelectOption::new("gpt-4.1", "GPT-4.1"),
                agent_client_protocol::SessionConfigSelectOption::new("gpt-4o", "GPT-4o"),
            ],
        );

        let summary = config_options_summary(&[config_option]);
        assert!(summary.contains("**Model**"));
        assert!(summary.contains("GPT-4.1"));
    }

    #[test]
    fn tool_rows_ignore_plain_text_output_content() {
        let tool_call = ToolCall::new("tool-1", "Show output").content(vec![
            agent_client_protocol::ToolCallContent::Content(Content::new(ContentBlock::from(
                "hello world",
            ))),
        ]);

        let rows = tool_rows_for(&tool_call);
        assert!(!rows.iter().any(|row| row.label == "Output"));
        assert!(!rows
            .iter()
            .any(|row| row.value.as_deref() == Some("hello world")));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].value.as_deref(), Some("Show output"));
    }

    #[test]
    fn tool_rows_map_created_file_summary_into_change_row() {
        let tool_call = ToolCall::new("tool-1", "Change story.md")
            .kind(ToolKind::Edit)
            .content(vec![agent_client_protocol::ToolCallContent::Content(Content::new(
                ContentBlock::from("Created file /tmp/project/story.md with 1134 characters"),
            ))]);

        let rows = tool_rows_for(&tool_call);
        assert!(rows.iter().any(|row| {
            matches!(
                row.kind,
                crate::features::tasks::agents::agent_gui::types::GuiToolRowKind::Change
            ) && row.label == "Created"
                && row.path.as_deref() == Some("/tmp/project/story.md")
        }));
        assert!(!rows.iter().any(|row| {
            row.value.as_deref()
                == Some("Created file /tmp/project/story.md with 1134 characters")
        }));
    }

    #[test]
    fn edit_tool_titles_normalize_change_to_edit() {
        let tool_call = ToolCall::new("tool-1", "Change story.md").kind(ToolKind::Edit);

        assert_eq!(normalized_tool_call_title(&tool_call), "Edit story.md");
    }

    #[test]
    fn finalize_active_messages_completes_running_tool_calls() {
        let events = Arc::new(Mutex::new(Vec::<GuiMessageEvent>::new()));
        let callbacks = test_callbacks(events.clone());
        let state = Arc::new(Mutex::new(AcpAgentState::default()));
        let tool_call = ToolCall::new("tool-1", "Run cargo check")
            .kind(Execute)
            .status(ToolCallStatus::InProgress)
            .raw_input(serde_json::json!({ "command": "cargo check" }));

        upsert_tool_call_message(&state, &callbacks, tool_call);
        finalize_active_messages(&state, &callbacks);

        let emitted = events.lock();
        assert_eq!(emitted.len(), 2);
        let final_event = emitted.last().unwrap();
        assert!(final_event.is_final);
        assert!(!final_event.presentation.is_tool_running);
    }

    fn test_callbacks(events: Arc<Mutex<Vec<GuiMessageEvent>>>) -> AgentCallbacks {
        AgentCallbacks {
            on_output: Arc::new(|_| {}),
            on_status: Arc::new(|_: TaskStatus| {}),
            on_exit: Arc::new(|_| {}),
            on_gui_event: Arc::new(move |event| {
                events.lock().push(event);
            }),
            on_gui_history: Arc::new(|_| {}),
            on_gui_activity: Arc::new(|_| {}),
            on_gui_plan: Arc::new(|_| {}),
            on_gui_token_usage: Arc::new(|_| {}),
            on_gui_request: Arc::new(|_| {}),
            on_gui_hydrated: Arc::new(|| {}),
        }
    }
}
