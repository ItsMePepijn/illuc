use crate::features::tasks::agents::{GuiMessageEvent, GuiMessageRole};
use serde_json::Value;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(super) fn extract_role(item: Option<&Value>, method: &str) -> Option<GuiMessageRole> {
    let item_type = item
        .and_then(|value| value.get("type"))
        .and_then(|value| value.as_str());
    match item_type {
        Some("agentMessage") | Some("agent_message") => Some(GuiMessageRole::Assistant),
        Some("systemMessage") | Some("system_message") => Some(GuiMessageRole::System),
        Some("commandExecution") | Some("command_execution") => Some(GuiMessageRole::System),
        Some("toolCall") | Some("tool_call") => Some(GuiMessageRole::System),
        Some("toolResult") | Some("tool_result") => Some(GuiMessageRole::System),
        Some("fileChange") | Some("file_change") => Some(GuiMessageRole::System),
        Some("fileRead") | Some("file_read") => Some(GuiMessageRole::System),
        Some("plan") => Some(GuiMessageRole::System),
        Some("reasoning") => Some(GuiMessageRole::Reasoning),
        Some("mcpToolCall") | Some("dynamicToolCall") | Some("collabAgentToolCall") => {
            Some(GuiMessageRole::System)
        }
        Some("webSearch") | Some("imageView") | Some("contextCompaction") => {
            Some(GuiMessageRole::System)
        }
        Some("enteredReviewMode") | Some("exitedReviewMode") => Some(GuiMessageRole::System),
        Some("userMessage") | Some("user_message") => None,
        _ if method.starts_with("item/agentMessage/")
            || method.starts_with("item/agent_message/") =>
        {
            Some(GuiMessageRole::Assistant)
        }
        _ if method.starts_with("item/systemMessage/")
            || method.starts_with("item/system_message/") =>
        {
            Some(GuiMessageRole::System)
        }
        _ if method.starts_with("item/commandExecution/")
            || method.starts_with("item/command_execution/") =>
        {
            Some(GuiMessageRole::System)
        }
        _ if method.starts_with("item/toolCall/") || method.starts_with("item/tool_call/") => {
            Some(GuiMessageRole::System)
        }
        _ if method.starts_with("item/toolResult/") || method.starts_with("item/tool_result/") => {
            Some(GuiMessageRole::System)
        }
        _ if method.starts_with("item/fileChange/") || method.starts_with("item/file_change/") => {
            Some(GuiMessageRole::System)
        }
        _ if method.starts_with("item/fileRead/") || method.starts_with("item/file_read/") => {
            Some(GuiMessageRole::System)
        }
        _ if method.starts_with("item/plan/")
            || method.starts_with("item/mcpToolCall/")
            || method.starts_with("item/dynamicToolCall/")
            || method.starts_with("item/collabAgentToolCall/")
            || method.starts_with("item/webSearch/")
            || method.starts_with("item/imageView/")
            || method.starts_with("item/contextCompaction/")
            || method.starts_with("item/enteredReviewMode/")
            || method.starts_with("item/exitedReviewMode/") =>
        {
            Some(GuiMessageRole::System)
        }
        _ if method.starts_with("item/reasoning/") => Some(GuiMessageRole::Reasoning),
        _ => None,
    }
}

pub(super) fn extract_content(params: &Value, item: Option<&Value>) -> String {
    if is_reasoning_summary_item(item) {
        return String::new();
    }
    if let Some(command_text) = extract_command_execution_content(params, item) {
        return command_text;
    }
    if let Some(tool_call_text) = extract_tool_call_content(params, item) {
        return tool_call_text;
    }
    if let Some(file_change_text) = extract_file_change_content(params, item) {
        return file_change_text;
    }
    if let Some(file_read_text) = extract_file_read_content(params, item) {
        return file_read_text;
    }
    if let Some(special_item_text) = extract_special_item_content(params, item) {
        return special_item_text;
    }

    if let Some(delta) = params.get("delta") {
        let text = text_from_value(delta);
        if !text.is_empty() {
            return text;
        }
    }

    if let Some(message) = params.get("message") {
        let text = text_from_value(message);
        if !text.is_empty() {
            return text;
        }
    }

    if let Some(item) = item {
        let text = text_from_value(item);
        if !text.is_empty() {
            return text;
        }
    }

    String::new()
}

fn is_reasoning_summary_item(item: Option<&Value>) -> bool {
    let Some(item) = item else {
        return false;
    };
    matches!(
        item.get("type").and_then(Value::as_str),
        Some("reasoning")
    ) && item.get("summary").is_some()
}

