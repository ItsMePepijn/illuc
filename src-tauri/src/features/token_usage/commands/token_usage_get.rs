use crate::commands::CommandResult;
use crate::features::tasks::git::get_repo_root;
use crate::features::token_usage::load_token_usage;
use serde::Deserialize;
use std::path::PathBuf;
use tauri::AppHandle;
use tokio::task;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub base_repo_path: String,
}

#[tauri::command]
pub async fn token_usage_get(
    app: AppHandle,
    req: Request,
) -> CommandResult<crate::features::token_usage::TokenUsageResponse> {
    let app = app.clone();
    let base_repo_path = PathBuf::from(req.base_repo_path);
    task::spawn_blocking(move || {
        let repo_root = get_repo_root(base_repo_path.as_path()).map_err(|err| err.to_string())?;
        load_token_usage(&app, &repo_root).map_err(|err| err.to_string())
    })
    .await
    .map_err(|err| err.to_string())?
}
