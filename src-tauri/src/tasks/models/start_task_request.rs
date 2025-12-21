use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartTaskRequest {
    pub task_id: Uuid,
    pub codex_args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
}
