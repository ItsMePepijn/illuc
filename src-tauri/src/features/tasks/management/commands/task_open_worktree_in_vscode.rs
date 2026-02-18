use crate::commands::CommandResult;
use crate::features::launcher;
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
pub async fn task_open_worktree_in_vscode(
    manager: tauri::State<'_, TaskManager>,
    req: Request,
) -> CommandResult<Response> {
    let path = manager
        .worktree_path(req.task_id)
        .map_err(|err| err.to_string())?;
    launcher::open_path_in_vscode(path.as_path()).map_err(|err| err.to_string())
}
