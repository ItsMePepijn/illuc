use crate::commands::CommandResult;
use crate::error::TaskError;
use crate::features::tasks::agents::{AgentCallbacks, AgentRuntime};
use crate::features::tasks::events::emit_status;
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
