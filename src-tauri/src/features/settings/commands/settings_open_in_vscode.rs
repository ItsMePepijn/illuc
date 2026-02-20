use crate::commands::CommandResult;
use crate::features::launcher;
use crate::features::settings::ensure_user_settings_file;
use tauri::Manager;

pub type Response = ();

#[tauri::command]
pub async fn settings_open_in_vscode(window: tauri::WebviewWindow) -> CommandResult<Response> {
    let app = window.app_handle();
    let settings_path = ensure_user_settings_file(app).map_err(|err| err.to_string())?;
    launcher::open_file_in_vscode(settings_path.as_path(), None, None).map_err(|err| err.to_string())
}
