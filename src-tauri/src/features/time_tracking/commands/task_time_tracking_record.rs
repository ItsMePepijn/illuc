use crate::commands::CommandResult;
use crate::features::tasks::git::get_repo_root;
use crate::features::time_tracking::{load_store, save_store, BranchTimeEntry};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub base_repo_path: String,
    pub branch_name: String,
    pub title: Option<String>,
    pub days: HashMap<String, u64>,
}

#[tauri::command]
pub async fn task_time_tracking_record(req: Request) -> CommandResult<()> {
    if req.days.is_empty() || req.branch_name.trim().is_empty() {
        return Ok(());
    }
    let repo_root = get_repo_root(PathBuf::from(req.base_repo_path).as_path())
        .map_err(|err| err.to_string())?;
    let mut store = load_store(&repo_root).map_err(|err| err.to_string())?;
    let entry = store
        .branches
        .entry(req.branch_name.clone())
        .or_insert_with(|| BranchTimeEntry {
            branch_name: req.branch_name.clone(),
            title: req.title.clone(),
            by_date: HashMap::new(),
        });

    if let Some(title) = req.title {
        entry.title = Some(title);
    }

    for (day, seconds) in req.days {
        if seconds == 0 {
            continue;
        }
        let slot = entry.by_date.entry(day).or_insert(0);
        *slot = slot.saturating_add(seconds);
    }

    save_store(&repo_root, &store).map_err(|err| err.to_string())?;
    Ok(())
}
