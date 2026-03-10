use crate::commands::CommandResult;
use crate::features::tasks::agents::agent_gui::commands::task_agent_gui_common::with_running_gui_agent_mut;
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
pub async fn task_agent_gui_send(
    manager: tauri::State<'_, TaskManager>,
    req: Request,
) -> CommandResult<Response> {
    let content = req.content.trim().to_string();
    if content.is_empty() {
        return Ok(());
    }

    with_running_gui_agent_mut(&manager, req.task_id, |gui_agent| {
        gui_agent
            .set_model(req.model)
            .map_err(|error| error.to_string())?;
        gui_agent
            .set_reasoning_effort(req.effort)
            .map_err(|error| error.to_string())?;
        gui_agent
            .set_service_tier(req.service_tier)
            .map_err(|error| error.to_string())?;

        gui_agent
            .send_message(content)
            .map_err(|error| error.to_string())
    })
}
