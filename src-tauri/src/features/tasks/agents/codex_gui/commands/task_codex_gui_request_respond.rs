use crate::commands::CommandResult;
use crate::features::tasks::agents::codex_gui::commands::task_codex_gui_common::require_running_codex_gui_record_mut;
use crate::features::tasks::TaskManager;
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub task_id: Uuid,
    pub request_id: String,
    pub response: Value,
}

pub type Response = ();

#[tauri::command]
pub async fn task_codex_gui_request_respond(
    manager: tauri::State<'_, TaskManager>,
    req: Request,
) -> CommandResult<Response> {
    let mut tasks = manager.inner.tasks.write();
    let record = require_running_codex_gui_record_mut(&mut tasks, req.task_id)?;
    record
        .agent
        .respond_ui_request(req.request_id, req.response)
        .map_err(|error| error.to_string())
}
