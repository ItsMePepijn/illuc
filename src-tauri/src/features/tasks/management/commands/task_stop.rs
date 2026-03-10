use crate::commands::CommandResult;
use crate::error::TaskError;
use crate::features::tasks::events::emit_status;
use crate::features::tasks::{TaskManager, TaskStatus, TaskSummary};
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
    {
        let mut tasks = manager.inner.tasks.write();
        let record = tasks
            .get_mut(&task_id)
            .ok_or_else(|| TaskError::NotFound.to_string())?;
        if !record.agent.is_running() {
            return Ok(record.summary.clone());
        }
        if let Err(err) = record.agent.stop() {
            warn!("failed to stop task process for {}: {}", task_id, err);
        }
    }

    let mut tasks = manager.inner.tasks.write();
    let record = tasks
        .get_mut(&task_id)
        .ok_or_else(|| TaskError::NotFound.to_string())?;
    record.summary.status = TaskStatus::Stopped;
    emit_status(&app_handle, &record.summary);
    Ok(record.summary.clone())
}
