use crate::features::tasks::agents::agent_gui::types::{GuiMessageEvent, GuiMessagePresentation};
use crate::features::tasks::{TaskSummary, TerminalKind};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

pub fn emit_status(app: &AppHandle, summary: &TaskSummary) {
    if let Err(error) = app.emit("task_status_changed", summary) {
        log::warn!("failed to emit task_status_changed event: {error}");
    }
}

pub fn emit_terminal_output(app: &AppHandle, task_id: Uuid, data: String, kind: TerminalKind) {
    let payload = TerminalOutputPayload {
        task_id,
        data,
        kind,
    };
    if let Err(error) = app.emit("task_terminal_output", payload) {
        log::warn!("failed to emit task_terminal_output event: {error}");
    }
}

pub fn emit_terminal_exit(app: &AppHandle, task_id: Uuid, exit_code: i32, kind: TerminalKind) {
    let payload = TerminalExitPayload {
        task_id,
        exit_code,
        kind,
    };
    if let Err(error) = app.emit("task_terminal_exit", payload) {
        log::warn!("failed to emit task_terminal_exit event: {error}");
    }
}

pub fn emit_diff_changed(app: &AppHandle, task_id: Uuid) {
    let payload = DiffChangedPayload { task_id };
    if let Err(error) = app.emit("task_diff_changed", payload) {
        log::warn!("failed to emit task_diff_changed event: {error}");
    }
}

pub fn emit_review_changed(app: &AppHandle, task_id: Uuid) {
    let payload = ReviewChangedPayload { task_id };
    if let Err(error) = app.emit("task_review_changed", payload) {
        log::warn!("failed to emit task_review_changed event: {error}");
    }
}

pub fn emit_agent_gui_message(
    app: &AppHandle,
    task_id: Uuid,
    message_id: String,
    role: &'static str,
    content: String,
    presentation: GuiMessagePresentation,
    is_delta: bool,
    is_final: bool,
) {
    let payload = GuiAgentMessagePayload {
        task_id,
        message_id,
        role,
        content,
        presentation,
        is_delta,
        is_final,
    };
    if let Err(error) = app.emit("task_agent_gui_message", payload) {
        log::warn!("failed to emit task_agent_gui_message event: {error}");
    }
}

pub fn emit_agent_gui_history(app: &AppHandle, task_id: Uuid, events: Vec<GuiMessageEvent>) {
    let payload = GuiAgentHistoryPayload {
        task_id,
        events: events
            .into_iter()
            .map(|event| GuiAgentHistoryMessagePayload {
                message_id: event.message_id,
                role: event.role.as_str(),
                content: event.content,
                presentation: event.presentation,
                is_delta: event.is_delta,
                is_final: event.is_final,
            })
            .collect(),
    };
    if let Err(error) = app.emit("task_agent_gui_history", payload) {
        log::warn!("failed to emit task_agent_gui_history event: {error}");
    }
}

pub fn emit_agent_gui_hydrated(app: &AppHandle, task_id: Uuid) {
    let payload = GuiAgentHydratedPayload { task_id };
    if let Err(error) = app.emit("task_agent_gui_hydrated", payload) {
        log::warn!("failed to emit task_agent_gui_hydrated event: {error}");
    }
}

pub fn emit_agent_gui_activity(
    app: &AppHandle,
    task_id: Uuid,
    label: Option<String>,
    started_at: Option<chrono::DateTime<chrono::Utc>>,
) {
    let payload = GuiAgentActivityPayload {
        task_id,
        label,
        started_at,
    };
    if let Err(error) = app.emit("task_agent_gui_activity", payload) {
        log::warn!("failed to emit task_agent_gui_activity event: {error}");
    }
}

pub fn emit_agent_gui_plan(
    app: &AppHandle,
    task_id: Uuid,
    explanation: Option<String>,
    plan: Vec<GuiAgentPlanStepPayload>,
) {
    let payload = GuiAgentPlanPayload {
        task_id,
        explanation,
        plan,
    };
    if let Err(error) = app.emit("task_agent_gui_plan", payload) {
        log::warn!("failed to emit task_agent_gui_plan event: {error}");
    }
}

pub fn emit_agent_gui_token_usage(
    app: &AppHandle,
    task_id: Uuid,
    usage: GuiAgentTokenUsagePayload,
) {
    let payload = GuiAgentTokenUsageEventPayload { task_id, usage };
    if let Err(error) = app.emit("task_agent_gui_token_usage", payload) {
        log::warn!("failed to emit task_agent_gui_token_usage event: {error}");
    }
}

pub fn emit_agent_gui_request(app: &AppHandle, payload: GuiAgentRequestPayload) {
    if let Err(error) = app.emit("task_agent_gui_request", payload) {
        log::warn!("failed to emit task_agent_gui_request event: {error}");
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TerminalOutputPayload {
    task_id: Uuid,
    data: String,
    kind: TerminalKind,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TerminalExitPayload {
    task_id: Uuid,
    exit_code: i32,
    kind: TerminalKind,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DiffChangedPayload {
    task_id: Uuid,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ReviewChangedPayload {
    task_id: Uuid,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct GuiAgentMessagePayload {
    task_id: Uuid,
    message_id: String,
    role: &'static str,
    content: String,
    presentation: GuiMessagePresentation,
    is_delta: bool,
    is_final: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct GuiAgentHistoryPayload {
    task_id: Uuid,
    events: Vec<GuiAgentHistoryMessagePayload>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct GuiAgentHistoryMessagePayload {
    message_id: String,
    role: &'static str,
    content: String,
    presentation: GuiMessagePresentation,
    is_delta: bool,
    is_final: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct GuiAgentHydratedPayload {
    task_id: Uuid,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct GuiAgentActivityPayload {
    task_id: Uuid,
    label: Option<String>,
    started_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GuiAgentPlanStepPayload {
    pub step: String,
    pub status: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct GuiAgentPlanPayload {
    task_id: Uuid,
    explanation: Option<String>,
    plan: Vec<GuiAgentPlanStepPayload>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GuiAgentTokenUsagePayload {
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_output_tokens: u64,
    pub last_total_tokens: u64,
    pub last_input_tokens: u64,
    pub last_cached_input_tokens: u64,
    pub last_output_tokens: u64,
    pub last_reasoning_output_tokens: u64,
    pub model_context_window: Option<u64>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct GuiAgentTokenUsageEventPayload {
    task_id: Uuid,
    usage: GuiAgentTokenUsagePayload,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GuiAgentQuestionOptionPayload {
    pub label: String,
    pub description: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GuiAgentQuestionPayload {
    pub id: String,
    pub header: String,
    pub question: String,
    pub is_other: bool,
    pub is_secret: bool,
    pub options: Vec<GuiAgentQuestionOptionPayload>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GuiAgentRequestPayload {
    pub task_id: Uuid,
    pub request_id: Option<String>,
    pub kind: String,
    pub item_id: Option<String>,
    pub approval_id: Option<String>,
    pub command: Option<String>,
    pub cwd: Option<String>,
    pub reason: Option<String>,
    pub network_host: Option<String>,
    pub network_protocol: Option<String>,
    pub additional_read_roots: Vec<String>,
    pub additional_write_roots: Vec<String>,
    pub additional_network: bool,
    pub available_decisions: Vec<String>,
    pub proposed_exec_policy: Vec<String>,
    pub proposed_network_policy: Vec<String>,
    pub grant_root: Option<String>,
    pub questions: Vec<GuiAgentQuestionPayload>,
}
