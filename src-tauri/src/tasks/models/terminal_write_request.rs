use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalWriteRequest {
    pub task_id: Uuid,
    pub data: String,
}
