use crate::commands::CommandResult;
use crate::error::TaskError;
use crate::features::tasks::agents::AgentModelCapability;
use crate::features::tasks::{AgentKind, TaskManager};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub task_id: Uuid,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub models: Vec<String>,
    pub model_capabilities: Vec<AgentModelCapability>,
    pub selected_model: Option<String>,
    pub selected_effort: Option<String>,
    pub selected_service_tier: Option<String>,
}

#[tauri::command]
pub async fn task_codex_gui_models(
    manager: tauri::State<'_, TaskManager>,
    req: Request,
) -> CommandResult<Response> {
    let tasks = manager.inner.tasks.read();
    let record = tasks
        .get(&req.task_id)
        .ok_or_else(|| TaskError::NotFound.to_string())?;

    if record.agent_kind != AgentKind::CodexGui {
        return Ok(Response {
            models: Vec::new(),
            model_capabilities: Vec::new(),
            selected_model: None,
            selected_effort: None,
            selected_service_tier: None,
        });
    }

    Ok(Response {
        models: record.agent.available_models(),
        model_capabilities: record.agent.available_model_capabilities(),
        selected_model: record.agent.selected_model(),
        selected_effort: record.agent.selected_reasoning_effort(),
        selected_service_tier: record.agent.selected_service_tier(),
    })
}
