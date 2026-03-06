use super::super::message_parsing::{collect_history_events, fallback_message_id};
use super::super::{rpc, CodexGuiAgentState};
use crate::features::tasks::agents::codex_gui::types::{
    GuiMessageEvent, GuiMessagePresentation, GuiMessagePresentationKind, GuiMessageRole,
    GuiMessageTextFormat, GuiPlanEvent,
};
use crate::features::tasks::agents::AgentCallbacks;
use parking_lot::Mutex;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

pub(super) fn handle_response(state: &Arc<Mutex<CodexGuiAgentState>>, value: &Value) {
    let response_id = value.get("id").and_then(Value::as_u64);
    let mut system_message: Option<String> = None;
    let mut callbacks: Option<AgentCallbacks> = None;
    let mut history_callbacks: Option<AgentCallbacks> = None;
    let mut history_events: Vec<GuiMessageEvent> = Vec::new();
    let mut hydrated_callbacks: Option<AgentCallbacks> = None;
    let mut clear_plan_callbacks: Option<AgentCallbacks> = None;

    {
        let mut state = state.lock();

        if let Some(id) = response_id {
            if state.thread_list_request_id == Some(id) {
                state.thread_list_request_id = None;
                if let Some(thread_id) = parse_newest_thread_id(value) {
                    if let Err(error) = rpc::send_thread_resume_request(&mut state, &thread_id) {
                        log::warn!("failed to send thread/resume request: {}", error);
                        if let Err(start_error) = rpc::send_thread_start_request(&mut state) {
                            log::warn!(
                                "failed to fallback to thread/start after resume request error: {}",
                                start_error
                            );
                        }
                        hydrated_callbacks = state.callbacks.clone();
                    }
                } else if let Err(error) = rpc::send_thread_start_request(&mut state) {
                    log::warn!("failed to send thread/start request: {}", error);
                    hydrated_callbacks = state.callbacks.clone();
                } else {
                    hydrated_callbacks = state.callbacks.clone();
                }
            } else {
                if state.thread_resume_request_id == Some(id) {
                    state.thread_resume_request_id = None;
                    if has_error(value) {
                        if let Err(error) = rpc::send_thread_start_request(&mut state) {
                            log::warn!(
                                "failed to fallback to thread/start after resume error: {}",
                                error
                            );
                            callbacks = state.callbacks.clone();
                            system_message = Some(
                                "Codex GUI resume failed and fallback start also failed."
                                    .to_string(),
                            );
                        }
                    } else {
                        history_events = collect_history_events(value);
                        if !history_events.is_empty() {
                            history_callbacks = state.callbacks.clone();
                        } else {
                            log_resume_history_miss(value);
                        }
                    }
                    hydrated_callbacks = state.callbacks.clone();
                }

                if state.model_list_request_id == Some(id) {
                    state.model_list_request_id = None;
                    if let Some(parsed_models) = parse_model_list(value) {
                        if !parsed_models.capabilities.is_empty() {
                            state.available_models = parsed_models
                                .capabilities
                                .iter()
                                .map(|item| item.model.clone())
                                .collect();
                            state.available_model_capabilities = parsed_models.capabilities;
                        }
                        if state.model.is_none() {
                            state.model = parsed_models.default_model;
                        }
                        if state.reasoning_effort.is_none() {
                            state.reasoning_effort = parsed_models.default_reasoning_effort;
                        }
                    }
                }

                if state.rate_limits_request_id == Some(id) {
                    state.rate_limits_request_id = None;
                    if !has_error(value) {
                        state.latest_rate_limits = parse_rate_limits_result(value);
                    }
                }

                if state.rollback_request_id == Some(id) {
                    state.rollback_request_id = None;
                    if !has_error(value) {
                        state.rollback_history_events = collect_history_events(value);
                    } else {
                        state.rollback_history_events.clear();
                    }
                    clear_plan_callbacks = state.callbacks.clone();
                }
            }
        }

        let thread_id = parse_thread_id(value);

        if let Some(thread_id) = thread_id {
            state.thread_id = Some(thread_id);
            if let Err(error) = rpc::flush_pending_messages(&mut state) {
                log::warn!("failed to flush queued Codex GUI turns: {}", error);
            }
        }

        if let Some(model) = value
            .pointer("/result/model")
            .and_then(Value::as_str)
            .map(str::to_string)
            .filter(|value| !value.is_empty())
        {
            state.model = Some(model);
        }
        if let Some(effort) = value
            .pointer("/result/reasoningEffort")
            .and_then(Value::as_str)
            .map(str::to_string)
            .filter(|value| !value.is_empty())
        {
            state.reasoning_effort = Some(effort);
        }

        if value.get("error").is_some() {
            let message = value
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("Codex GUI request failed")
                .to_string();
            callbacks = state.callbacks.clone();
            system_message = Some(message);
        }
    }

    if let Some(message) = system_message {
        emit_system_message(callbacks, message);
    }
    if let Some(callbacks) = history_callbacks {
        (callbacks.on_gui_history)(history_events);
    }
    if let Some(callbacks) = hydrated_callbacks {
        (callbacks.on_gui_hydrated)();
    }
    if let Some(callbacks) = clear_plan_callbacks {
        (callbacks.on_gui_plan)(GuiPlanEvent {
            explanation: None,
            plan: Vec::new(),
        });
    }
}

fn emit_system_message(callbacks: Option<AgentCallbacks>, message: String) {
    if let Some(callbacks) = callbacks {
        let text = message.clone();
        (callbacks.on_gui_event)(GuiMessageEvent {
            message_id: fallback_message_id(),
            role: GuiMessageRole::System,
            content: message,
            presentation: GuiMessagePresentation {
                kind: GuiMessagePresentationKind::Standard,
                text: Some(text),
                text_format: Some(GuiMessageTextFormat::Markdown),
                tool_rows: Vec::new(),
                tool_status_label: None,
                is_tool_running: false,
            },
            is_delta: false,
            is_final: true,
        });
    }
}

