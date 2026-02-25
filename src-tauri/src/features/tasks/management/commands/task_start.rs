use crate::commands::CommandResult;
use crate::error::TaskError;
use crate::features::tasks::agents::{AgentCallbacks, AgentRuntime};
use crate::features::tasks::events::emit_status;
use crate::features::tasks::events::{
    emit_codex_gui_activity, emit_codex_gui_hydrated, emit_codex_gui_message, emit_codex_gui_plan,
    emit_codex_gui_request, emit_codex_gui_token_usage, CodexGuiPlanStepPayload,
    CodexGuiQuestionOptionPayload, CodexGuiQuestionPayload, CodexGuiRequestPayload,
    CodexGuiTokenUsagePayload,
};
use crate::features::tasks::{
    agent_label, build_agent, AgentKind, TaskManager, TaskRuntime, TaskStatus, TaskSummary,
    DEFAULT_PTY_COLS, DEFAULT_PTY_ROWS, DEFAULT_SCREEN_COLS, DEFAULT_SCREEN_ROWS,
};
use anyhow::Context;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub task_id: Uuid,
    pub cols: Option<u16>,
    pub rows: Option<u16>,
    pub agent: Option<AgentKind>,
}

pub type Response = TaskSummary;

#[tauri::command]
pub async fn task_start(
    manager: tauri::State<'_, TaskManager>,
    app_handle: tauri::AppHandle,
    req: Request,
) -> CommandResult<Response> {
    let Request {
        task_id,
        cols,
        rows,
        agent,
    } = req;
    let requested_rows = rows.filter(|value| *value > 0);
    let requested_cols = cols.filter(|value| *value > 0);
    let screen_rows = requested_rows
        .map(|value| value as usize)
        .unwrap_or(DEFAULT_SCREEN_ROWS);
    let screen_cols = requested_cols
        .map(|value| value as usize)
        .unwrap_or(DEFAULT_SCREEN_COLS);
    let pty_rows = requested_rows.unwrap_or(DEFAULT_PTY_ROWS);
    let pty_cols = requested_cols.unwrap_or(DEFAULT_PTY_COLS);
    {
        let tasks = manager.inner.tasks.read();
        let record = tasks
            .get(&task_id)
            .ok_or_else(|| TaskError::NotFound.to_string())?;
        if record.runtime.is_some() {
            return Err(TaskError::AlreadyRunning.to_string());
        }
    }

    let (worktree_path, title) = {
        let tasks = manager.inner.tasks.read();
        let record = tasks
            .get(&task_id)
            .ok_or_else(|| TaskError::NotFound.to_string())?;
        (
            PathBuf::from(&record.summary.worktree_path),
            record.summary.title.clone(),
        )
    };

    let status_manager = manager.inner().clone();
    let status_app = app_handle.clone();
    let output_manager = manager.inner().clone();
    let output_app = app_handle.clone();
    let exit_manager = manager.inner().clone();
    let exit_app = app_handle.clone();
    let gui_message_app = app_handle.clone();
    let gui_activity_app = app_handle.clone();
    let gui_plan_app = app_handle.clone();
    let gui_token_usage_app = app_handle.clone();
    let gui_request_app = app_handle.clone();
    let gui_hydrated_app = app_handle.clone();
    let callbacks = AgentCallbacks {
        on_output: Arc::new(move |chunk: String| {
            output_manager.handle_agent_output(task_id, chunk, &output_app);
        }),
        on_status: Arc::new(move |status: TaskStatus| {
            status_manager.handle_agent_status(task_id, status, &status_app);
        }),
        on_exit: Arc::new(move |exit_code: i32| {
            exit_manager.handle_agent_exit(task_id, exit_code, &exit_app);
        }),
        on_gui_event: Arc::new(move |event| {
            emit_codex_gui_message(
                &gui_message_app,
                task_id,
                event.message_id,
                event.role.as_str(),
                event.content,
                event.is_delta,
                event.is_final,
            );
        }),
        on_gui_activity: Arc::new(move |event| {
            emit_codex_gui_activity(&gui_activity_app, task_id, event.label, event.started_at);
        }),
        on_gui_plan: Arc::new(move |event| {
            emit_codex_gui_plan(
                &gui_plan_app,
                task_id,
                event.explanation,
                event
                    .plan
                    .into_iter()
                    .map(|step| CodexGuiPlanStepPayload {
                        step: step.step,
                        status: step.status,
                    })
                    .collect(),
            );
        }),
        on_gui_token_usage: Arc::new(move |event| {
            emit_codex_gui_token_usage(
                &gui_token_usage_app,
                task_id,
                CodexGuiTokenUsagePayload {
                    total_tokens: event.total_tokens,
                    input_tokens: event.input_tokens,
                    cached_input_tokens: event.cached_input_tokens,
                    output_tokens: event.output_tokens,
                    reasoning_output_tokens: event.reasoning_output_tokens,
                    last_total_tokens: event.last_total_tokens,
                    last_input_tokens: event.last_input_tokens,
                    last_cached_input_tokens: event.last_cached_input_tokens,
                    last_output_tokens: event.last_output_tokens,
                    last_reasoning_output_tokens: event.last_reasoning_output_tokens,
                    model_context_window: event.model_context_window,
                },
            );
        }),
        on_gui_request: Arc::new(move |event| {
            let payload = match event {
                crate::features::tasks::agents::GuiRequestEvent::Cleared => {
                    CodexGuiRequestPayload {
                        task_id,
                        request_id: None,
                        kind: "none".to_string(),
                        item_id: None,
                        approval_id: None,
                        command: None,
                        cwd: None,
                        reason: None,
                        network_host: None,
                        network_protocol: None,
                        additional_read_roots: Vec::new(),
                        additional_write_roots: Vec::new(),
                        additional_network: false,
                        available_decisions: Vec::new(),
                        proposed_exec_policy: Vec::new(),
                        proposed_network_policy: Vec::new(),
                        grant_root: None,
                        questions: Vec::new(),
                    }
                }
                crate::features::tasks::agents::GuiRequestEvent::CommandApproval {
                    request_id,
                    item_id,
                    approval_id,
                    command,
                    cwd,
                    reason,
                    network_host,
                    network_protocol,
                    additional_read_roots,
                    additional_write_roots,
                    additional_network,
                    available_decisions,
                    proposed_exec_policy,
                    proposed_network_policy,
                } => CodexGuiRequestPayload {
                    task_id,
                    request_id: Some(request_id),
                    kind: "commandApproval".to_string(),
                    item_id: Some(item_id),
                    approval_id,
                    command,
                    cwd,
                    reason,
                    network_host,
                    network_protocol,
                    additional_read_roots,
                    additional_write_roots,
                    additional_network,
                    available_decisions,
                    proposed_exec_policy,
                    proposed_network_policy,
                    grant_root: None,
                    questions: Vec::new(),
                },
                crate::features::tasks::agents::GuiRequestEvent::FileChangeApproval {
                    request_id,
                    item_id,
                    reason,
                    grant_root,
                    available_decisions,
                } => CodexGuiRequestPayload {
                    task_id,
                    request_id: Some(request_id),
                    kind: "fileChangeApproval".to_string(),
                    item_id: Some(item_id),
                    approval_id: None,
                    command: None,
                    cwd: None,
                    reason,
                    network_host: None,
                    network_protocol: None,
                    additional_read_roots: Vec::new(),
                    additional_write_roots: Vec::new(),
                    additional_network: false,
                    available_decisions,
                    proposed_exec_policy: Vec::new(),
                    proposed_network_policy: Vec::new(),
                    grant_root,
                    questions: Vec::new(),
                },
                crate::features::tasks::agents::GuiRequestEvent::UserInput {
                    request_id,
                    item_id,
                    questions,
                } => CodexGuiRequestPayload {
                    task_id,
                    request_id: Some(request_id),
                    kind: "userInput".to_string(),
                    item_id: Some(item_id),
                    approval_id: None,
                    command: None,
                    cwd: None,
                    reason: None,
                    network_host: None,
                    network_protocol: None,
                    additional_read_roots: Vec::new(),
                    additional_write_roots: Vec::new(),
                    additional_network: false,
                    available_decisions: Vec::new(),
                    proposed_exec_policy: Vec::new(),
                    proposed_network_policy: Vec::new(),
                    grant_root: None,
                    questions: questions
                        .into_iter()
                        .map(|question| CodexGuiQuestionPayload {
                            id: question.id,
                            header: question.header,
                            question: question.question,
                            is_other: question.is_other,
                            is_secret: question.is_secret,
                            options: question
                                .options
                                .into_iter()
                                .map(|option| CodexGuiQuestionOptionPayload {
                                    label: option.label,
                                    description: option.description,
                                })
                                .collect(),
                        })
                        .collect(),
                },
            };
            emit_codex_gui_request(&gui_request_app, payload);
        }),
        on_gui_hydrated: Arc::new(move || {
            emit_codex_gui_hydrated(&gui_hydrated_app, task_id);
        }),
    };

    let agent_runtime = {
        let mut tasks = manager.inner.tasks.write();
        let record = tasks
            .get_mut(&task_id)
            .ok_or_else(|| TaskError::NotFound.to_string())?;
        if let Some(requested_agent) = agent {
            record.agent_kind = requested_agent;
            record.agent = build_agent(requested_agent);
        }
        record.summary.agent_kind = record.agent_kind;
        let label = agent_label(record.agent_kind);
        record.agent.reset(screen_rows, screen_cols);
        record
            .agent
            .start(&worktree_path, callbacks, pty_rows, pty_cols)
            .with_context(|| format!("failed to start {} for task {}", label, title))
            .map_err(|err| err.to_string())?
    };

    let AgentRuntime {
        child,
        writer,
        master,
    } = agent_runtime;

    {
        let mut tasks = manager.inner.tasks.write();
        let record = tasks
            .get_mut(&task_id)
            .ok_or_else(|| TaskError::NotFound.to_string())?;
        record.summary.status = TaskStatus::Idle;
        record.summary.started_at = Some(chrono::Utc::now());
        record.summary.exit_code = None;
        record.runtime = Some(TaskRuntime {
            child: child.clone(),
            writer: writer.clone(),
            master: master.clone(),
        });
        emit_status(&app_handle, &record.summary);
    }

    let tasks = manager.inner.tasks.read();
    let record = tasks
        .get(&task_id)
        .ok_or_else(|| TaskError::NotFound.to_string())?;
    Ok(record.summary.clone())
}
