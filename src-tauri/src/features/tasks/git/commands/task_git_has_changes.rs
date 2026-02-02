use crate::commands::CommandResult;
use crate::features::tasks::TaskManager;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub task_id: Uuid,
}

pub type Response = bool;

#[tauri::command]
pub async fn task_git_has_changes(
    manager: tauri::State<'_, TaskManager>,
    req: Request,
) -> CommandResult<Response> {
    manager
        .has_uncommitted_changes(req)
        .map_err(|err| err.to_string())
}
