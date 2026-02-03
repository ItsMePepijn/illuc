pub mod commands;

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const TIME_TRACKING_VERSION: u32 = 1;
const TIME_TRACKING_FILE: &str = "time-tracking.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeTrackingStore {
    pub version: u32,
    pub branches: HashMap<String, BranchTimeEntry>,
}

impl Default for TimeTrackingStore {
    fn default() -> Self {
        Self {
            version: TIME_TRACKING_VERSION,
            branches: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchTimeEntry {
    pub branch_name: String,
    pub title: Option<String>,
    pub by_date: HashMap<String, u64>,
}

pub fn load_store(repo_root: &Path) -> Result<TimeTrackingStore> {
    let path = time_tracking_path(repo_root)?;
    if !path.exists() {
        return Ok(TimeTrackingStore::default());
    }
    let contents = std::fs::read_to_string(&path)?;
    let parsed: TimeTrackingStore = serde_json::from_str(&contents).unwrap_or_default();
    if parsed.version != TIME_TRACKING_VERSION {
        return Ok(TimeTrackingStore::default());
    }
    Ok(parsed)
}

pub fn save_store(repo_root: &Path, store: &TimeTrackingStore) -> Result<()> {
    let path = time_tracking_path(repo_root)?;
    let payload =
        serde_json::to_string_pretty(store).map_err(|err| anyhow::Error::from(err))?;
    std::fs::write(path, payload)?;
    Ok(())
}

fn time_tracking_path(repo_root: &Path) -> Result<PathBuf> {
    let illuc_dir = repo_root.join(".illuc");
    if !illuc_dir.exists() {
        std::fs::create_dir_all(&illuc_dir)?;
    }
    Ok(illuc_dir.join(TIME_TRACKING_FILE))
}
