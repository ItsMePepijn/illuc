use super::super::message_parsing::{
    build_presentation, extract_content, extract_message_id, extract_role, fallback_message_id,
};
use super::super::CodexGuiAgentState;
use crate::features::tasks::agents::agent_gui::types::{
    GuiActivityEvent, GuiMessageEvent, GuiPlanEvent, GuiPlanStep, GuiTokenUsageEvent,
};
use crate::features::tasks::agents::AgentCallbacks;
use crate::features::tasks::TaskStatus;
use chrono::Utc;
use parking_lot::Mutex;
use serde_json::Value;
use std::sync::Arc;

pub(super) fn handle_notification(state: &Arc<Mutex<CodexGuiAgentState>>, value: &Value) {
    let method = value.get("method").and_then(|value| value.as_str());
    let Some(method) = method else {
        return;
    };

    if method == "turn/started" {
        let callbacks = {
            let mut guard = state.lock();
            guard.current_turn_id = value
                .pointer("/params/turn/id")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| {
                    value
                        .pointer("/params/turnId")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                });
            set_activity(&mut guard, Some("Working".to_string()), true);
            guard.callbacks.clone()
        };
        if let Some(callbacks) = callbacks {
            (callbacks.on_status)(TaskStatus::Working);
            emit_activity_from_state(state, &callbacks);
        }
        return;
    }
    if method == "turn/completed" {
        let callbacks = {
            let mut guard = state.lock();
            guard.current_turn_id = None;
            set_activity(&mut guard, None, false);
            guard.callbacks.clone()
        };
        if let Some(callbacks) = callbacks {
            (callbacks.on_status)(TaskStatus::Idle);
            emit_activity_from_state(state, &callbacks);
            (callbacks.on_gui_plan)(GuiPlanEvent {
                explanation: None,
                plan: Vec::new(),
            });
        }
        return;
    }

    if method == "turn/plan/updated" {
        if let Some(callbacks) = update_activity_from_notification(state, value, "Planning", true) {
            emit_activity_from_state(state, &callbacks);
            (callbacks.on_gui_plan)(build_plan_event(value));
        }
        return;
    }

    if method == "thread/tokenUsage/updated" {
        if let Some(callbacks) = state.lock().callbacks.clone() {
            if let Some(token_usage) = parse_token_usage_event(value) {
                (callbacks.on_gui_token_usage)(token_usage);
            }
        }
        return;
    }

    if method == "thread/status/changed" {
        if let Some((callbacks, status)) = update_activity_from_thread_status(state, value) {
            (callbacks.on_status)(status);
            emit_activity_from_state(state, &callbacks);
        }
        return;
    }

    if method == "item/started" {
        if let Some(callbacks) = update_activity_from_notification(state, value, "Working", false) {
            emit_activity_from_state(state, &callbacks);
        }
    }

    if method.starts_with("item/reasoning/") {
        if let Some(callbacks) = update_activity_from_notification(state, value, "Thinking", false)
        {
            emit_activity_from_state(state, &callbacks);
        }
    }

    if !method.starts_with("item/") {
        return;
    }

    let params = value.get("params").unwrap_or(&Value::Null);
    let item = params.get("item");

    if method.starts_with("item/reasoning/") && has_reasoning_summary(value, item) {
        return;
    }

    let Some(role) = extract_role(item, method) else {
        return;
    };
    let message_id = extract_message_id(params, item).unwrap_or_else(fallback_message_id);

    let is_delta = method.ends_with("/delta") || params.get("delta").is_some();
    let is_final = method.ends_with("/completed");

    let content = extract_content(params, item);
    if content.is_empty() && !is_final {
        return;
    }
    let presentation = build_presentation(role, params, item, method, &content, is_final);

    if let Some(callbacks) = state.lock().callbacks.clone() {
        (callbacks.on_gui_event)(GuiMessageEvent {
            message_id,
            role,
            content,
            presentation,
            is_delta,
            is_final,
        });
    }
}

fn update_activity_from_thread_status(
    state: &Arc<Mutex<CodexGuiAgentState>>,
    value: &Value,
) -> Option<(AgentCallbacks, TaskStatus)> {
    let mut guard = state.lock();
    let active_flags = value
        .pointer("/params/status/activeFlags")
        .and_then(Value::as_array);
    let waiting_on_approval = active_flags
        .map(|flags| {
            flags.iter().any(|flag| {
                flag.as_str() == Some("waitingOnApproval")
                    || flag
                        .get("type")
                        .and_then(Value::as_str)
                        .is_some_and(|kind| kind == "waitingOnApproval")
            })
        })
        .unwrap_or(false);
    if waiting_on_approval {
        set_activity(&mut guard, Some("Waiting for approval".to_string()), false);
        return guard
            .callbacks
            .clone()
            .map(|callbacks| (callbacks, TaskStatus::AwaitingApproval));
    } else if matches!(
        value.pointer("/params/status/type").and_then(Value::as_str),
        Some("idle" | "notLoaded" | "systemError")
    ) {
        set_activity(&mut guard, None, false);
        return guard
            .callbacks
            .clone()
            .map(|callbacks| (callbacks, TaskStatus::Idle));
    } else if guard.current_turn_id.is_some() {
        set_activity(&mut guard, Some("Working".to_string()), false);
        return guard
            .callbacks
            .clone()
            .map(|callbacks| (callbacks, TaskStatus::Working));
    }
    None
}

fn update_activity_from_notification(
    state: &Arc<Mutex<CodexGuiAgentState>>,
    value: &Value,
    fallback: &str,
    reset_started_at: bool,
) -> Option<AgentCallbacks> {
    let label = activity_label_from_notification(value).unwrap_or_else(|| fallback.to_string());
    let mut guard = state.lock();
    set_activity(&mut guard, Some(label), reset_started_at);
    guard.callbacks.clone()
}

