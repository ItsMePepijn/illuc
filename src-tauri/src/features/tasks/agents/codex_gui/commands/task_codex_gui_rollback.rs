use crate::commands::CommandResult;
use crate::features::tasks::agents::codex_gui::commands::task_codex_gui_common::require_running_codex_gui_record_mut;
use crate::features::tasks::TaskManager;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub task_id: Uuid,
    pub num_turns: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub events: Vec<RollbackEvent>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackEvent {
    pub message_id: String,
    pub role: String,
    pub content: String,
    pub is_delta: bool,
    pub is_final: bool,
}

#[tauri::command]
pub async fn task_codex_gui_rollback(
    manager: tauri::State<'_, TaskManager>,
    req: Request,
) -> CommandResult<Response> {
    let mut tasks = manager.inner.tasks.write();
    let record = require_running_codex_gui_record_mut(&mut tasks, req.task_id)?;
    let events = record
        .agent
        .rollback_thread(req.num_turns.unwrap_or(1))
        .map_err(|error| error.to_string())?;
    Ok(Response {
        events: events
            .into_iter()
            .map(|event| RollbackEvent {
                message_id: event.message_id,
                role: event.role.as_str().to_string(),
                content: event.content,
                is_delta: event.is_delta,
                is_final: event.is_final,
            })
            .collect(),
    })
}
