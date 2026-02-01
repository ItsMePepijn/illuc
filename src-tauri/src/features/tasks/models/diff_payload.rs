use crate::features::tasks::git::DiffFile;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum DiffLineType {
    Add,
    Del,
    Context,
    Meta,
    Hunk,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    #[serde(rename = "type")]
    pub line_type: DiffLineType,
    pub content: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffPayload {
    pub task_id: Uuid,
    pub files: Vec<DiffFile>,
}
