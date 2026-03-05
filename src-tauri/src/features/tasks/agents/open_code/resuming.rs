use anyhow::Context;
#[cfg(target_os = "windows")]
use anyhow::anyhow;
use serde::Deserialize;
use std::fs;
use std::path::Path;
#[cfg(not(target_os = "windows"))]
use std::process::Command;

#[cfg(target_os = "windows")]
use crate::utils::windows::{build_wsl_process_command, to_wsl_path};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenCodeSessionSummary {
    id: String,
    directory: String,
    updated: i64,
}

#[cfg(target_os = "windows")]
fn normalize_directory(path: &Path) -> anyhow::Result<String> {
    let canonical = fs::canonicalize(path)
        .with_context(|| format!("failed to canonicalize {}", path.display()))?;
    to_wsl_path(&canonical)
        .ok_or_else(|| anyhow!("failed to convert {} to WSL path", canonical.display()))
}

#[cfg(not(target_os = "windows"))]
fn normalize_directory(path: &Path) -> anyhow::Result<String> {
    fs::canonicalize(path)
        .with_context(|| format!("failed to canonicalize {}", path.display()))
        .map(|path| path.to_string_lossy().to_string())
}

fn session_matches_directory(session_directory: &str, desired_directory: &str) -> bool {
    if session_directory == desired_directory {
        return true;
    }

    let session_path = Path::new(session_directory);
    match normalize_directory(session_path) {
        Ok(normalized) => normalized == desired_directory,
        Err(_) => false,
    }
}

pub fn find_latest_session_id(worktree_path: &Path) -> anyhow::Result<Option<String>> {
    let desired_directory = normalize_directory(worktree_path)?;

    #[cfg(target_os = "windows")]
    let output = build_wsl_process_command(
        worktree_path,
        "opencode",
        &["session", "list", "--format", "json"],
    )
    .output()
    .context("failed to list OpenCode sessions in WSL")?;

    #[cfg(not(target_os = "windows"))]
    let output = Command::new("opencode")
        .args(["session", "list", "--format", "json"])
        .current_dir(worktree_path)
        .output()
        .context("failed to list OpenCode sessions")?;

    if !output.status.success() {
        return Ok(None);
    }

    let sessions: Vec<OpenCodeSessionSummary> = serde_json::from_slice(&output.stdout)
        .context("failed to parse OpenCode session list JSON")?;

    Ok(sessions
        .into_iter()
        .filter(|session| session_matches_directory(&session.directory, &desired_directory))
        .max_by_key(|session| session.updated)
        .map(|session| session.id))
}
