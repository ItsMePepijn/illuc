use crate::commands::CommandResult;
use crate::features::tasks::review::{load_store, save_store, ReviewComment, ReviewCommentStatus};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub worktree_path: String,
    pub task_id: String,
    pub comment_id: String,
    pub status: ReviewCommentStatus,
}

#[tauri::command]
pub async fn task_review_update_comment_status(
    req: Request,
) -> CommandResult<ReviewComment> {
    if req.task_id.trim().is_empty() || req.comment_id.trim().is_empty() {
        return Err("Review comment target is invalid.".to_string());
    }
    let worktree_root = PathBuf::from(&req.worktree_path);
    let mut store = load_store(&worktree_root).map_err(|err| err.to_string())?;
    let entry = store
        .tasks
        .get_mut(&req.task_id)
        .ok_or_else(|| "Review task entry not found.".to_string())?;
    let mut updated: Option<ReviewComment> = None;
    for comment in &mut entry.comments {
        if comment.id == req.comment_id {
            comment.status = req.status;
            updated = Some(comment.clone());
            break;
        }
    }
    let updated =
        updated.ok_or_else(|| "Review comment not found.".to_string())?;
    save_store(&worktree_root, &store).map_err(|err| err.to_string())?;
    Ok(updated)
}