fn extract_command_execution_content(params: &Value, item: Option<&Value>) -> Option<String> {
    let item = item?;
    let item_type = item.get("type").and_then(Value::as_str)?;
    if item_type != "commandExecution" && item_type != "command_execution" {
        return None;
    }

    if let Some(delta) = params.get("delta").and_then(Value::as_str) {
        if !delta.is_empty() {
            return Some(delta.to_string());
        }
    }

    let command = item.get("command").and_then(Value::as_str).unwrap_or("");
    let status = item.get("status").and_then(Value::as_str).unwrap_or("");
    let exit_code = item
        .get("exitCode")
        .and_then(Value::as_i64)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "?".to_string());

    if !command.is_empty() && status == "inProgress" {
        return Some(format!("$ {}\n", command));
    }

    if status == "completed" || status == "failed" || status == "declined" {
        let mut summary = if !command.is_empty() {
            format!("$ {}\n", command)
        } else {
            String::new()
        };
        summary.push_str(format!("[{} exit {}]", status, exit_code).as_str());
        return Some(summary);
    }

    if !command.is_empty() {
        return Some(format!("$ {}", command));
    }

    None
}

fn extract_tool_call_content(params: &Value, item: Option<&Value>) -> Option<String> {
    let item = item?;
    let item_type = item.get("type").and_then(Value::as_str)?;
    if item_type != "toolCall"
        && item_type != "tool_call"
        && item_type != "toolResult"
        && item_type != "tool_result"
    {
        return None;
    }

    if let Some(delta) = params.get("delta").and_then(Value::as_str) {
        if !delta.is_empty() {
            return Some(delta.to_string());
        }
    }

    if let Some(command) = item.get("command").and_then(Value::as_str) {
        if !command.is_empty() {
            return Some(format!("$ {}", command));
        }
    }

    let tool_name = item
        .get("toolName")
        .and_then(Value::as_str)
        .or_else(|| item.get("name").and_then(Value::as_str))
        .unwrap_or("tool");
    let target = item
        .get("path")
        .and_then(Value::as_str)
        .or_else(|| item.get("filePath").and_then(Value::as_str));

    if let Some(path) = target {
        let encoded_path = encode_fragment(path);
        return Some(format!(
            "- {} [{}](#diff:{})",
            tool_name, path, encoded_path
        ));
    }

    Some(format!("- {}", tool_name))
}

fn extract_file_change_content(params: &Value, item: Option<&Value>) -> Option<String> {
    let item = item?;
    let item_type = item.get("type").and_then(Value::as_str)?;
    if item_type != "fileChange" && item_type != "file_change" {
        return None;
    }

    if params.get("delta").is_some() {
        return None;
    }

    let status = item
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let changes = item.get("changes").and_then(Value::as_array)?;
    if changes.is_empty() {
        return Some(format!("File changes [{}]", status));
    }

    let mut lines = vec![format!("File changes [{}]:", status)];
    for change in changes {
        let path = change
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let kind = change
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("change");
        let action = match kind {
            "change" | "update" | "edit" => "Edited",
            "create" | "add" => "Created",
            "delete" | "remove" => "Deleted",
            "rename" => "Renamed",
            _ => "Edited",
        };
        let diff = change.get("diff").and_then(Value::as_str).unwrap_or("");
        let (added, removed) = count_diff_lines(diff);
        let encoded_path = encode_fragment(path);
        lines.push(format!(
            "- {} [{}](#diff:{}) (+{} -{})",
            action, path, encoded_path, added, removed
        ));
    }
    Some(lines.join("\n"))
}

fn extract_file_read_content(params: &Value, item: Option<&Value>) -> Option<String> {
    let item = item?;
    let item_type = item.get("type").and_then(Value::as_str)?;
    if item_type != "fileRead" && item_type != "file_read" {
        return None;
    }

    if params.get("delta").is_some() {
        return None;
    }

    let path = item
        .get("path")
        .and_then(Value::as_str)
        .or_else(|| item.get("filePath").and_then(Value::as_str))
        .or_else(|| item.pointer("/file/path").and_then(Value::as_str))
        .or_else(|| params.get("path").and_then(Value::as_str))
        .or_else(|| params.get("filePath").and_then(Value::as_str))
        .or_else(|| params.pointer("/file/path").and_then(Value::as_str))
        .unwrap_or("unknown");
    let encoded_path = encode_fragment(path);
    Some(format!("- Read [{}](#diff:{})", path, encoded_path))
}

fn extract_special_item_content(params: &Value, item: Option<&Value>) -> Option<String> {
    let item = item?;
    let item_type = item.get("type").and_then(Value::as_str)?;
    if params.get("delta").is_some() {
        return None;
    }
    match item_type {
        "plan" => Some(
            item.get("text")
                .and_then(Value::as_str)
                .map(|text| format!("Plan\n{}", text.trim()))
                .unwrap_or_else(|| "Plan".to_string()),
        ),
        "reasoning" => None,
        "webSearch" => item
            .get("query")
            .and_then(Value::as_str)
            .map(|query| format!("- Web search {}", query)),
        "imageView" => item
            .get("path")
            .and_then(Value::as_str)
            .map(|path| format!("- Viewed image {}", path)),
        "enteredReviewMode" => item
            .get("review")
            .and_then(Value::as_str)
            .map(|review| format!("Entered review mode: {}", review)),
        "exitedReviewMode" => item
            .get("review")
            .and_then(Value::as_str)
            .map(|review| format!("Exited review mode: {}", review)),
        "contextCompaction" => Some("Compacted context".to_string()),
        "mcpToolCall" => item
            .get("tool")
            .and_then(Value::as_str)
            .map(|tool| format!("- MCP tool {}", tool)),
        "dynamicToolCall" => item
            .get("tool")
            .and_then(Value::as_str)
            .map(|tool| format!("- Dynamic tool {}", tool)),
        "collabAgentToolCall" => item
            .get("tool")
            .and_then(Value::as_str)
            .map(|tool| format!("- Collaboration {}", tool)),
        _ => None,
    }
}

