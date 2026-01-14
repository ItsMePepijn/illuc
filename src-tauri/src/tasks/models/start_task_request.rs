use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum AgentKind {
    Codex,
    Copilot,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartTaskRequest {
    pub task_id: Uuid,
    pub cols: Option<u16>,
    pub rows: Option<u16>,
    pub agent: Option<AgentKind>,
}
