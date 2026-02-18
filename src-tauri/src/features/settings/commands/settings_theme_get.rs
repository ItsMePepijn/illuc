use crate::commands::CommandResult;
use crate::features::settings::{
    load_selected_syntax_theme_name, load_theme_settings, resolve_default_theme_name,
};
use serde::Serialize;
use std::collections::HashMap;
use tauri::Manager;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub syntax_theme: String,
    pub values: HashMap<String, String>,
}

#[tauri::command]
pub async fn settings_theme_get(window: tauri::WebviewWindow) -> CommandResult<Response> {
    let default_theme_name = resolve_default_theme_name(Some(window.theme()));
    let app = window.app_handle();

    let values = load_theme_settings(app, &default_theme_name).map_err(|err| err.to_string())?;
    let syntax_theme =
        load_selected_syntax_theme_name(app, &default_theme_name).map_err(|err| err.to_string())?;
    Ok(Response {
        syntax_theme,
        values,
    })
}
