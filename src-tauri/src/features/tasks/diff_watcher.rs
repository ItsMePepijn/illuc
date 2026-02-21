use crate::error::Result;
use crate::features::tasks::events::{emit_diff_changed, emit_review_changed};
use crate::utils::file_watcher::is_content_change_event;
use anyhow::Context;
use log::warn;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::ffi::OsStr;
use std::path::PathBuf;
use tauri::AppHandle;
use uuid::Uuid;

pub(crate) struct DiffWatcher {
    pub(crate) _watcher: RecommendedWatcher,
}

impl DiffWatcher {
    pub(crate) fn new(task_id: Uuid, path: PathBuf, app: AppHandle) -> Result<Self> {
        let mut watcher = notify::recommended_watcher(move |res| match res {
            Ok(event) => {
                if has_review_store_change(&event) {
                    emit_review_changed(&app, task_id);
                }
                if should_emit_diff_event(&event) {
                    emit_diff_changed(&app, task_id);
                }
            }
            Err(err) => {
                warn!("diff watch error task_id={} err={}", task_id, err);
            }
        })
        .with_context(|| format!("failed to create diff watcher for {}", path.display()))?;
        watcher
            .watch(&path, RecursiveMode::Recursive)
            .with_context(|| format!("failed to watch {}", path.display()))?;
        Ok(Self { _watcher: watcher })
    }
}

fn should_emit_diff_event(event: &Event) -> bool {
    is_content_change_event(event)
        && !event
            .paths
            .iter()
            .any(|path| is_review_store_path(path.as_path()))
}

fn has_review_store_change(event: &Event) -> bool {
    if !is_content_change_event(event) {
        return false;
    }
    event.paths.iter().any(|path| is_review_store_path(path.as_path()))
}

fn is_review_store_path(path: &std::path::Path) -> bool {
    path.file_name() == Some(OsStr::new("local-review.json"))
        && path.parent().and_then(std::path::Path::file_name) == Some(OsStr::new(".illuc"))
}
