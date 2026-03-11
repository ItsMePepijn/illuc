use crate::commands::CommandResult;
use crate::features::launcher;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub path: String,
    pub editor_id: String,
}

pub type Response = ();

#[tauri::command]
pub async fn open_path_in_editor(req: Request) -> CommandResult<Response> {
    let target = std::path::PathBuf::from(req.path);
    launcher::open_path_in_editor(target.as_path(), &req.editor_id).map_err(|err| err.to_string())
}
