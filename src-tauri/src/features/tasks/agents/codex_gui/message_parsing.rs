use crate::features::tasks::agents::codex_gui::types::{
    GuiMessageEvent, GuiMessagePresentation, GuiMessagePresentationKind, GuiMessageRole,
    GuiMessageTextFormat, GuiToolRow, GuiToolRowKind,
};
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

pub(super) fn build_presentation(
    role: GuiMessageRole,
    params: &Value,
    item: Option<&Value>,
    method: &str,
    content: &str,
    is_final: bool,
) -> GuiMessagePresentation {
    match role {
        GuiMessageRole::User => text_presentation(
            GuiMessagePresentationKind::User,
            Some(content.to_string()),
            Some(GuiMessageTextFormat::Markdown),
        ),
        GuiMessageRole::Assistant => text_presentation(
            GuiMessagePresentationKind::Standard,
            Some(content.to_string()),
            Some(GuiMessageTextFormat::Markdown),
        ),
        GuiMessageRole::Reasoning => text_presentation(
            GuiMessagePresentationKind::Reasoning,
            Some(normalize_reasoning_text(content)),
            Some(GuiMessageTextFormat::Plain),
        ),
        GuiMessageRole::System => build_system_presentation(params, item, method, content, is_final)
            .unwrap_or_else(|| {
                text_presentation(
                    GuiMessagePresentationKind::Standard,
                    Some(content.to_string()),
                    Some(GuiMessageTextFormat::Markdown),
                )
            }),
    }
}

fn text_presentation(
    kind: GuiMessagePresentationKind,
    text: Option<String>,
    text_format: Option<GuiMessageTextFormat>,
) -> GuiMessagePresentation {
    GuiMessagePresentation {
        kind,
        text,
        text_format,
        tool_rows: Vec::new(),
        tool_status_label: None,
        is_tool_running: false,
    }
}

fn tool_presentation(
    rows: Vec<GuiToolRow>,
    tool_status_label: Option<String>,
    is_tool_running: bool,
) -> GuiMessagePresentation {
    GuiMessagePresentation {
        kind: GuiMessagePresentationKind::Tool,
        text: None,
        text_format: None,
        tool_rows: rows,
        tool_status_label,
        is_tool_running,
    }
}

fn build_system_presentation(
    params: &Value,
    item: Option<&Value>,
    method: &str,
    content: &str,
    is_final: bool,
) -> Option<GuiMessagePresentation> {
    let item_type = item
        .and_then(|value| value.get("type"))
        .and_then(Value::as_str)
        .or_else(|| method.strip_prefix("item/").and_then(|value| value.split('/').next()));

    let rows = match item_type {
        Some("commandExecution") | Some("command_execution") => {
            build_command_tool_rows(params, item, content)
        }
        Some("fileChange") | Some("file_change") => build_file_change_tool_rows(item),
        Some("fileRead") | Some("file_read") => build_file_read_tool_rows(params, item),
        Some("toolCall")
        | Some("tool_call")
        | Some("toolResult")
        | Some("tool_result")
        | Some("mcpToolCall")
        | Some("mcp_tool_call")
        | Some("dynamicToolCall")
        | Some("dynamic_tool_call")
        | Some("collabAgentToolCall")
        | Some("collab_agent_tool_call")
        | Some("webSearch")
        | Some("imageView")
        | Some("contextCompaction")
        | Some("enteredReviewMode")
        | Some("exitedReviewMode") => build_named_tool_rows(params, item, item_type),
        _ => build_fallback_tool_rows(content),
    };

    if rows.is_empty() {
        return None;
    }

    let (tool_status_label, is_tool_running) = build_tool_status(item, is_final);
    Some(tool_presentation(rows, tool_status_label, is_tool_running))
}

