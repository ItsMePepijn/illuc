use crate::commands::CommandResult;
use crate::error::TaskError;
use crate::features::tasks::events::emit_diff_changed;
use crate::features::tasks::git::git_push;
use crate::features::tasks::TaskManager;
use serde::Deserialize;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub task_id: Uuid,
    pub remote: Option<String>,
    pub branch: Option<String>,
    pub set_upstream: Option<bool>,
}

pub type Response = ();

#[tauri::command]
pub async fn task_git_push(
    manager: tauri::State<'_, TaskManager>,
    app_handle: tauri::AppHandle,
    req: Request,
) -> CommandResult<Response> {
    let task_id = req.task_id;
    let (worktree_path, branch_name) = {
        let tasks = manager.inner.tasks.read();
        let record = tasks
            .get(&task_id)
            .ok_or_else(|| TaskError::NotFound.to_string())?;
        (
            PathBuf::from(&record.summary.worktree_path),
            record.summary.branch_name.clone(),
        )
    };
    let remote = req.remote.unwrap_or_else(|| "origin".to_string());
    let branch = req.branch.unwrap_or(branch_name);
    let set_upstream = req.set_upstream.unwrap_or(true);
    git_push(
        worktree_path.as_path(),
        remote.as_str(),
        branch.as_str(),
        set_upstream,
    )
    .map_err(|err| err.to_string())?;
    emit_diff_changed(&app_handle, task_id);
    Ok(())
}
