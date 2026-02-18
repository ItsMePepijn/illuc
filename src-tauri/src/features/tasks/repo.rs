use crate::error::Result;
use crate::features::tasks::git::{get_head_branch, get_head_commit, validate_git_repo};
use crate::features::tasks::models::BaseRepoInfo;
use crate::utils::fs::ensure_directory;
use crate::utils::path::normalize_path_string;
use log::warn;
use std::path::PathBuf;

pub fn handle_select_base_repo(path: String) -> Result<BaseRepoInfo> {
    let repo = PathBuf::from(&path);
    ensure_directory(&repo)?;
    validate_git_repo(&repo)?;
    let canonical_path = normalize_path_string(&repo.canonicalize().unwrap_or_else(|err| {
        warn!(
            "failed to canonicalize selected base repo {}; using provided path: {}",
            repo.display(),
            err
        );
        repo.clone()
    }));
    let current_branch = get_head_branch(&repo)?;
    let head = get_head_commit(&repo)?;
    Ok(BaseRepoInfo {
        path,
        canonical_path,
        current_branch,
        head,
    })
}
