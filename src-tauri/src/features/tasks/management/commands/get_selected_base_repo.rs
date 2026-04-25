use crate::commands::CommandResult;
use crate::features::tasks::{BaseRepoInfo, TaskManager};

pub type Response = Option<BaseRepoInfo>;

#[tauri::command]
pub async fn get_selected_base_repo(
    manager: tauri::State<'_, TaskManager>,
) -> CommandResult<Response> {
    Ok(manager.selected_base_repo())
}
