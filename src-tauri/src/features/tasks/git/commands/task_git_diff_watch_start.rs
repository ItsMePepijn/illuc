use crate::commands::CommandResult;
use crate::features::tasks::{DiffWatcher, TaskManager};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub task_id: Uuid,
}

pub type Response = ();

#[tauri::command]
pub async fn task_git_diff_watch_start(
    manager: tauri::State<'_, TaskManager>,
    app_handle: tauri::AppHandle,
    req: Request,
) -> CommandResult<Response> {
    let task_id = req.task_id;
    let worktree_path = manager
        .worktree_path(task_id)
        .map_err(|err| err.to_string())?;
    let mut watchers = manager.inner.diff_watchers.lock();
    if watchers.contains_key(&task_id) {
        return Ok(());
    }
    let watcher =
        DiffWatcher::new(task_id, worktree_path, app_handle).map_err(|err| err.to_string())?;
    watchers.insert(task_id, watcher);
    Ok(())
}
