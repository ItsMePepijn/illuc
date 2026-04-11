use crate::commands::CommandResult;
use crate::features::tasks::{AgentKind, AgentKindAvailability};

pub type Response = Vec<AgentKindAvailability>;

#[tauri::command]
pub async fn task_agent_kinds_list() -> CommandResult<Response> {
    Ok(AgentKind::ALL
        .into_iter()
        .map(AgentKindAvailability::from_kind)
        .collect())
}
