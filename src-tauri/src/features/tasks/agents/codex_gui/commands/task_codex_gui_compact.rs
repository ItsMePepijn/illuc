use crate::commands::CommandResult;
use crate::features::tasks::agents::codex_gui::commands::task_codex_gui_common::require_running_codex_gui_record_mut;
use crate::features::tasks::TaskManager;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub task_id: Uuid,
}

pub type Response = ();

#[tauri::command]
pub async fn task_codex_gui_compact(
    manager: tauri::State<'_, TaskManager>,
    req: Request,
) -> CommandResult<Response> {
    let mut tasks = manager.inner.tasks.write();
    let record = require_running_codex_gui_record_mut(&mut tasks, req.task_id)?;
    record
        .agent
        .compact_thread()
        .map_err(|error| error.to_string())
}
