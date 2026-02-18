use crate::commands::CommandResult;
use crate::features::tasks::git::has_uncommitted_changes;
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
    let path = manager
        .worktree_path(req.task_id)
        .map_err(|err| err.to_string())?;
    has_uncommitted_changes(path.as_path()).map_err(|err| err.to_string())
}
