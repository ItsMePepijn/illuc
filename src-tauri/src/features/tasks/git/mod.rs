pub mod commands;

use crate::error::{Result, TaskError};
use crate::features::tasks::{DiffLine, DiffLineType};
use git2::{
    BranchType, Cred, Delta, DiffFormat, DiffOptions, IndexAddOption, PushOptions,
    RemoteCallbacks, Repository, Signature, Status, StatusOptions, WorktreeAddOptions,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum DiffMode {
    Worktree,
    Branch,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiffFile {
    pub path: String,
    pub status: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffPayloadResult {
    pub files: Vec<DiffFile>,
}

#[derive(Debug, Default, Clone)]
pub struct WorktreeEntry {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub head: String,
}

fn map_git_err(err: git2::Error) -> TaskError {
    TaskError::Message(err.message().to_string())
}

fn open_repo(path: &Path) -> Result<Repository> {
    Repository::discover(path).map_err(map_git_err)
}

pub fn validate_git_repo(path: &Path) -> Result<()> {
    let _ = open_repo(path)?;
    Ok(())
}

pub fn get_repo_root(path: &Path) -> Result<PathBuf> {
    let repo = open_repo(path)?;
    if let Some(workdir) = repo.workdir() {
        Ok(workdir.to_path_buf())
    } else {
        Ok(repo.path().to_path_buf())
    }
}

pub fn resolve_commit_id(path: &Path, rev: &str) -> Result<String> {
    let repo = open_repo(path)?;
    let object = repo.revparse_single(rev).map_err(map_git_err)?;
    let commit = object.peel_to_commit().map_err(map_git_err)?;
    Ok(commit.id().to_string())
}

pub fn get_head_commit(path: &Path) -> Result<String> {
    let repo = open_repo(path)?;
    let head = repo.head().map_err(map_git_err)?;
    let oid = head
        .target()
        .ok_or_else(|| TaskError::Message("HEAD is not a commit.".into()))?;
    Ok(oid.to_string())
}

pub fn get_head_branch(path: &Path) -> Result<String> {
    let repo = open_repo(path)?;
    let head = repo.head().map_err(map_git_err)?;
    Ok(head.shorthand().unwrap_or("HEAD").to_string())
}

pub fn list_branches(path: &Path) -> Result<Vec<String>> {
    let repo = open_repo(path)?;
    let mut branches = Vec::new();
    let iter = repo.branches(None).map_err(map_git_err)?;
    for branch in iter {
        let (branch, _) = branch.map_err(map_git_err)?;
        let name = branch.name().map_err(map_git_err)?.unwrap_or("HEAD");
        if !name.contains("HEAD") {
            branches.push(name.to_string());
        }
    }
    branches.sort();
    branches.dedup();
    Ok(branches)
}

pub fn add_worktree(
    repo_root: &Path,
    branch_name: &str,
    worktree_path: &Path,
    base_ref: &str,
    worktree_name: &str,
) -> Result<()> {
    let repo = open_repo(repo_root)?;
    let base_object = repo.revparse_single(base_ref).map_err(map_git_err)?;
    let base_commit = base_object.peel_to_commit().map_err(map_git_err)?;
    if repo
        .find_branch(branch_name, BranchType::Local)
        .is_err()
    {
        repo.branch(branch_name, &base_commit, false)
            .map_err(map_git_err)?;
    }
    let reference_name = format!("refs/heads/{}", branch_name);
    let reference = repo.find_reference(&reference_name).map_err(map_git_err)?;
    let mut options = WorktreeAddOptions::new();
    options.reference(Some(&reference));
    repo.worktree(worktree_name, worktree_path, Some(&options))
        .map_err(map_git_err)?;
    Ok(())
}

pub fn remove_worktree(repo_root: &Path, worktree_path: &Path) -> Result<()> {
    let repo = open_repo(repo_root)?;
    let worktrees = repo.worktrees().map_err(map_git_err)?;
    for name in worktrees.iter().flatten() {
        let worktree = repo.find_worktree(name).map_err(map_git_err)?;
        if worktree.path() == worktree_path {
            worktree.prune(None).map_err(map_git_err)?;
            return Ok(());
        }
    }
    Ok(())
}

pub fn delete_branch(repo_root: &Path, branch_name: &str) -> Result<()> {
    let repo = open_repo(repo_root)?;
    if let Ok(mut branch) = repo.find_branch(branch_name, BranchType::Local) {
        branch.delete().map_err(map_git_err)?;
    }
    Ok(())
}

pub fn list_worktrees(repo_root: &Path) -> Result<Vec<WorktreeEntry>> {
    let repo = open_repo(repo_root)?;
    let worktrees = repo.worktrees().map_err(map_git_err)?;
    let mut entries = Vec::new();
    for name in worktrees.iter().flatten() {
        let worktree = repo.find_worktree(name).map_err(map_git_err)?;
        let path = worktree.path();
        let worktree_repo = Repository::open(path).map_err(map_git_err)?;
        let head = worktree_repo.head().map_err(map_git_err)?;
        let head_oid = head
            .target()
            .ok_or_else(|| TaskError::Message("HEAD is not a commit.".into()))?;
        let branch = if head.is_branch() {
            head.shorthand().map(|name| name.to_string())
        } else {
            None
        };
        entries.push(WorktreeEntry {
            path: path.to_path_buf(),
            branch,
            head: head_oid.to_string(),
        });
    }
    Ok(entries)
}

pub fn prune_worktrees(repo_root: &Path) -> Result<()> {
    let repo = open_repo(repo_root)?;
    let worktrees = repo.worktrees().map_err(map_git_err)?;
    for name in worktrees.iter().flatten() {
        if let Ok(worktree) = repo.find_worktree(name) {
            let _ = worktree.prune(None);
        }
    }
    Ok(())
}

pub fn git_commit(repo: &Path, message: &str, stage_all: bool) -> Result<()> {
    let repo = open_repo(repo)?;
    let mut index = repo.index().map_err(map_git_err)?;
    if stage_all {
        index
            .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
            .map_err(map_git_err)?;
    }
    index.write().map_err(map_git_err)?;
    let tree_id = index.write_tree().map_err(map_git_err)?;
    let tree = repo.find_tree(tree_id).map_err(map_git_err)?;

    let signature = repo.signature().or_else(|_| {
        Signature::now("illuc", "illuc@local").map_err(map_git_err)
    })?;

    let mut parents = Vec::new();
    if let Ok(head) = repo.head() {
        if let Some(oid) = head.target() {
            if let Ok(parent) = repo.find_commit(oid) {
                parents.push(parent);
            }
        }
    }
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &parent_refs,
    )
    .map_err(map_git_err)?;

    Ok(())
}

pub fn stage_all(repo: &Path) -> Result<()> {
    let repo = open_repo(repo)?;
    let mut index = repo.index().map_err(map_git_err)?;
    index
        .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
        .map_err(map_git_err)?;
    index.write().map_err(map_git_err)?;
    Ok(())
}

pub fn git_push(repo: &Path, remote_name: &str, branch: &str, set_upstream: bool) -> Result<()> {
    let repo = open_repo(repo)?;
    let config = repo.config().map_err(map_git_err)?;

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(move |url, username_from_url, allowed| {
        if allowed.is_ssh_key() {
            if let Some(username) = username_from_url {
                if let Ok(cred) = Cred::ssh_key_from_agent(username) {
                    return Ok(cred);
                }
            }
        }

        if allowed.is_user_pass_plaintext() {
            if let Ok(cred) = Cred::credential_helper(&config, url, username_from_url) {
                return Ok(cred);
            }
            #[cfg(target_os = "windows")]
            if let Some(cred) = try_gcm_credential(url, username_from_url) {
                return Ok(cred);
            }
            return Err(git2::Error::from_str(
                "No git credentials configured. Configure credential.helper (e.g. manager-core) or set up SSH.",
            ));
        }

        if allowed.is_username() {
            if let Some(username) = username_from_url {
                return Cred::username(username);
            }
        }

        if allowed.is_default() {
            return Cred::default();
        }

        Err(git2::Error::from_str(
            "No supported credential methods available. Configure SSH or a credential helper.",
        ))
    });

    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(callbacks);

    let mut remote = repo.find_remote(remote_name).map_err(map_git_err)?;
    let local_ref = if branch.starts_with("refs/") {
        branch.to_string()
    } else {
        format!("refs/heads/{}", branch)
    };
    let refspec = format!("{}:refs/heads/{}", local_ref, branch);
    remote
        .push(&[refspec.as_str()], Some(&mut push_options))
        .map_err(map_git_err)?;

    if set_upstream {
        if let Ok(mut local_branch) = repo.find_branch(branch, BranchType::Local) {
            let upstream = format!("{}/{}", remote_name, branch);
            let _ = local_branch.set_upstream(Some(&upstream));
        }
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn try_gcm_credential(url: &str, username: Option<&str>) -> Option<Cred> {
    let mut config = git2::Config::new().ok()?;
    for helper in ["manager-core", "manager"] {
        if config.set_str("credential.helper", helper).is_err() {
            continue;
        }
        if let Ok(cred) = Cred::credential_helper(&config, url, username) {
            return Some(cred);
        }
    }
    None
}

fn map_delta_status(delta: Delta) -> &'static str {
    match delta {
        Delta::Unmodified => " ",
        Delta::Added => "A",
        Delta::Deleted => "D",
        Delta::Modified => "M",
        Delta::Renamed => "R",
        Delta::Copied => "C",
        Delta::Typechange => "T",
        Delta::Untracked => "?",
        Delta::Ignored => "I",
        Delta::Unreadable => "U",
        Delta::Conflicted => "U",
    }
}

fn map_line_type(origin: char) -> DiffLineType {
    match origin {
        '+' => DiffLineType::Add,
        '-' => DiffLineType::Del,
        ' ' => DiffLineType::Context,
        'H' => DiffLineType::Hunk,
        _ => DiffLineType::Meta,
    }
}

pub fn git_diff(
    repo: &Path,
    base_commit: &str,
    ignore_whitespace: Option<&str>,
) -> Result<DiffPayloadResult> {
    let repo = open_repo(repo)?;
    let base_object = repo.revparse_single(base_commit).map_err(map_git_err)?;
    let base_commit = base_object.peel_to_commit().map_err(map_git_err)?;
    let base_tree = base_commit.tree().map_err(map_git_err)?;

    let mut options = DiffOptions::new();
    if ignore_whitespace.is_some() {
        options.ignore_whitespace(true);
        options.ignore_whitespace_change(true);
        options.ignore_whitespace_eol(true);
    }

    let diff = repo
        .diff_tree_to_workdir_with_index(Some(&base_tree), Some(&mut options))
        .map_err(map_git_err)?;

    let mut files_by_path: HashMap<String, DiffFile> = HashMap::new();
    let mut file_order: Vec<String> = Vec::new();

    for delta in diff.deltas() {
        let status = map_delta_status(delta.status()).to_string();
        let path = match delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
        {
            Some(path) => path.to_string_lossy().to_string(),
            None => continue,
        };
        let entry = files_by_path.entry(path.clone());
        if let std::collections::hash_map::Entry::Vacant(vacant) = entry {
            file_order.push(path.clone());
            vacant.insert(DiffFile {
                path,
                status,
                lines: Vec::new(),
            });
        } else if let Some(file) = files_by_path.get_mut(&path) {
            file.status = status;
        }
    }

    diff.print(DiffFormat::Patch, |delta, _hunk, line| {
        let path = match delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
        {
            Some(path) => path.to_string_lossy().to_string(),
            None => return true,
        };
        if !files_by_path.contains_key(&path) {
            file_order.push(path.clone());
            files_by_path.insert(
                path.clone(),
                DiffFile {
                    path: path.clone(),
                    status: "M".to_string(),
                    lines: Vec::new(),
                },
            );
        }
        let content = String::from_utf8_lossy(line.content());
        let content = content.trim_end_matches(['\r', '\n']).to_string();
        if let Some(file) = files_by_path.get_mut(&path) {
            file.lines.push(DiffLine {
                line_type: map_line_type(line.origin()),
                content,
            });
        }
        true
    })
    .map_err(map_git_err)?;

    let mut files = Vec::with_capacity(file_order.len());
    for path in file_order {
        if let Some(file) = files_by_path.remove(&path) {
            files.push(file);
        }
    }

    Ok(DiffPayloadResult { files })
}

pub fn has_uncommitted_changes(repo: &Path) -> Result<bool> {
    let repo = open_repo(repo)?;
    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false)
        .include_unmodified(false)
        .exclude_submodules(true);
    let statuses = repo.statuses(Some(&mut options)).map_err(map_git_err)?;
    Ok(statuses
        .iter()
        .any(|entry| entry.status() != Status::CURRENT))
}