#[derive(Debug, Deserialize)]
struct ThreadListResponse {
    #[serde(default)]
    result: ThreadListResult,
}

#[derive(Debug, Deserialize, Default)]
struct ThreadListResult {
    #[serde(default)]
    data: Vec<ThreadSummary>,
}

#[derive(Debug, Deserialize)]
struct ThreadSummary {
    id: String,
    #[serde(rename = "updatedAt")]
    #[serde(alias = "updated_at")]
    updated_at: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ThreadResultResponse {
    #[serde(default)]
    result: ThreadResult,
}

#[derive(Debug, Deserialize, Default)]
struct ThreadResult {
    #[serde(rename = "threadId")]
    thread_id: Option<String>,
    thread: Option<ThreadRef>,
}

#[derive(Debug, Deserialize)]
struct ThreadRef {
    id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResponseEnvelope {
    result: Option<Value>,
    error: Option<Value>,
}

fn has_error(value: &Value) -> bool {
    serde_json::from_value::<ResponseEnvelope>(value.clone())
        .map(|parsed| parsed.error.is_some())
        .unwrap_or(false)
}

fn parse_rate_limits_result(value: &Value) -> Option<Value> {
    serde_json::from_value::<ResponseEnvelope>(value.clone())
        .ok()
        .and_then(|parsed| parsed.result)
}

fn parse_thread_id(value: &Value) -> Option<String> {
    serde_json::from_value::<ThreadResultResponse>(value.clone())
        .ok()
        .and_then(|parsed| {
            parsed
                .result
                .thread_id
                .or_else(|| parsed.result.thread.and_then(|thread| thread.id))
        })
}

fn parse_newest_thread_id(value: &Value) -> Option<String> {
    serde_json::from_value::<ThreadListResponse>(value.clone())
        .ok()
        .and_then(|parsed| {
            parsed
                .result
                .data
                .into_iter()
                .max_by_key(|thread| thread.updated_at.unwrap_or(0))
                .map(|thread| thread.id)
        })
}

fn log_resume_history_miss(value: &Value) {
    let result = value.get("result").unwrap_or(&Value::Null);
    let summary = summarize_resume_result(result);
    log::warn!("codex gui resume returned no history events; shape: {}", summary);
    match serde_json::to_string_pretty(value) {
        Ok(serialized) => {
            log::warn!("codex gui thread/resume raw payload:\n{}", serialized);
        }
        Err(error) => {
            log::warn!(
                "failed to serialize codex gui thread/resume payload for debugging: {}",
                error
            );
        }
    }
}

fn summarize_resume_result(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let keys = map.keys().cloned().collect::<Vec<_>>().join(", ");
            let turns_count = map
                .get("turns")
                .and_then(Value::as_array)
                .map(|items| items.len())
                .or_else(|| {
                    map.get("thread")
                        .and_then(|thread| thread.get("turns"))
                        .and_then(Value::as_array)
                        .map(|items| items.len())
                });
            let data_count = map
                .get("data")
                .and_then(Value::as_array)
                .map(|items| items.len());
            format!(
                "object keys=[{}] turns={} data={}",
                keys,
                turns_count
                    .map(|count| count.to_string())
                    .unwrap_or_else(|| "none".to_string()),
                data_count
                    .map(|count| count.to_string())
                    .unwrap_or_else(|| "none".to_string())
            )
        }
        Value::Array(items) => format!("array len={}", items.len()),
        Value::Null => "null".to_string(),
        _ => value.to_string(),
    }
}

struct ParsedModelList {
    capabilities: Vec<crate::features::tasks::agents::AgentModelCapability>,
    default_model: Option<String>,
    default_reasoning_effort: Option<String>,
}

fn parse_model_list(value: &Value) -> Option<ParsedModelList> {
    #[derive(Debug, Deserialize)]
    struct ModelListResponse {
        #[serde(default)]
        result: ModelListResult,
    }
    #[derive(Debug, Deserialize, Default)]
    struct ModelListResult {
        #[serde(default)]
        data: Vec<ModelItem>,
    }
    #[derive(Debug, Deserialize)]
    struct ModelItem {
        id: Option<String>,
        model: Option<String>,
        #[serde(rename = "defaultReasoningEffort")]
        default_reasoning_effort: Option<String>,
        #[serde(rename = "isDefault")]
        is_default: Option<bool>,
        #[serde(rename = "supportedReasoningEfforts")]
        supported_reasoning_efforts: Option<Vec<ModelEffort>>,
    }
    #[derive(Debug, Deserialize)]
    struct ModelEffort {
        #[serde(rename = "reasoningEffort")]
        reasoning_effort: Option<String>,
    }

    let parsed = serde_json::from_value::<ModelListResponse>(value.clone()).ok()?;
    let mut default_model = None;
    let mut default_reasoning_effort = None;
    let capabilities = parsed
        .result
        .data
        .into_iter()
        .filter_map(|item| {
            let model = item.model.or(item.id)?.trim().to_string();
            if model.is_empty() {
                return None;
            }
            if item.is_default.unwrap_or(false) && default_model.is_none() {
                default_model = Some(model.clone());
                default_reasoning_effort = item.default_reasoning_effort.clone();
            }
            Some(crate::features::tasks::agents::AgentModelCapability {
                model,
                reasoning_efforts: item
                    .supported_reasoning_efforts
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|effort| effort.reasoning_effort)
                    .collect(),
            })
        })
        .collect();

    Some(ParsedModelList {
        capabilities,
        default_model,
        default_reasoning_effort,
    })
}