fn emit_activity_from_state(state: &Arc<Mutex<CodexGuiAgentState>>, callbacks: &AgentCallbacks) {
    let (label, started_at) = {
        let guard = state.lock();
        (
            guard.activity_label.clone(),
            guard.activity_started_at.clone(),
        )
    };
    (callbacks.on_gui_activity)(GuiActivityEvent { label, started_at });
}

fn set_activity(state: &mut CodexGuiAgentState, label: Option<String>, reset_started_at: bool) {
    state.activity_label = label.and_then(|value| {
        let normalized = value.trim().to_string();
        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    });
    if state.activity_label.is_none() {
        state.activity_started_at = None;
    } else if reset_started_at || state.activity_started_at.is_none() {
        state.activity_started_at = Some(Utc::now());
    }
}

fn activity_label_from_notification(value: &Value) -> Option<String> {
    if let Some(label) = plan_activity_label(value) {
        return Some(label);
    }
    let item = value.pointer("/params/item")?;
    let item_type = item.get("type").and_then(Value::as_str)?;
    match item_type {
        "commandExecution" | "command_execution" => {
            let command = item.get("command").and_then(Value::as_str)?.trim();
            if command.is_empty() {
                Some("Running command".to_string())
            } else {
                Some(format!("Running {}", command))
            }
        }
        "fileRead" | "file_read" => {
            let path = item
                .get("path")
                .and_then(Value::as_str)
                .or_else(|| item.get("filePath").and_then(Value::as_str))
                .unwrap_or("file");
            Some(format!("Reading {}", path))
        }
        "fileChange" | "file_change" => Some("Applying file changes".to_string()),
        "reasoning" => reasoning_activity_summary(item).or_else(|| Some("Thinking".to_string())),
        "plan" => Some("Planning".to_string()),
        "agentMessage" | "agent_message" => {
            let phase = item.get("phase").and_then(Value::as_str).unwrap_or("");
            if phase == "commentary" {
                Some("Thinking".to_string())
            } else {
                Some("Drafting response".to_string())
            }
        }
        "toolCall" | "tool_call" | "toolResult" | "tool_result" => {
            let tool_name = item
                .get("toolName")
                .and_then(Value::as_str)
                .or_else(|| item.get("name").and_then(Value::as_str))
                .unwrap_or("tool");
            Some(format!("Running {}", tool_name))
        }
        "mcpToolCall" | "mcp_tool_call" => {
            let tool_name = item.get("tool").and_then(Value::as_str).unwrap_or("tool");
            Some(format!("Running {}", tool_name))
        }
        _ => None,
    }
}

fn reasoning_activity_summary(item: &Value) -> Option<String> {
    summary_to_label(item.get("summary")?)
}

fn has_reasoning_summary(value: &Value, item: Option<&Value>) -> bool {
    item.and_then(|entry| entry.get("summary"))
        .and_then(summary_to_label)
        .is_some()
        || value
            .pointer("/params/summary")
            .and_then(summary_to_label)
            .is_some()
}

fn summary_to_label(summary: &Value) -> Option<String> {
    if let Some(text) = summary.as_str() {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    if let Some(array) = summary.as_array() {
        let joined = array
            .iter()
            .filter_map(summary_entry_text)
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" · ");
        if !joined.is_empty() {
            return Some(joined);
        }
    }

    if let Some(text) = summary.get("text").and_then(Value::as_str) {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    None
}

fn summary_entry_text(entry: &Value) -> Option<&str> {
    entry
        .as_str()
        .or_else(|| entry.get("text").and_then(Value::as_str))
}

fn plan_activity_label(value: &Value) -> Option<String> {
    let explanation = value
        .pointer("/params/explanation")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(explanation) = explanation {
        return Some(explanation.to_string());
    }
    let first_step = value
        .pointer("/params/plan")
        .and_then(Value::as_array)
        .and_then(|steps| steps.first())
        .and_then(|step| step.get("step"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(format!("Planning {}", first_step))
}

fn build_plan_event(value: &Value) -> GuiPlanEvent {
    let explanation = value
        .pointer("/params/explanation")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string);
    let plan = value
        .pointer("/params/plan")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    Some(GuiPlanStep {
                        step: item.get("step")?.as_str()?.to_string(),
                        status: item
                            .get("status")
                            .and_then(Value::as_str)
                            .unwrap_or("pending")
                            .to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    GuiPlanEvent { explanation, plan }
}

fn parse_token_usage_event(value: &Value) -> Option<GuiTokenUsageEvent> {
    let usage = value.pointer("/params/tokenUsage")?;
    Some(GuiTokenUsageEvent {
        total_tokens: usage.pointer("/total/totalTokens")?.as_u64()?,
        input_tokens: usage.pointer("/total/inputTokens")?.as_u64()?,
        cached_input_tokens: usage.pointer("/total/cachedInputTokens")?.as_u64()?,
        output_tokens: usage.pointer("/total/outputTokens")?.as_u64()?,
        reasoning_output_tokens: usage.pointer("/total/reasoningOutputTokens")?.as_u64()?,
        last_total_tokens: usage.pointer("/last/totalTokens")?.as_u64()?,
        last_input_tokens: usage.pointer("/last/inputTokens")?.as_u64()?,
        last_cached_input_tokens: usage.pointer("/last/cachedInputTokens")?.as_u64()?,
        last_output_tokens: usage.pointer("/last/outputTokens")?.as_u64()?,
        last_reasoning_output_tokens: usage.pointer("/last/reasoningOutputTokens")?.as_u64()?,
        model_context_window: usage.pointer("/modelContextWindow").and_then(Value::as_u64),
    })
}
