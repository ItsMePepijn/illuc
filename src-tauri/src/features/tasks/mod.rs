mod agents;
mod events;
pub mod git;
pub mod management;
pub mod models;
mod repo;
pub mod review;
mod worktree;

pub use git::commands::task_git_commit::Request as CommitTaskRequest;
pub use git::commands::task_git_diff_get::Request as DiffRequest;
pub use git::commands::task_git_diff_watch_start::Request as StartDiffWatchRequest;
pub use git::commands::task_git_diff_watch_stop::Request as StopDiffWatchRequest;
pub use git::commands::task_git_has_changes::Request as HasChangesRequest;
pub use git::commands::task_git_push::Request as PushTaskRequest;
pub use management::commands::task_create::Request as CreateTaskRequest;
pub use management::commands::task_discard::Request as DiscardTaskRequest;
pub use management::commands::task_open_worktree_in_vscode::Request as OpenWorktreeInVsCodeRequest;
pub use management::commands::task_open_worktree_terminal::Request as OpenWorktreeTerminalRequest;
pub use management::commands::task_start::Request as StartTaskRequest;
pub use management::commands::task_stop::Request as StopTaskRequest;
pub use management::commands::task_terminal_resize::Request as TerminalResizeRequest;
pub use management::commands::task_terminal_start::Request as StartWorktreeTerminalRequest;
pub use management::commands::task_terminal_write::Request as TerminalWriteRequest;
pub use models::diff_payload::{DiffLine, DiffLineType};
pub use models::TerminalKind;
pub use models::{AgentKind, BaseRepoInfo, DiffPayload, TaskStatus, TaskSummary};
pub use repo::handle_select_base_repo;

use crate::error::{Result, TaskError};
use crate::features::launcher;
use crate::features::tasks::agents::codex::CodexAgent;
use crate::features::tasks::agents::copilot::CopilotAgent;
use crate::features::tasks::agents::{Agent, AgentCallbacks, AgentRuntime};
use crate::features::tasks::git::{
    add_worktree, delete_branch, get_head_branch, get_head_commit, get_repo_root, git_commit,
    git_diff, git_push, has_uncommitted_changes, list_worktrees, prune_worktrees,
    fetch_base_branch_best_effort, remove_worktree, resolve_commit_id, stage_all, validate_git_repo,
};
use crate::utils::fs::ensure_directory;
use crate::utils::path::normalize_path_string;
use crate::utils::pty::{
    wrap_portable_child, wrap_portable_master, ChildHandle, MasterHandle, TerminalSize, WriteHandle,
};
use chrono::Utc;
use events::{emit_diff_changed, emit_status, emit_terminal_exit, emit_terminal_output};
use log::warn;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::{Mutex, RwLock};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::AppHandle;
use uuid::Uuid;
use worktree::{clean_branch_name, format_title_from_branch, managed_worktree_root};

const DEFAULT_SCREEN_ROWS: usize = 40;
const DEFAULT_SCREEN_COLS: usize = 120;
const DEFAULT_PTY_ROWS: u16 = 40;
const DEFAULT_PTY_COLS: u16 = 80;

pub use git::DiffMode;

fn build_agent(agent_kind: AgentKind) -> Box<dyn Agent> {
    match agent_kind {
        AgentKind::Codex => Box::new(CodexAgent::default()),
        AgentKind::Copilot => Box::new(CopilotAgent::default()),
    }
}

fn agent_label(agent_kind: AgentKind) -> &'static str {
    match agent_kind {
        AgentKind::Codex => "Codex",
        AgentKind::Copilot => "Copilot CLI",
    }
}

fn build_worktree_shell_command(worktree_path: &Path) -> CommandBuilder {
    #[cfg(target_os = "windows")]
    {
        let mut command = CommandBuilder::new("powershell.exe");
        command.arg("-NoLogo");
        command.cwd(worktree_path);
        command
    }
    #[cfg(not(target_os = "windows"))]
    {
        let shell = std::env::var("SHELL")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "bash".to_string());
        let mut command = CommandBuilder::new(shell);
        command.cwd(worktree_path);
        command
    }
}

