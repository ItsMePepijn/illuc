pub mod commands;

use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const REVIEW_VERSION: u32 = 1;
const REVIEW_FILE: &str = "local-review.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewStore {
    pub version: u32,
    pub tasks: HashMap<String, TaskReviewEntry>,
}

impl Default for ReviewStore {
    fn default() -> Self {
        Self {
            version: REVIEW_VERSION,
            tasks: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskReviewEntry {
    pub task_id: String,
    pub comments: Vec<ReviewComment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewComment {
    pub id: String,
    pub file_path: String,
    pub line_number_old: Option<u32>,
    pub line_number_new: Option<u32>,
    pub line_type: ReviewLineType,
    #[serde(default)]
    pub status: ReviewCommentStatus,
    pub body: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewCommentStatus {
    Active,
    Pending,
    Resolved,
    WontFix,
    Closed,
}

impl Default for ReviewCommentStatus {
    fn default() -> Self {
        Self::Active
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewLineType {
    Add,
    Del,
    Context,
    Meta,
    Hunk,
}

pub fn load_store(worktree_root: &Path) -> Result<ReviewStore> {
    let path = review_path(worktree_root)?;
    if !path.exists() {
        return Ok(ReviewStore::default());
    }
    let contents = std::fs::read_to_string(&path)?;
    let parsed: ReviewStore = serde_json::from_str(&contents).unwrap_or_default();
    if parsed.version != REVIEW_VERSION {
        return Ok(ReviewStore::default());
    }
    Ok(parsed)
}

pub fn save_store(worktree_root: &Path, store: &ReviewStore) -> Result<()> {
    let path = review_path(worktree_root)?;
    let payload =
        serde_json::to_string_pretty(store).map_err(|err| anyhow::Error::from(err))?;
    std::fs::write(path, payload)?;
    Ok(())
}

fn review_path(worktree_root: &Path) -> Result<PathBuf> {
    let illuc_dir = worktree_root.join(".illuc");
    if !illuc_dir.exists() {
        std::fs::create_dir_all(&illuc_dir)?;
    }
    Ok(illuc_dir.join(REVIEW_FILE))
}
