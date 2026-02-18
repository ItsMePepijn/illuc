use super::*;
use crate::error::{Result, TaskError};
use crate::features::tasks::agents::Agent;
use crate::features::tasks::events::{emit_status, emit_terminal_exit, emit_terminal_output};
use crate::utils::path::normalize_path_string;
use crate::utils::pty::{ChildHandle, MasterHandle, WriteHandle};
use chrono::Utc;
use log::warn;
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::AppHandle;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct TaskManager {
    pub(crate) inner: Arc<TaskManagerInner>,
}

pub(crate) struct TaskManagerInner {
    pub(crate) tasks: RwLock<HashMap<Uuid, TaskRecord>>,
    pub(crate) diff_watchers: Mutex<HashMap<Uuid, DiffWatcher>>,
}

impl Default for TaskManagerInner {
    fn default() -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
            diff_watchers: Mutex::new(HashMap::new()),
        }
    }
}

pub(crate) struct TaskRecord {
    pub(crate) agent: Box<dyn Agent>,
    pub(crate) agent_kind: AgentKind,
    pub(crate) summary: TaskSummary,
    pub(crate) runtime: Option<TaskRuntime>,
    pub(crate) shell: Option<TaskRuntime>,
}

pub(crate) struct TaskRuntime {
    pub(crate) child: Arc<Mutex<ChildHandle>>,
    pub(crate) writer: WriteHandle,
    pub(crate) master: MasterHandle,
}

impl TaskManager {
    pub(crate) fn apply_agent_status(
        &self,
        record: &mut TaskRecord,
        status: TaskStatus,
        app: &AppHandle,
    ) {
        if record.summary.status != status {
            record.summary.status = status;
            emit_status(app, &record.summary);
        }
    }

    pub fn handle_agent_status(&self, task_id: Uuid, status: TaskStatus, app: &AppHandle) {
        let mut tasks = self.inner.tasks.write();
        if let Some(record) = tasks.get_mut(&task_id) {
            if record.runtime.is_none() {
                return;
            }
            if matches!(
                record.summary.status,
                TaskStatus::Stopped
                    | TaskStatus::Discarded
                    | TaskStatus::Completed
                    | TaskStatus::Failed
            ) {
                return;
            }
            self.apply_agent_status(record, status, app);
        }
    }

    pub fn handle_agent_output(&self, task_id: Uuid, chunk: String, app: &AppHandle) {
        emit_terminal_output(app, task_id, chunk, TerminalKind::Agent);
    }

    pub fn handle_agent_exit(&self, task_id: Uuid, exit_code: i32, app: &AppHandle) {
        if let Err(err) = self.finish_task(task_id, exit_code, app) {
            warn!(
                "failed to finalize task {} after agent exit (code {}): {}",
                task_id, exit_code, err
            );
        }
        emit_terminal_exit(app, task_id, exit_code, TerminalKind::Agent);
    }

    pub(crate) fn contains_worktree_path(&self, path: &Path) -> bool {
        let target = normalize_path_string(path);
        self.inner
            .tasks
            .read()
            .values()
            .any(|record| record.summary.worktree_path == target)
    }

    pub(crate) fn worktree_path(&self, task_id: Uuid) -> Result<PathBuf> {
        let tasks = self.inner.tasks.read();
        let record = tasks.get(&task_id).ok_or(TaskError::NotFound)?;
        Ok(PathBuf::from(&record.summary.worktree_path))
    }

    pub(crate) fn remove_diff_watch(&self, task_id: Uuid) {
        let mut watchers = self.inner.diff_watchers.lock();
        watchers.remove(&task_id);
    }

    pub(crate) fn finish_task(&self, task_id: Uuid, exit_code: i32, app: &AppHandle) -> Result<()> {
        let mut tasks = self.inner.tasks.write();
        let record = tasks.get_mut(&task_id).ok_or(TaskError::NotFound)?;
        if record.runtime.is_none() {
            warn!("finish_task task_id={} without runtime", task_id);
        }
        record.summary.exit_code = Some(exit_code);
        record.summary.ended_at = Some(Utc::now());
        record.runtime = None;
        let target_status = match record.summary.status {
            TaskStatus::Stopped => TaskStatus::Stopped,
            TaskStatus::Discarded => TaskStatus::Discarded,
            _ if exit_code == 0 => TaskStatus::Completed,
            _ => TaskStatus::Failed,
        };
        record.summary.status = target_status;
        emit_status(app, &record.summary);
        Ok(())
    }
}
