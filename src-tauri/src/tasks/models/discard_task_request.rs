use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscardTaskRequest {
    pub task_id: Uuid,
}
