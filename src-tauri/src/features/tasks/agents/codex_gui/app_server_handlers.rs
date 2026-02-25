use super::message_parsing::{
    collect_history_events, extract_content, extract_message_id, extract_role, fallback_message_id,
};
use super::{rpc, CodexGuiAgentState};
use crate::features::tasks::agents::{
    AgentCallbacks, GuiActivityEvent, GuiMessageEvent, GuiPlanEvent, GuiPlanStep, GuiRequestEvent,
    GuiRequestQuestion, GuiRequestQuestionOption, GuiTokenUsageEvent,
};
use crate::features::tasks::TaskStatus;
use anyhow::Result;
use chrono::Utc;
use parking_lot::Mutex;
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader};
use std::process::ChildStderr;
use std::sync::mpsc::sync_channel;
use std::sync::Arc;

pub(super) fn spawn_stderr_logger(stderr: Option<ChildStderr>) {
    let Some(stderr) = stderr else {
        return;
    };
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(std::result::Result::ok) {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                log::warn!("codex app-server stderr: {}", trimmed);
            }
        }
    });
}

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
        for event in history_events {
            (callbacks.on_gui_event)(event);
        }
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

pub(super) fn handle_server_request(
    state: &Arc<Mutex<CodexGuiAgentState>>,
    request: &Value,
    request_id: &Value,
    method: &str,
) -> Result<()> {
    let Some(request) = parse_request_event(request, request_id, method) else {
        let payload = json!({
            "id": request_id,
            "result": {}
        });
        let mut state = state.lock();
        return rpc::send_rpc(&mut state, payload);
    };

    let (callbacks, response) = {
        let (sender, receiver) = sync_channel(1);
        let callbacks = {
            let mut guard = state.lock();
            guard
                .pending_server_requests
                .insert(request.request_id.clone(), sender);
            guard.callbacks.clone()
        };
        if let Some(callbacks) = callbacks.clone() {
            (callbacks.on_gui_request)(request.event);
        }
        let response = receiver.recv().unwrap_or_else(|_| json!({}));
        (callbacks, response)
    };

    {
        let mut guard = state.lock();
        rpc::send_rpc(
            &mut guard,
            json!({
                "id": request_id,
                "result": response
            }),
        )?;
    }

    if let Some(callbacks) = callbacks {
        (callbacks.on_gui_request)(GuiRequestEvent::Cleared);
    }

    Ok(())
}

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

    if !method.starts_with("item/") {
        return;
    }

    let params = value.get("params").unwrap_or(&Value::Null);
    let item = params.get("item");

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

    if let Some(callbacks) = state.lock().callbacks.clone() {
        (callbacks.on_gui_event)(GuiMessageEvent {
            message_id,
            role,
            content,
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
        "reasoning" => Some("Thinking".to_string()),
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

struct PendingRequestEvent {
    request_id: String,
    event: GuiRequestEvent,
}

fn parse_request_event(
    request: &Value,
    request_id: &Value,
    method: &str,
) -> Option<PendingRequestEvent> {
    let request_id_text = request_id.to_string();
    let params = request.get("params").cloned().unwrap_or(Value::Null);
    let item_id = params
        .get("itemId")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    let event = match method {
        "item/commandExecution/requestApproval" => GuiRequestEvent::CommandApproval {
            request_id: request_id_text.clone(),
            item_id,
            approval_id: params
                .get("approvalId")
                .and_then(Value::as_str)
                .map(str::to_string),
            command: params
                .get("command")
                .and_then(Value::as_str)
                .map(str::to_string),
            cwd: params
                .get("cwd")
                .and_then(Value::as_str)
                .map(str::to_string),
            reason: params
                .get("reason")
                .and_then(Value::as_str)
                .map(str::to_string),
            network_host: params
                .pointer("/networkApprovalContext/host")
                .and_then(Value::as_str)
                .map(str::to_string),
            network_protocol: params
                .pointer("/networkApprovalContext/protocol")
                .and_then(Value::as_str)
                .map(str::to_string),
            additional_read_roots: params
                .pointer("/additionalPermissions/fileSystem/read")
                .and_then(Value::as_array)
                .map(string_vec)
                .unwrap_or_default(),
            additional_write_roots: params
                .pointer("/additionalPermissions/fileSystem/write")
                .and_then(Value::as_array)
                .map(string_vec)
                .unwrap_or_default(),
            additional_network: params
                .pointer("/additionalPermissions/network")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            available_decisions: params
                .get("availableDecisions")
                .and_then(Value::as_array)
                .map(string_vec)
                .unwrap_or_else(|| vec!["approved".to_string(), "denied".to_string()]),
            proposed_exec_policy: params
                .get("proposedExecpolicyAmendment")
                .and_then(Value::as_array)
                .map(string_vec)
                .unwrap_or_default(),
            proposed_network_policy: params
                .get("proposedNetworkPolicyAmendments")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| {
                            Some(format!(
                                "{} {}",
                                item.get("action")?.as_str()?,
                                item.get("host")?.as_str()?
                            ))
                        })
                        .collect()
                })
                .unwrap_or_default(),
        },
        "item/fileChange/requestApproval" => GuiRequestEvent::FileChangeApproval {
            request_id: request_id_text.clone(),
            item_id,
            reason: params
                .get("reason")
                .and_then(Value::as_str)
                .map(str::to_string),
            grant_root: params
                .get("grantRoot")
                .and_then(Value::as_str)
                .map(str::to_string),
            available_decisions: vec!["approved".to_string(), "denied".to_string()],
        },
        "item/tool/requestUserInput" => GuiRequestEvent::UserInput {
            request_id: request_id_text.clone(),
            item_id,
            questions: params
                .get("questions")
                .and_then(Value::as_array)
                .map(|questions| {
                    questions
                        .iter()
                        .map(|question| GuiRequestQuestion {
                            id: question
                                .get("id")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string(),
                            header: question
                                .get("header")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string(),
                            question: question
                                .get("question")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string(),
                            is_other: question
                                .get("isOther")
                                .and_then(Value::as_bool)
                                .unwrap_or(false),
                            is_secret: question
                                .get("isSecret")
                                .and_then(Value::as_bool)
                                .unwrap_or(false),
                            options: question
                                .get("options")
                                .and_then(Value::as_array)
                                .map(|options| {
                                    options
                                        .iter()
                                        .map(|option| GuiRequestQuestionOption {
                                            label: option
                                                .get("label")
                                                .and_then(Value::as_str)
                                                .unwrap_or_default()
                                                .to_string(),
                                            description: option
                                                .get("description")
                                                .and_then(Value::as_str)
                                                .unwrap_or_default()
                                                .to_string(),
                                        })
                                        .collect()
                                })
                                .unwrap_or_default(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        },
        _ => return None,
    };

    Some(PendingRequestEvent {
        request_id: request_id_text,
        event,
    })
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

fn string_vec(items: &Vec<Value>) -> Vec<String> {
    items
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn emit_system_message(callbacks: Option<AgentCallbacks>, message: String) {
    if let Some(callbacks) = callbacks {
        (callbacks.on_gui_event)(GuiMessageEvent {
            message_id: fallback_message_id(),
            role: crate::features::tasks::agents::GuiMessageRole::System,
            content: message,
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
    let mut capabilities = Vec::new();
    let mut default_model = None;
    let mut default_reasoning_effort = None;
    for model in parsed.result.data {
        let id = model.id.or(model.model)?;
        if model.is_default.unwrap_or(false) {
            default_model = Some(id.clone());
            default_reasoning_effort = model.default_reasoning_effort.clone();
        }
        if capabilities.iter().any(
            |existing: &crate::features::tasks::agents::AgentModelCapability| existing.model == id,
        ) {
            continue;
        }
        let efforts = model
            .supported_reasoning_efforts
            .unwrap_or_default()
            .into_iter()
            .filter_map(|item| item.reasoning_effort)
            .collect::<Vec<_>>();
        capabilities.push(crate::features::tasks::agents::AgentModelCapability {
            model: id,
            reasoning_efforts: efforts,
        });
    }
    Some(ParsedModelList {
        capabilities,
        default_model,
        default_reasoning_effort,
    })
}
