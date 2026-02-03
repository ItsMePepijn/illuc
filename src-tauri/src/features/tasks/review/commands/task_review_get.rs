use crate::commands::CommandResult;
use crate::features::tasks::review::{load_store, ReviewStore};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub worktree_path: String,
}

pub type Response = ReviewStore;

#[tauri::command]
pub async fn task_review_get(req: Request) -> CommandResult<Response> {
    let worktree_root = PathBuf::from(req.worktree_path);
    load_store(&worktree_root).map_err(|err| err.to_string())
}
