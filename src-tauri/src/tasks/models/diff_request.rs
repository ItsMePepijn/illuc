use crate::tasks::git::DiffMode;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffRequest {
    pub task_id: Uuid,
    pub ignore_whitespace: Option<bool>,
    pub mode: Option<DiffMode>,
}
