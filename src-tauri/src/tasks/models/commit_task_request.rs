use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitTaskRequest {
    pub task_id: Uuid,
    pub message: String,
    pub stage_all: Option<bool>,
}
