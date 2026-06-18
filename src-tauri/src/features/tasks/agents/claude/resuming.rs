use anyhow::Context;
use chrono::{DateTime, Utc};
use log::warn;
use serde::Deserialize;
use std::fs;
use std::path::Path;
#[cfg(not(target_os = "windows"))]
use std::path::PathBuf;

use crate::utils::windows::windows_path_text_to_wsl_path;
#[cfg(target_os = "windows")]
use crate::utils::windows::{resolve_wsl_home_dir, to_wsl_path};

const CLAUDE_PROJECTS_DIR: &str = ".claude/projects";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeHistoryLine {
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    timestamp: Option<String>,
}

struct SessionCandidate {
    session_id: String,
    timestamp: Option<DateTime<Utc>>,
}

#[cfg(not(target_os = "windows"))]
fn resolve_home_dir() -> anyhow::Result<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .context("failed to resolve home directory")
}

#[cfg(target_os = "windows")]
fn normalize_directory(path: &Path) -> anyhow::Result<String> {
    let canonical = fs::canonicalize(path)
        .with_context(|| format!("failed to canonicalize {}", path.display()))?;
    to_wsl_path(&canonical)
        .with_context(|| format!("failed to convert {} to WSL path", canonical.display()))
}

#[cfg(not(target_os = "windows"))]
fn normalize_directory(path: &Path) -> anyhow::Result<String> {
    fs::canonicalize(path)
        .with_context(|| format!("failed to canonicalize {}", path.display()))
        .map(|path| path.to_string_lossy().to_string())
}

fn claude_project_dir_name(directory: &str) -> String {
    directory
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect()
}

fn session_cwd_matches(value: &str, desired_directory: &str) -> bool {
    let value = value.trim();
    if value == desired_directory {
        return true;
    }
    windows_path_text_to_wsl_path(value).as_deref() == Some(desired_directory)
}

fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value.trim())
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .ok()
}

fn parse_history_file(path: &Path, desired_directory: &str) -> Option<SessionCandidate> {
    let data = match fs::read_to_string(path) {
        Ok(data) => data,
        Err(error) => {
            warn!(
                "failed to read Claude session file {}: {}",
                path.display(),
                error
            );
            return None;
        }
    };

    let mut session_id = path
        .file_stem()
        .and_then(|value| value.to_str())
        .map(|value| value.to_string());
    let mut latest_timestamp: Option<DateTime<Utc>> = None;
    let mut matches_desired_directory = false;

    for line in data.lines().filter(|line| !line.trim().is_empty()) {
        let parsed: ClaudeHistoryLine = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(error) => {
                warn!(
                    "failed to parse Claude session JSON line in {}: {}",
                    path.display(),
                    error
                );
                continue;
            }
        };

        if let Some(cwd) = parsed.cwd.as_deref() {
            if session_cwd_matches(cwd, desired_directory) {
                matches_desired_directory = true;
            }
        }
        if let Some(id) = parsed.session_id {
            session_id = Some(id);
        }
        if let Some(timestamp) = parsed.timestamp.as_deref().and_then(parse_timestamp) {
            latest_timestamp = match latest_timestamp {
                Some(current) if current >= timestamp => Some(current),
                _ => Some(timestamp),
            };
        }
    }

    if !matches_desired_directory {
        return None;
    }

    Some(SessionCandidate {
        session_id: session_id?,
        timestamp: latest_timestamp,
    })
}

fn find_latest_session_in_dir(dir: &Path, desired_directory: &str) -> Option<String> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return None;
        }
        Err(error) => {
            warn!(
                "failed to read Claude project history directory {}: {}",
                dir.display(),
                error
            );
            return None;
        }
    };

    let mut best: Option<SessionCandidate> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
            continue;
        }
        let Some(candidate) = parse_history_file(&path, desired_directory) else {
            continue;
        };
        let replace = match (&candidate.timestamp, &best) {
            (Some(candidate_timestamp), Some(best)) => match best.timestamp {
                Some(best_timestamp) => candidate_timestamp > &best_timestamp,
                None => true,
            },
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => true,
        };
        if replace {
            best = Some(candidate);
        }
    }

    best.map(|candidate| candidate.session_id)
}

pub fn find_latest_session_id(worktree_path: &Path) -> anyhow::Result<Option<String>> {
    let desired_directory = normalize_directory(worktree_path)?;
    #[cfg(target_os = "windows")]
    let home_dir = resolve_wsl_home_dir()?;
    #[cfg(not(target_os = "windows"))]
    let home_dir = resolve_home_dir()?;

    let project_dir = home_dir
        .join(CLAUDE_PROJECTS_DIR)
        .join(claude_project_dir_name(&desired_directory));

    Ok(find_latest_session_in_dir(&project_dir, &desired_directory))
}
