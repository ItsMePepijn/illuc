use crate::commands::CommandResult;
use crate::error::TaskError;
use crate::features::tasks::events::{emit_diff_changed, emit_status};
use crate::features::tasks::git::{get_head_commit, git_merge_branch, git_push};
use crate::features::tasks::TaskManager;
use serde::Deserialize;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub task_id: Uuid,
    pub target_branch: String,
    pub push_main_after_merge: Option<bool>,
}

pub type Response = ();

#[tauri::command]
pub async fn task_git_merge(
    manager: tauri::State<'_, TaskManager>,
    app_handle: tauri::AppHandle,
    req: Request,
) -> CommandResult<Response> {
    let task_id = req.task_id;
    let target_branch = req.target_branch.trim().to_string();
    let push_main_after_merge = req.push_main_after_merge.unwrap_or(false);
    if target_branch.is_empty() {
        return Err(TaskError::Message("Target branch is required.".into()).to_string());
    }

    let (base_repo_path, source_branch) = {
        let tasks = manager.inner.tasks.read();
        let record = tasks
            .get(&task_id)
            .ok_or_else(|| TaskError::NotFound.to_string())?;
        (
            PathBuf::from(&record.summary.base_repo_path),
            record.summary.branch_name.clone(),
        )
    };

    git_merge_branch(base_repo_path.as_path(), target_branch.as_str(), source_branch.as_str())
        .map_err(|err| err.to_string())?;
    let base_commit = get_head_commit(base_repo_path.as_path()).map_err(|err| err.to_string())?;

    {
        let mut tasks = manager.inner.tasks.write();
        let record = tasks
            .get_mut(&task_id)
            .ok_or_else(|| TaskError::NotFound.to_string())?;
        record.summary.base_branch = target_branch.clone();
        record.summary.base_commit = base_commit;
        emit_status(&app_handle, &record.summary);
    }

    emit_diff_changed(&app_handle, task_id);
    if push_main_after_merge {
        git_push(base_repo_path.as_path(), "origin", target_branch.as_str(), false).map_err(
            |err| {
                format!(
                    "Merged into {} locally, but pushing {} failed: {}",
                    target_branch, target_branch, err
                )
            },
        )?;
    }
    Ok(())
}
