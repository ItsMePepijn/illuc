use crate::commands::CommandResult;
use crate::features::tasks::agents::agent_gui::commands::task_agent_gui_common::with_running_gui_thread_agent_mut;
use crate::features::tasks::agents::agent_gui::types::GuiMessagePresentation;
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
    pub presentation: GuiMessagePresentation,
    pub is_delta: bool,
    pub is_final: bool,
}

#[tauri::command]
pub async fn task_agent_gui_rollback(
    manager: tauri::State<'_, TaskManager>,
    req: Request,
) -> CommandResult<Response> {
    with_running_gui_thread_agent_mut(&manager, req.task_id, |gui_agent| {
        let events = gui_agent
            .rollback_thread(req.num_turns.unwrap_or(1))
            .map_err(|error| error.to_string())?;
        Ok(Response {
            events: events
                .into_iter()
                .map(|event| RollbackEvent {
                    message_id: event.message_id,
                    role: event.role.as_str().to_string(),
                    content: event.content,
                    presentation: event.presentation,
                    is_delta: event.is_delta,
                    is_final: event.is_final,
                })
                .collect(),
        })
    })
}
