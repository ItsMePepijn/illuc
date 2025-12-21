use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskRequest {
    pub base_repo_path: String,
    pub task_title: Option<String>,
    pub base_ref: Option<String>,
    pub branch_name: Option<String>,
}
