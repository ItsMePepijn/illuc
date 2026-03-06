use crate::commands::CommandResult;
use crate::features::tasks::agents::codex_gui::commands::task_codex_gui_common::require_running_codex_gui_record_mut;
use crate::features::tasks::TaskManager;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub task_id: Uuid,
    pub content: String,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub service_tier: Option<String>,
}

pub type Response = ();

#[tauri::command]
pub async fn task_codex_gui_send(
    manager: tauri::State<'_, TaskManager>,
    req: Request,
) -> CommandResult<Response> {
    let content = req.content.trim().to_string();
    if content.is_empty() {
        return Ok(());
    }

    let mut tasks = manager.inner.tasks.write();
    let record = require_running_codex_gui_record_mut(&mut tasks, req.task_id)?;

    record
        .agent
        .set_model(req.model)
        .map_err(|error| error.to_string())?;
    record
        .agent
        .set_reasoning_effort(req.effort)
        .map_err(|error| error.to_string())?;
    record
        .agent
        .set_service_tier(req.service_tier)
        .map_err(|error| error.to_string())?;

    record
        .agent
        .send_message(content)
        .map_err(|error| error.to_string())
}
