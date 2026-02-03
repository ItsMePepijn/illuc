use crate::commands::CommandResult;

#[tauri::command]
pub async fn task_review_get_user_display_name() -> CommandResult<String> {
    let realname = whoami::realname();
    let trimmed = realname.trim();
    if !trimmed.is_empty() {
        return Ok(trimmed.to_string());
    }
    Ok(whoami::username())
}
