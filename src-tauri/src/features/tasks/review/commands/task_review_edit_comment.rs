use crate::commands::CommandResult;
use crate::features::tasks::review::{find_thread_mut, load_store, save_store, ReviewComment};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub worktree_path: String,
    pub task_id: String,
    pub file_path: String,
    pub line_number_old: Option<u32>,
    pub line_number_new: Option<u32>,
    pub comment_id: String,
    pub body: String,
}

#[tauri::command]
pub async fn task_review_edit_comment(req: Request) -> CommandResult<ReviewComment> {
    let body = req.body.trim().to_string();
    if req.task_id.trim().is_empty()
        || req.file_path.trim().is_empty()
        || req.comment_id.trim().is_empty()
    {
        return Err("Review comment target is invalid.".to_string());
    }
    if req.line_number_old.is_none() && req.line_number_new.is_none() {
        return Err("Review comment must include a line number.".to_string());
    }
    if body.is_empty() {
        return Err("Review comment body cannot be empty.".to_string());
    }

    let worktree_root = PathBuf::from(&req.worktree_path);
    let mut store = load_store(&worktree_root).map_err(|err| err.to_string())?;
    let entry = store
        .tasks
        .get_mut(&req.task_id)
        .ok_or_else(|| "Review task entry not found.".to_string())?;
    let thread = find_thread_mut(
        entry,
        &req.file_path,
        req.line_number_old,
        req.line_number_new,
    )
    .ok_or_else(|| "Review thread not found.".to_string())?;
    let comment = thread
        .comments
        .iter_mut()
        .find(|comment| comment.id == req.comment_id)
        .ok_or_else(|| "Review comment not found.".to_string())?;
    comment.body = body;
    let updated = comment.clone();
    save_store(&worktree_root, &store).map_err(|err| err.to_string())?;
    Ok(updated)
}
