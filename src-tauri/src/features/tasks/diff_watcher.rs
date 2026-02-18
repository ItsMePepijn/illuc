use crate::error::Result;
use crate::features::tasks::events::emit_diff_changed;
use anyhow::Context;
use log::warn;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
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
    matches!(
        event.kind,
        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
    )
}
