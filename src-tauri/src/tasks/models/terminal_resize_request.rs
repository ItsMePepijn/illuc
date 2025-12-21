use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalResizeRequest {
    pub task_id: Uuid,
    pub cols: u16,
    pub rows: u16,
}