fn count_diff_lines(diff: &str) -> (usize, usize) {
    let mut added = 0usize;
    let mut removed = 0usize;
    for line in diff.lines() {
        if line.starts_with("+++") || line.starts_with("---") {
            continue;
        }
        if line.starts_with('+') {
            added += 1;
            continue;
        }
        if line.starts_with('-') {
            removed += 1;
        }
    }
    (added, removed)
}

fn encode_fragment(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~' | b'/') {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(format!("%{:02X}", byte).as_str());
        }
    }
    encoded
}

fn text_from_value(value: &Value) -> String {
    if let Some(text) = value.as_str() {
        return text.to_string();
    }

    if let Some(text) = value.get("text").and_then(|value| value.as_str()) {
        return text.to_string();
    }

    if let Some(content) = value.get("content") {
        if let Some(text) = content.as_str() {
            return text.to_string();
        }
        if let Some(items) = content.as_array() {
            let mut merged = String::new();
            for item in items {
                merged.push_str(text_from_value(item).as_str());
            }
            if !merged.is_empty() {
                return merged;
            }
        }
    }

    if let Some(parts) = value.as_array() {
        let mut merged = String::new();
        for part in parts {
            merged.push_str(text_from_value(part).as_str());
        }
        return merged;
    }

    String::new()
}

pub(super) fn extract_message_id(params: &Value, item: Option<&Value>) -> Option<String> {
    params
        .get("itemId")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
        .or_else(|| {
            params
                .get("messageId")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string())
        })
        .or_else(|| {
            params
                .get("id")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string())
        })
        .or_else(|| {
            item.and_then(|value| value.get("id"))
                .and_then(|value| value.as_str())
                .map(|value| value.to_string())
        })
}

pub(super) fn collect_history_events(value: &Value) -> Vec<GuiMessageEvent> {
    let turns = value
        .pointer("/result/thread/turns")
        .and_then(Value::as_array)
        .or_else(|| value.pointer("/result/turns").and_then(Value::as_array));
    let Some(turns) = turns else {
        return Vec::new();
    };

    let mut events = Vec::new();
    for turn in turns {
        let items = turn
            .get("items")
            .and_then(Value::as_array)
            .or_else(|| turn.get("events").and_then(Value::as_array))
            .or_else(|| turn.get("messages").and_then(Value::as_array));
        let Some(items) = items else {
            continue;
        };
        for item in items {
            let item_type = item.get("type").and_then(Value::as_str);
            if is_reasoning_summary_item(Some(item)) {
                continue;
            }
            let is_command = matches!(
                item_type,
                Some("commandExecution") | Some("command_execution")
            );
            let is_file_change = matches!(item_type, Some("fileChange") | Some("file_change"));
            let is_file_read = matches!(item_type, Some("fileRead") | Some("file_read"));
            let synthetic_method = item_type
                .map(|kind| format!("item/{}/completed", kind))
                .unwrap_or_else(|| "item/unknown/completed".to_string());
            let role = match item_type {
                Some("userMessage") | Some("user_message") => Some(GuiMessageRole::User),
                _ => extract_role(Some(item), synthetic_method.as_str()).or_else(|| {
                    match item.get("role").and_then(Value::as_str) {
                        Some("user") => Some(GuiMessageRole::User),
                        Some("assistant") => Some(GuiMessageRole::Assistant),
                        Some("system") => Some(GuiMessageRole::System),
                        Some("reasoning") => Some(GuiMessageRole::Reasoning),
                        _ => None,
                    }
                }),
            };
            let Some(role) = role else {
                continue;
            };

            let content = if is_command {
                extract_command_execution_content(&Value::Null, Some(item))
                    .unwrap_or_else(|| text_from_value(item))
            } else if is_file_change {
                extract_file_change_content(&Value::Null, Some(item))
                    .unwrap_or_else(|| text_from_value(item))
            } else if is_file_read {
                extract_file_read_content(&Value::Null, Some(item))
                    .unwrap_or_else(|| text_from_value(item))
            } else {
                let extracted = extract_content(&Value::Null, Some(item));
                if extracted.trim().is_empty() {
                    let source = item.get("content").unwrap_or(item);
                    text_from_value(source)
                } else {
                    extracted
                }
            };
            if content.trim().is_empty() {
                continue;
            }

            let message_id = item
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(fallback_message_id);

            events.push(GuiMessageEvent {
                message_id,
                role,
                content,
                is_delta: false,
                is_final: true,
            });
        }
    }
    events
}

pub(super) fn fallback_message_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_millis(0))
        .as_millis();
    format!("assistant-{}", millis)
}