struct TaskRecord {
    agent: Box<dyn Agent>,
    agent_kind: AgentKind,
    summary: TaskSummary,
    runtime: Option<TaskRuntime>,
    shell: Option<TaskRuntime>,
}

struct TaskRuntime {
    child: Arc<Mutex<ChildHandle>>,
    writer: WriteHandle,
    master: MasterHandle,
}

struct DiffWatcher {
    _watcher: RecommendedWatcher,
}

#[derive(Clone, Default)]
pub struct TaskManager {
    inner: Arc<TaskManagerInner>,
}

struct TaskManagerInner {
    tasks: RwLock<HashMap<Uuid, TaskRecord>>,
    diff_watchers: Mutex<HashMap<Uuid, DiffWatcher>>,
}

impl Default for TaskManagerInner {
    fn default() -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
            diff_watchers: Mutex::new(HashMap::new()),
        }
    }
}

impl TaskManager {
    pub fn create_task(&self, req: CreateTaskRequest, app: &AppHandle) -> Result<TaskSummary> {
        let CreateTaskRequest {
            base_repo_path,
            task_title,
            base_ref,
            branch_name,
        } = req;

        let base_repo = PathBuf::from(base_repo_path);
        let repo_root = get_repo_root(&base_repo)?;
        ensure_directory(&base_repo)?;

        validate_git_repo(&base_repo)?;

        let base_ref = base_ref.unwrap_or_else(|| "HEAD".to_string());

        // Best-effort: fetch/update the base branch so the worktree is created from the latest
        // remote version. Never overwrite local commits: only fast-forward when local is behind.
        if let Err(err) = fetch_base_branch_best_effort(&repo_root, base_ref.as_str()) {
            warn!(
                "git fetch/ff-only for base ref '{}' failed: {}",
                base_ref.as_str(),
                err
            );
        }

        let base_commit = resolve_commit_id(&repo_root, base_ref.as_str())?;

        let task_id = Uuid::new_v4();
        let title = task_title.unwrap_or_else(|| format!("Task {}", task_id.simple()));
        let timestamp = Utc::now();
        let branch_name = branch_name
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| TaskError::Message("Branch name is required.".into()))?;

        let managed_root = managed_worktree_root(&repo_root)?;
        let worktree_path = managed_root.join(task_id.to_string());

        if worktree_path.exists() {
            if let Err(err) = std::fs::remove_dir_all(&worktree_path) {
                warn!(
                    "failed to remove pre-existing worktree path {}: {}",
                    worktree_path.display(),
                    err
                );
            }
        }

        let worktree_path_display = normalize_path_string(&worktree_path);
        let worktree_name = task_id.simple().to_string();
        add_worktree(
            &repo_root,
            branch_name.as_str(),
            &worktree_path,
            base_ref.as_str(),
            worktree_name.as_str(),
        )?;

        let summary = TaskSummary {
            task_id,
            title,
            status: TaskStatus::Stopped,
            agent_kind: AgentKind::Codex,
            created_at: timestamp,
            started_at: None,
            ended_at: None,
            worktree_path: worktree_path_display,
            branch_name,
            base_branch: base_ref.clone(),
            base_repo_path: normalize_path_string(&repo_root),
            base_commit,
            exit_code: None,
        };

