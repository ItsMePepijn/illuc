use crate::commands::CommandResult;
use crate::error::TaskError;
use crate::features::tasks::events::emit_status;
use crate::features::tasks::git::{delete_branch, remove_worktree};
use crate::features::tasks::{TaskManager, TaskStatus};
use log::warn;
use serde::Deserialize;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub task_id: Uuid,
}

pub type Response = ();

#[tauri::command]
pub async fn task_discard(
    manager: tauri::State<'_, TaskManager>,
    app_handle: tauri::AppHandle,
    req: Request,
) -> CommandResult<Response> {
    let task_id = req.task_id;
    manager.remove_diff_watch(task_id);
    let (worktree_path, branch_name, base_repo_path, runtime_exists, shell_exists) = {
        let tasks = manager.inner.tasks.read();
        let record = tasks
            .get(&task_id)
            .ok_or_else(|| TaskError::NotFound.to_string())?;
        (
            PathBuf::from(&record.summary.worktree_path),
            record.summary.branch_name.clone(),
            PathBuf::from(&record.summary.base_repo_path),
            record.runtime.is_some(),
            record.shell.is_some(),
        )
    };

    if runtime_exists {
        let child = {
            let tasks = manager.inner.tasks.read();
            let record = tasks
                .get(&task_id)
                .ok_or_else(|| TaskError::NotFound.to_string())?;
            record.runtime.as_ref().map(|runtime| runtime.child.clone())
        };
        if let Some(child) = child {
            if let Some(mut child_guard) = child.try_lock() {
                if let Err(err) = child_guard.kill() {
                    warn!("failed to kill task process for {}: {}", task_id, err);
                }
            }
            let mut tasks = manager.inner.tasks.write();
            if let Some(record) = tasks.get_mut(&task_id) {
                record.summary.status = TaskStatus::Stopped;
                emit_status(&app_handle, &record.summary);
            }
        }
    }

    if shell_exists {
        let mut tasks = manager.inner.tasks.write();
        if let Some(record) = tasks.get_mut(&task_id) {
            if let Some(shell) = record.shell.take() {
                if let Some(mut child_guard) = shell.child.try_lock() {
                    if let Err(err) = child_guard.kill() {
                        warn!("failed to kill worktree shell for {}: {}", task_id, err);
                    }
                }
            }
        }
    }

    if let Err(err) = remove_worktree(&base_repo_path, &worktree_path) {
        warn!(
            "failed to remove worktree {} for {}: {}",
            worktree_path.display(),
            task_id,
            err
        );
    }
    if let Err(err) = delete_branch(&base_repo_path, branch_name.as_str()) {
        warn!(
            "failed to delete branch {} for {}: {}",
            branch_name, task_id, err
        );
    }
    if worktree_path.exists() {
        if let Err(err) = std::fs::remove_dir_all(&worktree_path) {
            warn!(
                "failed to remove worktree directory {} for {}: {}",
                worktree_path.display(),
                task_id,
                err
            );
        }
    }

    {
        let mut tasks = manager.inner.tasks.write();
        if let Some(record) = tasks.get_mut(&task_id) {
            record.summary.status = TaskStatus::Discarded;
            record.runtime = None;
            emit_status(&app_handle, &record.summary);
        }
    }

    let mut tasks = manager.inner.tasks.write();
    tasks.remove(&task_id);
    Ok(())
}
