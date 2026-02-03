use crate::commands::CommandResult;
use crate::features::tasks::review::{
    find_thread_mut, load_store, save_store, ReviewComment, ReviewLineType, ReviewThread,
    TaskReviewEntry,
};
use chrono::Utc;
use serde::Deserialize;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub worktree_path: String,
    pub task_id: String,
    pub file_path: String,
    pub line_number_old: Option<u32>,
    pub line_number_new: Option<u32>,
    pub line_type: ReviewLineType,
    pub body: String,
}

#[tauri::command]
pub async fn task_review_add_comment(req: Request) -> CommandResult<ReviewComment> {
    let body = req.body.trim().to_string();
    if body.is_empty() {
        return Err("Review comment body cannot be empty.".to_string());
    }
    if req.file_path.trim().is_empty() || req.task_id.trim().is_empty() {
        return Err("Review comment target is invalid.".to_string());
    }
    if req.line_number_old.is_none() && req.line_number_new.is_none() {
        return Err("Review comment must include a line number.".to_string());
    }
    let worktree_path = req.worktree_path.clone();
    let worktree_root = PathBuf::from(&worktree_path);
    let mut store = load_store(&worktree_root).map_err(|err| err.to_string())?;
    let entry = store
        .tasks
        .entry(req.task_id.clone())
        .or_insert_with(|| TaskReviewEntry {
            task_id: req.task_id.clone(),
            threads: Vec::new(),
        });

    if find_thread_mut(
        entry,
        &req.file_path,
        req.line_number_old,
        req.line_number_new,
    )
    .is_none()
    {
        entry.threads.push(ReviewThread {
            file_path: req.file_path.clone(),
            line_number_old: req.line_number_old,
            line_number_new: req.line_number_new,
            line_type: req.line_type,
            status: Default::default(),
            comments: Vec::new(),
        });
    }
    let thread = find_thread_mut(
        entry,
        &req.file_path,
        req.line_number_old,
        req.line_number_new,
    )
    .ok_or_else(|| "Review thread target is invalid.".to_string())?;
    let comment = ReviewComment {
        id: Uuid::new_v4().to_string(),
        body,
        author: "user".to_string(),
        created_at: Utc::now(),
    };
    thread.comments.push(comment.clone());
    save_store(&worktree_root, &store).map_err(|err| err.to_string())?;
    Ok(comment)
}