fn build_command_tool_rows(params: &Value, item: Option<&Value>, content: &str) -> Vec<GuiToolRow> {
    let Some(command) = item
        .and_then(|value| value.get("command"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| command_from_content(content))
    else {
        return Vec::new();
    };

    let unwrapped_command = unwrap_shell_lc_command(command.as_str());
    let command_source = unwrapped_command.as_deref().unwrap_or(command.as_str());
    let patch_rows = patch_tool_rows(command_source);
    if !patch_rows.is_empty() {
        return patch_rows;
    }

    if let Some(pattern) = extract_rg_pattern(command_source) {
        return vec![GuiToolRow {
            kind: GuiToolRowKind::Search,
            label: "Search".to_string(),
            value: Some(pattern),
            path: None,
            added: None,
            removed: None,
        }];
    }

    if let Some(path) = extract_read_path(command_source) {
        return vec![GuiToolRow {
            kind: GuiToolRowKind::Read,
            label: "Read".to_string(),
            value: None,
            path: Some(path),
            added: None,
            removed: None,
        }];
    }

    let delta = params.get("delta").and_then(Value::as_str).map(str::trim);
    let command_value = delta
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| command_source.to_string());
    vec![GuiToolRow {
        kind: GuiToolRowKind::Command,
        label: "Command".to_string(),
        value: Some(command_value),
        path: None,
        added: None,
        removed: None,
    }]
}

