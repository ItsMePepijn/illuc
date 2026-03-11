use crate::commands::CommandResult;
use crate::error::TaskError;
use crate::features::tasks::events::emit_status;
use crate::features::tasks::{
    build_agent, TaskManager, TaskRuntimeState, TaskStatus, TaskSummary,
};
use log::warn;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub task_id: Uuid,
}

pub type Response = TaskSummary;

#[tauri::command]
pub async fn task_stop(
    manager: tauri::State<'_, TaskManager>,
    app_handle: tauri::AppHandle,
    req: Request,
) -> CommandResult<Response> {
    let task_id = req.task_id;
    let mut already_stopped = false;
    {
        let mut tasks = manager.inner.tasks.write();
        let record = tasks
            .get_mut(&task_id)
            .ok_or_else(|| TaskError::NotFound.to_string())?;
        if matches!(record.runtime_state, TaskRuntimeState::Stopped) {
            already_stopped = true;
        } else if matches!(record.runtime_state, TaskRuntimeState::Starting { .. }) {
            let mut agent =
                std::mem::replace(&mut record.agent, build_agent(record.agent_kind));
            record.startup_attempt_id = record.startup_attempt_id.saturating_add(1);
            record.runtime_state = TaskRuntimeState::Stopped;
            if let Err(err) = agent.stop() {
                warn!("failed to stop task process for {}: {}", task_id, err);
            }
        } else {
            if let Err(err) = record.agent.stop() {
                warn!("failed to stop task process for {}: {}", task_id, err);
            }
            record.runtime_state = TaskRuntimeState::Stopped;
        }
    }

    if already_stopped {
        let tasks = manager.inner.tasks.read();
        let record = tasks
            .get(&task_id)
            .ok_or_else(|| TaskError::NotFound.to_string())?;
        return Ok(record.summary.clone());
    }

    let mut tasks = manager.inner.tasks.write();
    let record = tasks
        .get_mut(&task_id)
        .ok_or_else(|| TaskError::NotFound.to_string())?;
    record.summary.status = TaskStatus::Stopped;
    emit_status(&app_handle, &record.summary);
    Ok(record.summary.clone())
}
