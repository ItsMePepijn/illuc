use crate::commands::CommandResult;
use crate::features::launcher;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub path: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

pub type Response = ();

#[tauri::command]
pub async fn open_file_in_vscode(req: Request) -> CommandResult<Response> {
    let target = std::path::PathBuf::from(req.path);
    launcher::open_file_in_vscode(target.as_path(), req.line, req.column)
        .map_err(|err| err.to_string())
}
