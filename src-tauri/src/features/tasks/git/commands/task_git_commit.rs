use crate::commands::CommandResult;
use crate::error::TaskError;
use crate::features::tasks::events::emit_diff_changed;
use crate::features::tasks::git::git_commit;
use crate::features::tasks::TaskManager;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub task_id: Uuid,
    pub message: String,
    pub stage_all: Option<bool>,
}

pub type Response = ();

#[tauri::command]
pub async fn task_git_commit(
    manager: tauri::State<'_, TaskManager>,
    app_handle: tauri::AppHandle,
    req: Request,
) -> CommandResult<Response> {
    let task_id = req.task_id;
    let message = req.message.trim();
    if message.is_empty() {
        return Err(TaskError::Message("Commit message is required.".into()).to_string());
    }
    let stage_all = req.stage_all.unwrap_or(true);
    let worktree_path = manager
        .worktree_path(task_id)
        .map_err(|err| err.to_string())?;
    git_commit(worktree_path.as_path(), message, stage_all).map_err(|err| err.to_string())?;
    emit_diff_changed(&app_handle, task_id);
    Ok(())
}
