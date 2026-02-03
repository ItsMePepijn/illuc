use crate::commands::CommandResult;
use crate::features::tasks::git::get_repo_root;
use crate::features::time_tracking::{load_store, TimeTrackingStore};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub base_repo_path: String,
}

pub type Response = TimeTrackingStore;

#[tauri::command]
pub async fn task_time_tracking_get(req: Request) -> CommandResult<Response> {
    let repo_root = get_repo_root(PathBuf::from(req.base_repo_path).as_path())
        .map_err(|err| err.to_string())?;
    load_store(&repo_root).map_err(|err| err.to_string())
}
