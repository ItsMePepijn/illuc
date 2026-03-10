use crate::commands::CommandResult;
use crate::features::tasks::agents::agent_gui::commands::task_agent_gui_common::with_running_gui_session_agent_mut;
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
pub async fn task_agent_gui_new_chat(
    manager: tauri::State<'_, TaskManager>,
    req: Request,
) -> CommandResult<Response> {
    with_running_gui_session_agent_mut(&manager, req.task_id, |gui_agent| {
        gui_agent
            .start_new_thread()
            .map_err(|error| error.to_string())
    })
}
