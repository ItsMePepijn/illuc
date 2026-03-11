use crate::commands::CommandResult;
use crate::features::launcher::{self, EditorApp};

pub type Response = Vec<EditorApp>;

#[tauri::command]
pub async fn list_installed_editors() -> CommandResult<Response> {
    Ok(launcher::list_installed_editors())
}
