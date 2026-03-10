use crate::commands::CommandResult;
use crate::features::tasks::agents::agent_gui::commands::task_agent_gui_common::require_gui_agent;
use crate::features::tasks::agents::AgentModelCapability;
use crate::features::tasks::TaskManager;
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
    pub capabilities: GuiAgentCapabilities,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuiAgentCapabilities {
    pub supports_new_chat: bool,
    pub supports_thread_history: bool,
    pub supports_usage: bool,
    pub supports_service_tier_toggle: bool,
}

#[tauri::command]
pub async fn task_agent_gui_models(
    manager: tauri::State<'_, TaskManager>,
    req: Request,
) -> CommandResult<Response> {
    let tasks = manager.inner.tasks.read();
    let gui_agent = require_gui_agent(&tasks, req.task_id)?;

    Ok(Response {
        models: gui_agent.available_models(),
        model_capabilities: gui_agent.available_model_capabilities(),
        selected_model: gui_agent.selected_model(),
        selected_effort: gui_agent.selected_reasoning_effort(),
        selected_service_tier: gui_agent.selected_service_tier(),
        capabilities: GuiAgentCapabilities {
            supports_new_chat: gui_agent.as_gui_session_agent().is_some(),
            supports_thread_history: gui_agent.as_gui_thread_agent().is_some(),
            supports_usage: gui_agent.as_gui_usage_agent().is_some(),
            supports_service_tier_toggle: gui_agent.supports_service_tier_toggle(),
        },
    })
}
