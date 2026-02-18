use crate::commands::CommandResult;
use crate::features::tasks::events::emit_status;
use crate::features::tasks::git::{
    get_head_branch, get_head_commit, get_repo_root, list_worktrees, prune_worktrees,
    validate_git_repo,
};
use crate::features::tasks::worktree::{
    clean_branch_name, format_title_from_branch, managed_worktree_root,
};
use crate::features::tasks::{
    build_agent, AgentKind, TaskManager, TaskRecord, TaskStatus, TaskSummary,
};
use crate::utils::fs::ensure_directory;
use crate::utils::path::normalize_path_string;
use chrono::Utc;
use log::warn;
use std::path::PathBuf;
use uuid::Uuid;

pub type Request = String;
pub type Response = Vec<TaskSummary>;

#[tauri::command]
pub async fn task_load_existing(
    manager: tauri::State<'_, TaskManager>,
    app_handle: tauri::AppHandle,
    base_repo_path: Request,
) -> CommandResult<Response> {
    let provided_path = PathBuf::from(&base_repo_path);
    ensure_directory(&provided_path).map_err(|err| err.to_string())?;
    validate_git_repo(&provided_path).map_err(|err| err.to_string())?;
    let repo_root = get_repo_root(&provided_path)
        .map_err(|err| err.to_string())?
        .canonicalize()
        .unwrap_or_else(|err| {
            warn!(
                "failed to canonicalize repo root {}; using provided path: {}",
                provided_path.display(),
                err
            );
            provided_path.clone()
        });
    if let Err(err) = prune_worktrees(&repo_root) {
        warn!("worktree prune failed: {}", err);
    }
    let managed_root = managed_worktree_root(&repo_root).map_err(|err| err.to_string())?;
    let base_repo_head = get_head_commit(&repo_root).map_err(|err| err.to_string())?;
    let base_repo_branch = get_head_branch(&repo_root).unwrap_or_else(|err| {
        warn!(
            "failed to resolve base repo branch for {}; defaulting to HEAD: {}",
            repo_root.display(),
            err
        );
        "HEAD".to_string()
    });
    let entries = list_worktrees(&repo_root).map_err(|err| err.to_string())?;
    let mut inserted = Vec::new();
    for entry in entries {
        let canonical_path = entry.path.canonicalize().unwrap_or_else(|err| {
            warn!(
                "failed to canonicalize worktree path {}; using raw path: {}",
                entry.path.display(),
                err
            );
            entry.path.clone()
        });
        if canonical_path == repo_root {
            continue;
        }
        if !canonical_path.starts_with(&managed_root) {
            continue;
        }
        if manager.contains_worktree_path(&canonical_path) {
            continue;
        }
        let worktree_path_display = normalize_path_string(&canonical_path);
        let task_id = canonical_path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| Uuid::parse_str(name).ok())
            .unwrap_or_else(Uuid::new_v4);
        let branch_name = entry
            .branch
            .as_ref()
            .map(|name| clean_branch_name(name))
            .unwrap_or_else(|| {
                let short_head: String = entry.head.chars().take(7).collect();
                format!("detached-{}", short_head)
            });
        let summary = TaskSummary {
            task_id,
            title: format_title_from_branch(&branch_name),
            status: TaskStatus::Stopped,
            agent_kind: AgentKind::Codex,
            created_at: Utc::now(),
            started_at: None,
            ended_at: None,
            worktree_path: worktree_path_display,
            branch_name,
            base_branch: base_repo_branch.clone(),
            base_repo_path: normalize_path_string(&repo_root),
            base_commit: base_repo_head.clone(),
            exit_code: None,
        };
        manager.inner.tasks.write().insert(
            summary.task_id,
            TaskRecord {
                agent: build_agent(AgentKind::Codex),
                agent_kind: AgentKind::Codex,
                summary: summary.clone(),
                runtime: None,
                shell: None,
            },
        );
        emit_status(&app_handle, &summary);
        inserted.push(summary);
    }
    Ok(inserted)
}
