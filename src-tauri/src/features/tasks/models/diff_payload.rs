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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_number_old: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_number_new: Option<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffPayload {
    pub task_id: Uuid,
    pub files: Vec<DiffFile>,
}
