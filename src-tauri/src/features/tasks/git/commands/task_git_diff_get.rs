use crate::commands::CommandResult;
use crate::error::TaskError;
use crate::features::tasks::git::DiffMode;
use crate::features::tasks::git::{git_diff, stage_all};
use crate::features::tasks::{DiffPayload, TaskManager};
use log::warn;
use serde::Deserialize;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub task_id: Uuid,
    pub ignore_whitespace: Option<bool>,
    pub mode: Option<DiffMode>,
}

pub type Response = DiffPayload;

#[tauri::command]
pub async fn task_git_diff_get(
    manager: tauri::State<'_, TaskManager>,
    req: Request,
) -> CommandResult<Response> {
    let task_id = req.task_id;
    let (worktree_path, base_commit) = {
        let tasks = manager.inner.tasks.read();
        let record = tasks
            .get(&task_id)
            .ok_or_else(|| TaskError::NotFound.to_string())?;
        (
            PathBuf::from(&record.summary.worktree_path),
            record.summary.base_commit.trim().to_string(),
        )
    };

    if let Err(err) = stage_all(worktree_path.as_path()) {
        warn!(
            "failed to stage files before diff for task {} at {}: {}",
            task_id,
            worktree_path.display(),
            err
        );
    }

    let whitespace_flag = if req.ignore_whitespace.unwrap_or(false) {
        Some("--ignore-all-space")
    } else {
        None
    };
    let mode = req.mode.unwrap_or(DiffMode::Worktree);
    match mode {
        DiffMode::Worktree => {
            let combined = git_diff(worktree_path.as_path(), "HEAD", whitespace_flag)
                .map_err(|err| err.to_string())?;
            Ok(DiffPayload {
                task_id,
                files: combined.files,
            })
        }
        DiffMode::Branch => {
            let branch_diff = git_diff(
                worktree_path.as_path(),
                base_commit.as_str(),
                whitespace_flag,
            )
            .map_err(|err| err.to_string())?;
            Ok(DiffPayload {
                task_id,
                files: branch_diff.files,
            })
        }
    }
}