fn build_file_change_tool_rows(item: Option<&Value>) -> Vec<GuiToolRow> {
    let Some(changes) = item
        .and_then(|value| value.get("changes"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    changes
        .iter()
        .filter_map(|change| {
            let path = change.get("path").and_then(Value::as_str)?.trim();
            if path.is_empty() {
                return None;
            }
            let label = match change
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("change")
            {
                "change" | "update" | "edit" => "Edited",
                "create" | "add" => "Created",
                "delete" | "remove" => "Deleted",
                "rename" => "Renamed",
                _ => "Edited",
            };
            let diff = change.get("diff").and_then(Value::as_str).unwrap_or("");
            let (added, removed) = count_diff_lines(diff);
            Some(GuiToolRow {
                kind: GuiToolRowKind::Change,
                label: label.to_string(),
                value: None,
                path: Some(path.to_string()),
                added: Some(added as u32),
                removed: Some(removed as u32),
            })
        })
        .collect()
}

fn build_file_read_tool_rows(params: &Value, item: Option<&Value>) -> Vec<GuiToolRow> {
    let path = item
        .and_then(|value| value.get("path"))
        .and_then(Value::as_str)
        .or_else(|| item.and_then(|value| value.get("filePath")).and_then(Value::as_str))
        .or_else(|| item.and_then(|value| value.pointer("/file/path")).and_then(Value::as_str))
        .or_else(|| params.get("path").and_then(Value::as_str))
        .or_else(|| params.get("filePath").and_then(Value::as_str))
        .or_else(|| params.pointer("/file/path").and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty());

    path.map(|value| {
        vec![GuiToolRow {
            kind: GuiToolRowKind::Read,
            label: "Read".to_string(),
            value: None,
            path: Some(value.to_string()),
            added: None,
            removed: None,
        }]
    })
    .unwrap_or_default()
}

fn build_named_tool_rows(
    params: &Value,
    item: Option<&Value>,
    item_type: Option<&str>,
) -> Vec<GuiToolRow> {
    let Some(item_type) = item_type else {
        return Vec::new();
    };
    let tool_name = item
        .and_then(|value| value.get("toolName"))
        .and_then(Value::as_str)
        .or_else(|| item.and_then(|value| value.get("name")).and_then(Value::as_str))
        .or_else(|| item.and_then(|value| value.get("tool")).and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let path = item
        .and_then(|value| value.get("path"))
        .and_then(Value::as_str)
        .or_else(|| item.and_then(|value| value.get("filePath")).and_then(Value::as_str))
        .or_else(|| params.get("path").and_then(Value::as_str))
        .or_else(|| params.get("filePath").and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let value = match item_type {
        "webSearch" => item
            .and_then(|value| value.get("query"))
            .and_then(Value::as_str)
            .map(str::to_string),
        "imageView" => path.clone(),
        "contextCompaction" => Some("Context compacted".to_string()),
        "enteredReviewMode" | "exitedReviewMode" => item
            .and_then(|value| value.get("review"))
            .and_then(Value::as_str)
            .map(str::to_string),
        _ => None,
    };

    let label = match item_type {
        "mcpToolCall" | "mcp_tool_call" => "MCP".to_string(),
        "dynamicToolCall" | "dynamic_tool_call" => "Dynamic tool".to_string(),
        "collabAgentToolCall" | "collab_agent_tool_call" => "Collaboration".to_string(),
        "webSearch" => "Web search".to_string(),
        "imageView" => "Viewed image".to_string(),
        "contextCompaction" => "Context".to_string(),
        "enteredReviewMode" => "Entered review".to_string(),
        "exitedReviewMode" => "Exited review".to_string(),
        _ => tool_name.unwrap_or("Tool").to_string(),
    };

    vec![GuiToolRow {
        kind: GuiToolRowKind::Text,
        label,
        value,
        path,
        added: None,
        removed: None,
    }]
}

fn build_fallback_tool_rows(content: &str) -> Vec<GuiToolRow> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let patch_rows = patch_tool_rows(trimmed);
    if !patch_rows.is_empty() {
        return patch_rows;
    }
    if let Some(command) = command_from_content(trimmed) {
        if let Some(pattern) = extract_rg_pattern(command.as_str()) {
            return vec![GuiToolRow {
                kind: GuiToolRowKind::Search,
                label: "Search".to_string(),
                value: Some(pattern),
                path: None,
                added: None,
                removed: None,
            }];
        }
        if let Some(path) = extract_read_path(command.as_str()) {
            return vec![GuiToolRow {
                kind: GuiToolRowKind::Read,
                label: "Read".to_string(),
                value: None,
                path: Some(path),
                added: None,
                removed: None,
            }];
        }
        return vec![GuiToolRow {
            kind: GuiToolRowKind::Command,
            label: "Command".to_string(),
            value: Some(command),
            path: None,
            added: None,
            removed: None,
        }];
    }
    trimmed
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| GuiToolRow {
            kind: GuiToolRowKind::Text,
            label: "Tool".to_string(),
            value: Some(line.trim_start_matches("- ").to_string()),
            path: None,
            added: None,
            removed: None,
        })
        .collect()
}

fn build_tool_status(item: Option<&Value>, is_final: bool) -> (Option<String>, bool) {
    let status = item
        .and_then(|value| value.get("status"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let exit_code = item
        .and_then(|value| value.get("exitCode"))
        .and_then(Value::as_i64)
        .unwrap_or(0);

    match status {
        "inProgress" => (None, true),
        "failed" => (Some("FAILED".to_string()), false),
        "declined" => (Some("DECLINED".to_string()), false),
        "completed" if exit_code != 0 => (Some("FAILED".to_string()), false),
        "completed" => (None, false),
        _ if !is_final => (None, true),
        _ => (None, false),
    }
}

fn normalize_reasoning_text(content: &str) -> String {
    let trimmed = content.trim();
    let body = if trimmed == "Reasoning" {
        ""
    } else if let Some(stripped) = trimmed.strip_prefix("Reasoning\n") {
        stripped
    } else {
        trimmed
    };

    body.lines()
        .map(|line| strip_reasoning_wrapper(line.trim()))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_reasoning_wrapper(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.len() > 4 {
        if trimmed.starts_with("**") && trimmed.ends_with("**") {
            return trimmed[2..trimmed.len() - 2].trim();
        }
        if trimmed.starts_with("__") && trimmed.ends_with("__") {
            return trimmed[2..trimmed.len() - 2].trim();
        }
    }
    trimmed
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

fn command_from_content(content: &str) -> Option<String> {
    let line = content
        .lines()
        .find(|line| line.trim_start().starts_with("$ "))?
        .trim();
    Some(line.trim_start_matches("$ ").trim().to_string())
}

fn extract_read_path(command: &str) -> Option<String> {
    if let Some(rest) = command.strip_prefix("cat ") {
        let candidate = rest.trim();
        if !candidate.is_empty() {
            return Some(unquote_shell_argument(candidate));
        }
    }

    let sed_prefix = "sed -n ";
    if let Some(rest) = command.strip_prefix(sed_prefix) {
        let mut tokens = split_command_tokens(rest);
        if tokens.len() >= 2 {
            return Some(unquote_shell_argument(tokens.pop()?.as_str()));
        }
    }
    None
}

fn extract_rg_pattern(command: &str) -> Option<String> {
    let tokens = split_command_tokens(command);
    let first = tokens.first()?;
    let binary = unquote_shell_argument(first);
    let binary_base = binary.rsplit('/').next().unwrap_or(binary.as_str());
    if binary_base != "rg" {
        return None;
    }

    let mut patterns: Vec<String> = Vec::new();
    let mut index = 1usize;
    while index < tokens.len() {
        let token = tokens[index].as_str();
        if token == "--" {
            if let Some(next) = tokens.get(index + 1) {
                patterns.push(unquote_shell_argument(next));
            }
            break;
        }
        if token == "-e" || token == "--regexp" {
            if let Some(next) = tokens.get(index + 1) {
                patterns.push(unquote_shell_argument(next));
                index += 1;
            }
            index += 1;
            continue;
        }
        if let Some(value) = token.strip_prefix("--regexp=") {
            patterns.push(unquote_shell_argument(value));
            index += 1;
            continue;
        }
        if token.starts_with("-e") && token.len() > 2 {
            patterns.push(unquote_shell_argument(&token[2..]));
            index += 1;
            continue;
        }
        if token.starts_with('-') {
            index += 1;
            continue;
        }
        patterns.push(unquote_shell_argument(token));
        break;
    }

    if patterns.is_empty() {
        None
    } else {
        Some(patterns.join(" | "))
    }
}

fn split_command_tokens(command: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut chars = command.chars().peekable();

    while let Some(ch) = chars.next() {
        match quote {
            Some(active) if ch == active => {
                current.push(ch);
                quote = None;
            }
            Some(_) => {
                current.push(ch);
            }
            None if ch == '"' || ch == '\'' => {
                quote = Some(ch);
                current.push(ch);
            }
            None if ch.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(current);
                    current = String::new();
                }
            }
            None => current.push(ch),
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn unwrap_shell_lc_command(command: &str) -> Option<String> {
    for prefix in ["/bin/bash -lc ", "bash -lc ", "/bin/sh -lc ", "sh -lc "] {
        if let Some(rest) = command.strip_prefix(prefix) {
            return Some(unquote_shell_argument(rest.trim()));
        }
    }

    let tokens = split_command_tokens(command);
    if tokens.len() < 3 {
        return None;
    }

    let shell = unquote_shell_argument(tokens[0].as_str());
    let shell_base = shell.rsplit('/').next().unwrap_or(shell.as_str());
    if !matches!(shell_base, "bash" | "sh" | "zsh" | "fish") {
        return None;
    }
    if !matches!(tokens[1].as_str(), "-lc" | "-c") {
        return None;
    }

    Some(unquote_shell_argument(tokens[2].trim()))
}

fn unquote_shell_argument(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() < 2 {
        return trimmed.to_string();
    }
    if trimmed.starts_with('"') && trimmed.ends_with('"') {
        return trimmed[1..trimmed.len() - 1]
            .replace("\\\"", "\"")
            .replace("\\\\", "\\");
    }
    if trimmed.starts_with('\'') && trimmed.ends_with('\'') {
        return trimmed[1..trimmed.len() - 1].replace("\\'", "'");
    }
    trimmed.to_string()
}

fn patch_tool_rows(text: &str) -> Vec<GuiToolRow> {
    if !text.contains("apply_patch <<") {
        return Vec::new();
    }

    let mut rows = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut push_row = |label: &str, path: &str| {
        let normalized = path.trim();
        if normalized.is_empty() {
            return;
        }
        let key = format!("{label}:{normalized}");
        if !seen.insert(key) {
            return;
        }
        rows.push(GuiToolRow {
            kind: GuiToolRowKind::Change,
            label: label.to_string(),
            value: None,
            path: Some(normalized.to_string()),
            added: None,
            removed: None,
        });
    };

    for line in text.lines() {
        if let Some(path) = line.strip_prefix("*** Update File: ") {
            push_row("Edited", path);
            continue;
        }
        if let Some(path) = line.strip_prefix("*** Add File: ") {
            push_row("Created", path);
            continue;
        }
        if let Some(path) = line.strip_prefix("*** Delete File: ") {
            push_row("Deleted", path);
            continue;
        }
        if let Some(path) = line.strip_prefix("*** Move to: ") {
            push_row("Renamed", path);
        }
    }

    rows
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
        .or_else(|| value.pointer("/result/turns").and_then(Value::as_array))
        .or_else(|| {
            value.get("result")
                .and_then(find_turns_array)
        });
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
            let presentation =
                build_presentation(role, &Value::Null, Some(item), synthetic_method.as_str(), &content, true);

            events.push(GuiMessageEvent {
                message_id,
                role,
                content,
                presentation,
                is_delta: false,
                is_final: true,
            });
        }
    }
    events
}

fn find_turns_array(value: &Value) -> Option<&Vec<Value>> {
    match value {
        Value::Object(map) => {
            if let Some(turns) = map.get("turns").and_then(Value::as_array) {
                return Some(turns);
            }
            map.values().find_map(find_turns_array)
        }
        Value::Array(items) => items.iter().find_map(find_turns_array),
        _ => None,
    }
}

pub(super) fn fallback_message_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_millis(0))
        .as_millis();
    format!("assistant-{}", millis)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn command_tool_rows_normalize_absolute_path_shell_cat_to_read() {
        let params = json!({});
        let item = json!({
            "type": "commandExecution",
            "command": "/usr/bin/bash -lc 'cat package.json'",
        });

        let rows = build_command_tool_rows(&params, Some(&item), "");

        assert_eq!(rows.len(), 1);
        assert!(matches!(rows[0].kind, GuiToolRowKind::Read));
        assert_eq!(rows[0].label, "Read");
        assert_eq!(rows[0].path.as_deref(), Some("package.json"));
        assert!(rows[0].value.is_none());
    }

    #[test]
    fn command_tool_rows_normalize_absolute_path_shell_rg_to_search() {
        let params = json!({});
        let item = json!({
            "type": "commandExecution",
            "command": "/usr/bin/bash -lc 'rg foo src'",
        });

        let rows = build_command_tool_rows(&params, Some(&item), "");

        assert_eq!(rows.len(), 1);
        assert!(matches!(rows[0].kind, GuiToolRowKind::Search));
        assert_eq!(rows[0].label, "Search");
        assert_eq!(rows[0].value.as_deref(), Some("foo"));
        assert!(rows[0].path.is_none());
    }

    #[test]
    fn command_tool_rows_unwrap_generic_shell_command_value() {
        let params = json!({});
        let item = json!({
            "type": "commandExecution",
            "command": "/usr/bin/bash -lc 'echo \"or whatever\"'",
        });

        let rows = build_command_tool_rows(&params, Some(&item), "");

        assert_eq!(rows.len(), 1);
        assert!(matches!(rows[0].kind, GuiToolRowKind::Command));
        assert_eq!(rows[0].value.as_deref(), Some("echo \"or whatever\""));
    }
}