        let mut tasks = self.inner.tasks.write();
        tasks.insert(
            task_id,
            TaskRecord {
                agent: build_agent(AgentKind::Codex),
                agent_kind: AgentKind::Codex,
                summary: summary.clone(),
                runtime: None,
                shell: None,
            },
        );
        drop(tasks);
        emit_status(app, &summary);
        Ok(summary)
    }

    pub fn start_task(&self, req: StartTaskRequest, app: &AppHandle) -> Result<TaskSummary> {
        let StartTaskRequest {
            task_id,
            cols,
            rows,
            agent,
        } = req;
        let requested_rows = rows.filter(|value| *value > 0);
        let requested_cols = cols.filter(|value| *value > 0);
        let screen_rows = requested_rows
            .map(|value| value as usize)
            .unwrap_or(DEFAULT_SCREEN_ROWS);
        let screen_cols = requested_cols
            .map(|value| value as usize)
            .unwrap_or(DEFAULT_SCREEN_COLS);
        let pty_rows = requested_rows.unwrap_or(DEFAULT_PTY_ROWS);
        let pty_cols = requested_cols.unwrap_or(DEFAULT_PTY_COLS);
        {
            let tasks = self.inner.tasks.read();
            let record = tasks.get(&task_id).ok_or(TaskError::NotFound)?;
            if record.runtime.is_some() {
                return Err(TaskError::AlreadyRunning);
            }
        }

        let (worktree_path, title, _has_started) = {
            let tasks = self.inner.tasks.read();
            let record = tasks.get(&task_id).ok_or(TaskError::NotFound)?;
            (
                PathBuf::from(&record.summary.worktree_path),
                record.summary.title.clone(),
                record.summary.started_at.is_some(),
            )
        };

        let status_manager = self.clone();
        let status_app = app.clone();
        let output_manager = self.clone();
        let output_app = app.clone();
        let exit_manager = self.clone();
        let exit_app = app.clone();
        let callbacks = AgentCallbacks {
            on_output: Arc::new(move |chunk: String| {
                output_manager.handle_agent_output(task_id, chunk, &output_app);
            }),
            on_status: Arc::new(move |status: TaskStatus| {
                status_manager.handle_agent_status(task_id, status, &status_app);
            }),
            on_exit: Arc::new(move |exit_code: i32| {
                exit_manager.handle_agent_exit(task_id, exit_code, &exit_app);
            }),
        };

        let agent_runtime = {
            let mut tasks = self.inner.tasks.write();
            let record = tasks.get_mut(&task_id).ok_or(TaskError::NotFound)?;
            if let Some(requested_agent) = agent {
                record.agent_kind = requested_agent;
                record.agent = build_agent(requested_agent);
            }
            record.summary.agent_kind = record.agent_kind;
            let label = agent_label(record.agent_kind);
            record.agent.reset(screen_rows, screen_cols);
            record
                .agent
                .start(&worktree_path, callbacks, pty_rows, pty_cols)
                .with_context(|| format!("failed to start {} for task {}", label, title))?
        };

        let AgentRuntime {
            child,
            writer,
            master,
        } = agent_runtime;

        {
            let mut tasks = self.inner.tasks.write();
            let record = tasks.get_mut(&task_id).ok_or(TaskError::NotFound)?;
            record.summary.status = TaskStatus::Idle;
            record.summary.started_at = Some(Utc::now());
            record.summary.exit_code = None;
            record.runtime = Some(TaskRuntime {
                child: child.clone(),
                writer: writer.clone(),
                master: master.clone(),
            });
            emit_status(app, &record.summary);
        }

        let tasks = self.inner.tasks.read();
        let record = tasks.get(&task_id).ok_or(TaskError::NotFound)?;
        Ok(record.summary.clone())
    }

    pub fn stop_task(&self, req: StopTaskRequest, app: &AppHandle) -> Result<TaskSummary> {
        let task_id = req.task_id;
        let child = {
            let tasks = self.inner.tasks.read();
            let record = tasks.get(&task_id).ok_or(TaskError::NotFound)?;
            if let Some(runtime) = &record.runtime {
                runtime.child.clone()
            } else {
                return Ok(record.summary.clone());
            }
        };

        if let Some(mut child_guard) = child.try_lock() {
            if let Err(err) = child_guard.kill() {
                warn!("failed to kill task process for {}: {}", task_id, err);
            }
        }

        {
            let mut tasks = self.inner.tasks.write();
            let record = tasks.get_mut(&task_id).ok_or(TaskError::NotFound)?;
            record.summary.status = TaskStatus::Stopped;
            emit_status(app, &record.summary);
            return Ok(record.summary.clone());
        }
    }

    pub fn discard_task(&self, req: DiscardTaskRequest, app: &AppHandle) -> Result<()> {
        let task_id = req.task_id;
        self.remove_diff_watch(task_id);
        let (worktree_path, branch_name, base_repo_path, runtime_exists, shell_exists) = {
            let tasks = self.inner.tasks.read();
            let record = tasks.get(&task_id).ok_or(TaskError::NotFound)?;
            (
                PathBuf::from(&record.summary.worktree_path),
                record.summary.branch_name.clone(),
                PathBuf::from(&record.summary.base_repo_path),
                record.runtime.is_some(),
                record.shell.is_some(),
            )
        };

        if runtime_exists {
            if let Err(err) = self.stop_task(StopTaskRequest { task_id }, app) {
                warn!("failed to stop task {} during discard: {}", task_id, err);
            }
        }
        if shell_exists {
            let mut tasks = self.inner.tasks.write();
            if let Some(record) = tasks.get_mut(&task_id) {
                if let Some(shell) = record.shell.take() {
                    if let Some(mut child_guard) = shell.child.try_lock() {
                        if let Err(err) = child_guard.kill() {
                            warn!("failed to kill worktree shell for {}: {}", task_id, err);
                        }
                    }
                }
            }
        }

        if let Err(err) = remove_worktree(&base_repo_path, &worktree_path) {
            warn!(
                "failed to remove worktree {} for {}: {}",
                worktree_path.display(),
                task_id,
                err
            );
        }
        if let Err(err) = delete_branch(&base_repo_path, branch_name.as_str()) {
            warn!(
                "failed to delete branch {} for {}: {}",
                branch_name,
                task_id,
                err
            );
        }
        if worktree_path.exists() {
            if let Err(err) = std::fs::remove_dir_all(&worktree_path) {
                warn!(
                    "failed to remove worktree directory {} for {}: {}",
                    worktree_path.display(),
                    task_id,
                    err
                );
            }
        }

        {
            let mut tasks = self.inner.tasks.write();
            if let Some(record) = tasks.get_mut(&task_id) {
                record.summary.status = TaskStatus::Discarded;
                record.runtime = None;
                emit_status(app, &record.summary);
            }
        }

        let mut tasks = self.inner.tasks.write();
        tasks.remove(&task_id);
        Ok(())
    }

    pub fn terminal_write(&self, req: TerminalWriteRequest) -> Result<()> {
        let task_id = req.task_id;
        let writer = {
            let tasks = self.inner.tasks.read();
            let record = tasks.get(&task_id).ok_or(TaskError::NotFound)?;
            match &record.runtime {
                Some(runtime) => runtime.writer.clone(),
                None => return Err(TaskError::NotRunning),
            }
        };
        let mut writer_guard = writer.lock();
        writer_guard
            .write_all(req.data.as_bytes())
            .with_context(|| "failed to write to terminal")?;
        if let Err(err) = writer_guard.flush() {
            warn!("failed to flush terminal input for {}: {}", task_id, err);
        }
        Ok(())
    }

    pub fn terminal_resize(&self, req: TerminalResizeRequest) -> Result<()> {
        let task_id = req.task_id;
        let master = {
            let tasks = self.inner.tasks.read();
            let record = tasks.get(&task_id).ok_or(TaskError::NotFound)?;
            match &record.runtime {
                Some(runtime) => runtime.master.clone(),
                None => return Err(TaskError::NotRunning),
            }
        };
        master
            .lock()
            .resize(TerminalSize {
                cols: req.cols,
                rows: req.rows,
            })
            .with_context(|| "failed to resize terminal")?;
        {
            let mut tasks = self.inner.tasks.write();
            if let Some(record) = tasks.get_mut(&task_id) {
                record.agent.resize(req.rows as usize, req.cols as usize);
            }
        }
        Ok(())
    }

    pub fn start_worktree_terminal(
        &self,
        req: StartWorktreeTerminalRequest,
        app: &AppHandle,
    ) -> Result<()> {
        let task_id = req.task_id;
        {
            let tasks = self.inner.tasks.read();
            let record = tasks.get(&task_id).ok_or(TaskError::NotFound)?;
            if record.shell.is_some() {
                return Ok(());
            }
        }

        let worktree_path = self.worktree_path(task_id)?;
        let rows = req.rows.unwrap_or(DEFAULT_PTY_ROWS).max(1);
        let cols = req.cols.unwrap_or(DEFAULT_PTY_COLS).max(1);
        let runtime =
            self.spawn_worktree_shell(task_id, worktree_path.as_path(), rows, cols, app)?;

        let mut tasks = self.inner.tasks.write();
        let record = tasks.get_mut(&task_id).ok_or(TaskError::NotFound)?;
        if record.shell.is_none() {
            record.shell = Some(runtime);
        }
        Ok(())
    }

    pub fn worktree_terminal_write(&self, req: TerminalWriteRequest) -> Result<()> {
        let task_id = req.task_id;
        let writer = {
            let tasks = self.inner.tasks.read();
            let record = tasks.get(&task_id).ok_or(TaskError::NotFound)?;
            match &record.shell {
                Some(runtime) => runtime.writer.clone(),
                None => return Err(TaskError::NotRunning),
            }
        };
        let mut writer_guard = writer.lock();
        writer_guard
            .write_all(req.data.as_bytes())
            .with_context(|| "failed to write to worktree terminal")?;
        if let Err(err) = writer_guard.flush() {
            warn!("failed to flush worktree terminal input for {}: {}", task_id, err);
        }
        Ok(())
    }

    pub fn worktree_terminal_resize(&self, req: TerminalResizeRequest) -> Result<()> {
        let task_id = req.task_id;
        let master = {
            let tasks = self.inner.tasks.read();
            let record = tasks.get(&task_id).ok_or(TaskError::NotFound)?;
            match &record.shell {
                Some(runtime) => runtime.master.clone(),
                None => return Err(TaskError::NotRunning),
            }
        };
        master
            .lock()
            .resize(TerminalSize {
                cols: req.cols,
                rows: req.rows,
            })
            .with_context(|| "failed to resize worktree terminal")?;
        Ok(())
    }

    pub fn get_diff(&self, req: DiffRequest) -> Result<DiffPayload> {
        let task_id = req.task_id;
        let (worktree_path, base_commit) = {
            let tasks = self.inner.tasks.read();
            let record = tasks.get(&task_id).ok_or(TaskError::NotFound)?;
            (
                PathBuf::from(&record.summary.worktree_path),
                record.summary.base_commit.trim().to_string(),
            )
        };

        if let Err(err) = stage_all(worktree_path.as_path()) {
            warn!(
                "failed to stage files before diff for task {} at {}: {}",
                task_id,
                worktree_path.display(),
                err
            );
        }

        let whitespace_flag = if req.ignore_whitespace.unwrap_or(false) {
            Some("--ignore-all-space")
        } else {
            None
        };
        let mode = req.mode.unwrap_or(DiffMode::Worktree);
        match mode {
            DiffMode::Worktree => {
                let combined = git_diff(worktree_path.as_path(), "HEAD", whitespace_flag)?;
                Ok(DiffPayload {
                    task_id,
                    files: combined.files,
                })
            }
            DiffMode::Branch => {
                let branch_diff = git_diff(
                    worktree_path.as_path(),
                    base_commit.as_str(),
                    whitespace_flag,
                )?;
                Ok(DiffPayload {
                    task_id,
                    files: branch_diff.files,
                })
            }
        }
    }

    pub fn has_uncommitted_changes(&self, req: HasChangesRequest) -> Result<bool> {
        let path = self.worktree_path(req.task_id)?;
        has_uncommitted_changes(path.as_path())
    }

    pub fn start_diff_watch(&self, req: StartDiffWatchRequest, app: &AppHandle) -> Result<()> {
        let task_id = req.task_id;
        let worktree_path = self.worktree_path(task_id)?;
        let mut watchers = self.inner.diff_watchers.lock();
        if watchers.contains_key(&task_id) {
            return Ok(());
        }
        let watcher = DiffWatcher::new(task_id, worktree_path, app.clone())?;
        watchers.insert(task_id, watcher);
        Ok(())
    }

    pub fn stop_diff_watch(&self, req: StopDiffWatchRequest) -> Result<()> {
        self.remove_diff_watch(req.task_id);
        Ok(())
    }

    pub fn commit_task(&self, req: CommitTaskRequest, app: &AppHandle) -> Result<()> {
        let task_id = req.task_id;
        let message = req.message.trim();
        if message.is_empty() {
            return Err(TaskError::Message("Commit message is required.".into()));
        }
        let stage_all = req.stage_all.unwrap_or(true);
        let worktree_path = self.worktree_path(task_id)?;
        git_commit(worktree_path.as_path(), message, stage_all)?;
        emit_diff_changed(app, task_id);
        Ok(())
    }

    pub fn push_task(&self, req: PushTaskRequest, app: &AppHandle) -> Result<()> {
        let task_id = req.task_id;
        let (worktree_path, branch_name) = {
            let tasks = self.inner.tasks.read();
            let record = tasks.get(&task_id).ok_or(TaskError::NotFound)?;
            (
                PathBuf::from(&record.summary.worktree_path),
                record.summary.branch_name.clone(),
            )
        };
        let remote = req.remote.unwrap_or_else(|| "origin".to_string());
        let branch = req.branch.unwrap_or(branch_name);
        let set_upstream = req.set_upstream.unwrap_or(true);
        git_push(
            worktree_path.as_path(),
            remote.as_str(),
            branch.as_str(),
            set_upstream,
        )?;
        emit_diff_changed(app, task_id);
        Ok(())
    }

    fn apply_agent_status(&self, record: &mut TaskRecord, status: TaskStatus, app: &AppHandle) {
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
                task_id,
                exit_code,
                err
            );
        }
        emit_terminal_exit(app, task_id, exit_code, TerminalKind::Agent);
    }

    fn handle_worktree_terminal_exit(&self, task_id: Uuid, exit_code: i32, app: &AppHandle) {
        let mut tasks = self.inner.tasks.write();
        if let Some(record) = tasks.get_mut(&task_id) {
            record.shell = None;
        }
        emit_terminal_exit(app, task_id, exit_code, TerminalKind::Worktree);
    }

    fn spawn_worktree_shell(
        &self,
        task_id: Uuid,
        worktree_path: &Path,
        rows: u16,
        cols: u16,
        app: &AppHandle,
    ) -> Result<TaskRuntime> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let master = pair.master;
        let writer = master
            .take_writer()
            .context("failed to obtain worktree terminal writer")?;
        let reader = master
            .try_clone_reader()
            .context("failed to clone worktree terminal reader")?;
        let master = wrap_portable_master(master);
        let writer = Arc::new(Mutex::new(writer));

        let command = build_worktree_shell_command(worktree_path);
        let child = pair
            .slave
            .spawn_command(command)
            .context("failed to start worktree terminal")?;
        let child = wrap_portable_child(child);

        let output_app = app.clone();
        std::thread::spawn(move || {
            let mut reader = reader;
            let mut buffer = [0u8; 8192];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(size) => {
                        let chunk = String::from_utf8_lossy(&buffer[..size]).to_string();
                        emit_terminal_output(&output_app, task_id, chunk, TerminalKind::Worktree);
                    }
                    Err(err) => {
                        warn!(
                            "worktree terminal read failed for task {}: {}",
                            task_id, err
                        );
                        break;
                    }
                }
            }
        });

        let exit_manager = self.clone();
        let exit_app = app.clone();
        let exit_child = child.clone();
        std::thread::spawn(move || {
            let exit_code = loop {
                {
                    let mut child_guard = exit_child.lock();
                    match child_guard.try_wait() {
                        Ok(Some(status)) => {
                            let code = status.exit_code() as i32;
                            break if status.success() { 0 } else { code };
                        }
                        Ok(None) => {}
                        Err(err) => {
                            warn!("worktree terminal wait failed for task {}: {}", task_id, err);
                            break 1;
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            };
            exit_manager.handle_worktree_terminal_exit(task_id, exit_code, &exit_app);
        });

        Ok(TaskRuntime {
            child,
            writer,
            master,
        })
    }

    fn contains_worktree_path(&self, path: &Path) -> bool {
        let target = normalize_path_string(path);
        self.inner
            .tasks
            .read()
            .values()
            .any(|record| record.summary.worktree_path == target)
    }

    pub fn register_existing_worktrees(
        &self,
        base_repo_path: String,
        app: &AppHandle,
    ) -> Result<Vec<TaskSummary>> {
        let provided_path = PathBuf::from(&base_repo_path);
        ensure_directory(&provided_path)?;
        validate_git_repo(&provided_path)?;
        let repo_root = get_repo_root(&provided_path)?
            .canonicalize()
            .unwrap_or_else(|err| {
                warn!(
                    "failed to canonicalize repo root {}; using provided path: {}",
                    provided_path.display(),
                    err
                );
                provided_path.clone()
            });
        if let Err(err) = prune_worktrees(&repo_root) {
            warn!("worktree prune failed: {}", err);
        }
        let managed_root = managed_worktree_root(&repo_root)?;
        let base_repo_head = get_head_commit(&repo_root)?;
        let base_repo_branch = get_head_branch(&repo_root).unwrap_or_else(|err| {
            warn!(
                "failed to resolve base repo branch for {}; defaulting to HEAD: {}",
                repo_root.display(),
                err
            );
            "HEAD".to_string()
        });
        let entries = list_worktrees(&repo_root)?;
        let mut inserted = Vec::new();
        for entry in entries {
            let canonical_path = entry
                .path
                .canonicalize()
                .unwrap_or_else(|err| {
                    warn!(
                        "failed to canonicalize worktree path {}; using raw path: {}",
                        entry.path.display(),
                        err
                    );
                    entry.path.clone()
                });
            if canonical_path == repo_root {
                continue;
            }
            if !canonical_path.starts_with(&managed_root) {
                continue;
            }
            if self.contains_worktree_path(&canonical_path) {
                continue;
            }
            let worktree_path_display = normalize_path_string(&canonical_path);
            let task_id = canonical_path
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| Uuid::parse_str(name).ok())
                .unwrap_or_else(Uuid::new_v4);
            let branch_name = entry
                .branch
                .as_ref()
                .map(|name| clean_branch_name(name))
                .unwrap_or_else(|| {
                    let short_head: String = entry.head.chars().take(7).collect();
                    format!("detached-{}", short_head)
                });
            let summary = TaskSummary {
                task_id,
                title: format_title_from_branch(&branch_name),
                status: TaskStatus::Stopped,
                agent_kind: AgentKind::Codex,
                created_at: Utc::now(),
                started_at: None,
                ended_at: None,
                worktree_path: worktree_path_display,
                branch_name,
                base_branch: base_repo_branch.clone(),
                base_repo_path: normalize_path_string(&repo_root),
                base_commit: base_repo_head.clone(),
                exit_code: None,
            };
            self.inner.tasks.write().insert(
                summary.task_id,
                TaskRecord {
                    agent: build_agent(AgentKind::Codex),
                    agent_kind: AgentKind::Codex,
                    summary: summary.clone(),
                    runtime: None,
                    shell: None,
                },
            );
            emit_status(app, &summary);
            inserted.push(summary);
        }
        Ok(inserted)
    }

    fn worktree_path(&self, task_id: Uuid) -> Result<PathBuf> {
        let tasks = self.inner.tasks.read();
        let record = tasks.get(&task_id).ok_or(TaskError::NotFound)?;
        Ok(PathBuf::from(&record.summary.worktree_path))
    }

    fn remove_diff_watch(&self, task_id: Uuid) {
        let mut watchers = self.inner.diff_watchers.lock();
        watchers.remove(&task_id);
    }

    pub fn open_in_vscode(&self, req: OpenWorktreeInVsCodeRequest) -> Result<()> {
        let path = self.worktree_path(req.task_id)?;
        launcher::open_path_in_vscode(path.as_path())
    }

    pub fn open_terminal(&self, req: OpenWorktreeTerminalRequest) -> Result<()> {
        let path = self.worktree_path(req.task_id)?;
        launcher::open_path_terminal(path.as_path())
    }

    fn finish_task(&self, task_id: Uuid, exit_code: i32, app: &AppHandle) -> Result<()> {
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

use anyhow::Context;

impl DiffWatcher {
    fn new(task_id: Uuid, path: PathBuf, app: AppHandle) -> Result<Self> {
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
