use crate::commands::CommandResult;
use crate::features::tasks::{handle_select_base_repo, BaseRepoInfo, TaskManager};

pub type Request = String;
pub type Response = BaseRepoInfo;

#[tauri::command]
pub async fn select_base_repo(
    manager: tauri::State<'_, TaskManager>,
    path: Request,
) -> CommandResult<Response> {
    let repo = handle_select_base_repo(path).map_err(|err| err.to_string())?;
    manager.set_selected_base_repo(repo.clone());
    Ok(repo)
}
