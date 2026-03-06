use anyhow::Context;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use log::warn;
use std::fs;
use std::path::Path;

#[cfg(target_os = "windows")]
use crate::utils::windows::{build_wsl_process_command, to_wsl_path};

const COPILOT_SESSION_DIR: &str = ".copilot/session-state";
const COPILOT_LEGACY_SESSION_DIR: &str = ".copilot/history-session-state";

struct SessionCandidate {
    session_id: String,
    timestamp: Option<DateTime<Utc>>,
}

fn resolve_session_cwd(worktree_path: &Path) -> anyhow::Result<String> {
    let canonical = fs::canonicalize(worktree_path)
        .with_context(|| format!("failed to resolve cwd {}", worktree_path.display()))?;
    #[cfg(target_os = "windows")]
    if let Some(wsl_path) = to_wsl_path(&canonical) {
        return Ok(wsl_path);
    }
    Ok(canonical.to_string_lossy().to_string())
}

#[cfg(not(target_os = "windows"))]
fn resolve_home_dir() -> anyhow::Result<std::path::PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .context("failed to resolve home directory")
}

#[cfg(target_os = "windows")]
fn resolve_wsl_home_dir(worktree_path: &Path) -> anyhow::Result<std::path::PathBuf> {
    let output = build_wsl_process_command(
        worktree_path,
        "bash",
        &["-lc", "wslpath -w \"$HOME\""],
    )
    .output()
    .context("failed to query WSL home directory")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!("failed to query WSL home directory"));
    }
    let home = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if home.is_empty() {
        return Err(anyhow::anyhow!("WSL home directory is empty"));
    }
    Ok(std::path::PathBuf::from(home))
}

fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    let mut normalized = value.trim().to_string();
    if normalized.ends_with('Z') {
        normalized = format!("{}+00:00", normalized.trim_end_matches('Z'));
    }
    if let Ok(parsed) = DateTime::parse_from_rfc3339(&normalized) {
        return Some(parsed.with_timezone(&Utc));
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(&normalized, "%Y-%m-%dT%H:%M:%S%.f") {
        return Some(Utc.from_utc_datetime(&naive));
    }
    None
}

fn parse_session_file(path: &Path, desired_cwd: &str) -> Option<SessionCandidate> {
    let data = match fs::read_to_string(path) {
        Ok(data) => data,
        Err(error) => {
            warn!(
                "failed to read copilot session file {}: {}",
                path.display(),
                error
            );
            return None;
        }
    };
    if !data.contains(desired_cwd) {
        return None;
    }

    let mut session_id: Option<String> = None;
    let mut latest_timestamp: Option<DateTime<Utc>> = None;

    for line in data.lines() {
        let value: serde_json::Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(error) => {
                warn!(
                    "failed to parse copilot session JSON line in {}: {}",
                    path.display(),
                    error
                );
                continue;
            }
        };
        if session_id.is_none() {
            if value.get("type").and_then(|value| value.as_str()) == Some("session.start") {
                if let Some(id) = value
                    .get("data")
                    .and_then(|value| value.get("sessionId"))
                    .and_then(|value| value.as_str())
                {
                    session_id = Some(id.to_string());
                }
            }
        }
        if let Some(ts) = value
            .get("timestamp")
            .and_then(|value| value.as_str())
            .and_then(parse_timestamp)
        {
            latest_timestamp = match latest_timestamp {
                Some(current) if current >= ts => Some(current),
                _ => Some(ts),
            };
        }
    }

    let session_id = session_id.or_else(|| {
        path.file_stem()
            .and_then(|value| value.to_str())
            .map(|value| value.to_string())
    })?;

    let timestamp = latest_timestamp;
    Some(SessionCandidate {
        session_id,
        timestamp,
    })
}

fn parse_session_dir(path: &Path, desired_cwd: &str) -> Option<SessionCandidate> {
    let yaml_path = path.join("workspace.yaml");
    let data = match fs::read_to_string(&yaml_path) {
        Ok(data) => data,
        Err(error) => {
            warn!(
                "failed to read {}: {}",
                yaml_path.display(),
                error
            );
            return None;
        }
    };

    let workspace: serde_json::Value = match serde_yaml::from_str(&data) {
        Ok(value) => value,
        Err(error) => {
            warn!(
                "failed to parse {}: {}",
                yaml_path.display(),
                error
            );
            return None;
        }
    };

    let cwd = workspace.get("cwd").and_then(|v| v.as_str())?;
    if cwd != desired_cwd {
        return None;
    }

    let session_id = workspace
        .get("id")
        .and_then(|v| v.as_str())
        .map(|v| v.to_string())
        .or_else(|| {
            path.file_name()
                .and_then(|v| v.to_str())
                .map(|v| v.to_string())
        })?;

    let timestamp = workspace
        .get("updated_at")
        .and_then(|v| v.as_str())
        .and_then(parse_timestamp);

    Some(SessionCandidate {
        session_id,
        timestamp,
    })
}

fn find_latest_session_in_dir(dir: &Path, desired_cwd: &str) -> Option<String> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            warn!(
                "failed to read copilot session directory {}: {}",
                dir.display(),
                error
            );
            return None;
        }
    };
    let mut best: Option<SessionCandidate> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) => {
                warn!(
                    "failed to read copilot session entry type for {}: {}",
                    path.display(),
                    error
                );
                continue;
            }
        };
        let candidate = if file_type.is_file() {
            parse_session_file(&path, desired_cwd)
        } else if file_type.is_dir() {
            parse_session_dir(&path, desired_cwd)
        } else {
            None
        };
        if let Some(candidate) = candidate {
            let replace = match (&candidate.timestamp, &best) {
                (Some(candidate_ts), Some(best)) => match best.timestamp {
                    Some(best_ts) => candidate_ts > &best_ts,
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
    }
    best.map(|candidate| candidate.session_id)
}

pub fn find_latest_session_id(worktree_path: &Path) -> anyhow::Result<Option<String>> {
    let desired_cwd = resolve_session_cwd(worktree_path)?;
    #[cfg(target_os = "windows")]
    let home_dir = resolve_wsl_home_dir(worktree_path)?;
    #[cfg(not(target_os = "windows"))]
    let home_dir = resolve_home_dir()?;
    let primary = home_dir.join(COPILOT_SESSION_DIR);
    let legacy = home_dir.join(COPILOT_LEGACY_SESSION_DIR);

    if let Some(session_id) = find_latest_session_in_dir(&primary, &desired_cwd) {
        return Ok(Some(session_id));
    }
    if let Some(session_id) = find_latest_session_in_dir(&legacy, &desired_cwd) {
        return Ok(Some(session_id));
    }

    Ok(None)
}
