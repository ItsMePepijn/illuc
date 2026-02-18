use crate::commands::CommandResult;
use crate::error::TaskError;
use crate::features::tasks::events::emit_status;
use crate::features::tasks::git::{
    add_worktree, fetch_base_branch_best_effort, get_repo_root, resolve_commit_id,
    validate_git_repo,
};
use crate::features::tasks::worktree::managed_worktree_root;
use crate::features::tasks::{
    build_agent, AgentKind, TaskManager, TaskRecord, TaskStatus, TaskSummary,
};
use crate::utils::fs::ensure_directory;
use crate::utils::path::normalize_path_string;
use chrono::Utc;
use log::warn;
use serde::Deserialize;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub base_repo_path: String,
    pub task_title: Option<String>,
    pub base_ref: Option<String>,
    pub branch_name: Option<String>,
}

pub type Response = TaskSummary;

#[tauri::command]
pub async fn task_create(
    manager: tauri::State<'_, TaskManager>,
    app_handle: tauri::AppHandle,
    req: Request,
) -> CommandResult<Response> {
    let Request {
        base_repo_path,
        task_title,
        base_ref,
        branch_name,
    } = req;

    let base_repo = PathBuf::from(base_repo_path);
    let repo_root = get_repo_root(&base_repo).map_err(|err| err.to_string())?;
    ensure_directory(&base_repo).map_err(|err| err.to_string())?;

    validate_git_repo(&base_repo).map_err(|err| err.to_string())?;

    let base_ref = base_ref.unwrap_or_else(|| "HEAD".to_string());

    if let Err(err) = fetch_base_branch_best_effort(&repo_root, base_ref.as_str()) {
        warn!(
            "git fetch/ff-only for base ref '{}' failed: {}",
            base_ref.as_str(),
            err
        );
    }

    let base_commit =
        resolve_commit_id(&repo_root, base_ref.as_str()).map_err(|err| err.to_string())?;

    let task_id = Uuid::new_v4();
    let title = task_title.unwrap_or_else(|| format!("Task {}", task_id.simple()));
    let timestamp = Utc::now();
    let branch_name = branch_name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| TaskError::Message("Branch name is required.".into()).to_string())?;

    let managed_root = managed_worktree_root(&repo_root).map_err(|err| err.to_string())?;
    let worktree_path = managed_root.join(task_id.to_string());

    if worktree_path.exists() {
        if let Err(err) = std::fs::remove_dir_all(&worktree_path) {
            warn!(
                "failed to remove pre-existing worktree path {}: {}",
                worktree_path.display(),
                err
            );
        }
    }

    let worktree_path_display = normalize_path_string(&worktree_path);
    let worktree_name = task_id.simple().to_string();
    add_worktree(
        &repo_root,
        branch_name.as_str(),
        &worktree_path,
        base_ref.as_str(),
        worktree_name.as_str(),
    )
    .map_err(|err| err.to_string())?;

    let summary = TaskSummary {
        task_id,
        title,
        status: TaskStatus::Stopped,
        agent_kind: AgentKind::Codex,
        created_at: timestamp,
        started_at: None,
        ended_at: None,
        worktree_path: worktree_path_display,
        branch_name,
        base_branch: base_ref.clone(),
        base_repo_path: normalize_path_string(&repo_root),
        base_commit,
        exit_code: None,
    };

    let mut tasks = manager.inner.tasks.write();
    tasks.insert(
        task_id,
        TaskRecord {
            agent: build_agent(AgentKind::Codex),
            agent_kind: AgentKind::Codex,
            summary: summary.clone(),
            runtime: None,
            shell: None,
        },
    );
    drop(tasks);
    emit_status(&app_handle, &summary);
    Ok(summary)
}
