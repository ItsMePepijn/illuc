use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushTaskRequest {
    pub task_id: Uuid,
    pub remote: Option<String>,
    pub branch: Option<String>,
    pub set_upstream: Option<bool>,
}
