mod resuming;

use crate::features::tasks::agents::{Agent, AgentCallbacks, TuiAgent};
use crate::features::tasks::TaskStatus;
use crate::utils::pty::{
    wrap_portable_child, wrap_portable_master, ChildHandle, MasterHandle, WriteHandle,
};
use crate::utils::screen::Screen;
#[cfg(target_os = "windows")]
use crate::utils::windows::build_wsl_command;
use anyhow::Context;
use git2::Repository;
use log::warn;
use parking_lot::Mutex;
#[cfg(not(target_os = "windows"))]
use portable_pty::CommandBuilder;
use portable_pty::{native_pty_system, PtySize};
use resuming::find_latest_session_id;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const DEFAULT_ROWS: u16 = 40;
const DEFAULT_COLS: u16 = 80;
const OPENCODE_TUI_CONFIG_FILE: &str = "tui.json";
const OPENCODE_TUI_CONFIG_CONTENT: &str = "{\n  \"theme\": \"system\"\n}\n";
const OPENCODE_GIT_EXCLUDE_ENTRY: &str = "/tui.json";
const OPENCODE_SIDEBAR_MARKER: &str = "• OpenCode";
const OPENCODE_SIDEBAR_TOGGLE: &[u8] = b"\x18b";
const OPENCODE_APPROVAL_MARKER: &str = "Permission required";

#[derive(Clone)]
pub struct OpenCodeAgent {
    state: Arc<Mutex<OpenCodeAgentState>>,
}

struct OpenCodeAgentState {
    screen: Screen,
    last_output: Option<Instant>,
    last_status: Option<TaskStatus>,
    sidebar_close_attempted: bool,
    child: Option<Arc<Mutex<ChildHandle>>>,
    writer: Option<WriteHandle>,
    master: Option<MasterHandle>,
}

impl Default for OpenCodeAgent {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(OpenCodeAgentState {
                screen: Screen::new(DEFAULT_ROWS as usize, DEFAULT_COLS as usize),
                last_output: None,
                last_status: None,
                sidebar_close_attempted: false,
                child: None,
                writer: None,
                master: None,
            })),
        }
    }
}

impl OpenCodeAgent {
    fn ensure_project_config(worktree_path: &Path) -> anyhow::Result<()> {
        let config_path = worktree_path.join(OPENCODE_TUI_CONFIG_FILE);
        write_project_config(&config_path)?;
        ensure_git_exclude_entry(worktree_path, OPENCODE_GIT_EXCLUDE_ENTRY)?;
        Ok(())
    }

    fn status_from_output(&self, raw: &[u8], timestamp: Instant) -> Option<TaskStatus> {
        let mut state = self.state.lock();
        state.last_output = Some(timestamp);
        state.screen.process(raw);
        let status = if screen_has_approval_prompt(&state.screen.full_text()) {
            TaskStatus::AwaitingApproval
        } else {
            TaskStatus::Working
        };
        let status_changed = state.last_status != Some(status);
        if status_changed {
            state.last_status = Some(status);
        }
        if status_changed {
            Some(status)
        } else {
            None
        }
    }

    fn should_close_sidebar(&self) -> bool {
        let mut state = self.state.lock();
        if state.sidebar_close_attempted {
            return false;
        }
        if state.screen.full_text().contains(OPENCODE_SIDEBAR_MARKER) {
            state.sidebar_close_attempted = true;
            return true;
        }
        false
    }

    fn toggle_sidebar(writer: &crate::utils::pty::WriteHandle) {
        let mut writer_guard = writer.lock();
        if let Err(error) = writer_guard.write_all(OPENCODE_SIDEBAR_TOGGLE) {
            warn!("failed to close opencode sidebar: {}", error);
        } else if let Err(error) = writer_guard.flush() {
            warn!("failed to flush opencode sidebar toggle: {}", error);
        }
    }

    fn status_if_idle(&self, now: Instant) -> Option<TaskStatus> {
        let mut state = self.state.lock();
        let last = state.last_output?;
        if now.duration_since(last) >= Duration::from_millis(1000)
            && matches!(
                state.last_status,
                Some(TaskStatus::Working | TaskStatus::AwaitingApproval)
            )
        {
            state.last_status = Some(TaskStatus::Idle);
            return Some(TaskStatus::Idle);
        }
        None
    }
}

impl Agent for OpenCodeAgent {
    fn as_tui_agent_mut(&mut self) -> Option<&mut dyn TuiAgent> {
        Some(self)
    }

    fn is_running(&self) -> bool {
        self.state.lock().child.is_some()
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        let child = {
            let state = self.state.lock();
            state.child.clone()
        }
        .context("opencode is not running")?;
        let mut child = child.lock();
        child.kill()
    }
}

impl TuiAgent for OpenCodeAgent {
    fn start_tui(
        &mut self,
        worktree_path: &Path,
        callbacks: AgentCallbacks,
        rows: u16,
        cols: u16,
    ) -> anyhow::Result<()> {
        let pty_system = native_pty_system();
        Self::ensure_project_config(worktree_path)?;
        let latest_session_id = find_latest_session_id(worktree_path)?;
        let rows = rows.max(1);
        let cols = cols.max(1);
        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let master = pair.master;
        let writer = master
            .take_writer()
            .context("failed to obtain pty writer")?;
        let reader = master
            .try_clone_reader()
            .context("failed to clone pty reader")?;
        let master = wrap_portable_master(master);
        let writer = Arc::new(Mutex::new(writer));

        #[cfg(target_os = "windows")]
        let command = {
            let args: Vec<&str> = if let Some(session_id) = latest_session_id.as_deref() {
                vec!["--session", session_id]
            } else {
                vec![]
            };
            build_wsl_command(worktree_path, "opencode", &args)
        };

        #[cfg(not(target_os = "windows"))]
        let command = {
            let mut command = CommandBuilder::new("opencode");
            if let Some(session_id) = latest_session_id.as_deref() {
                command.args(["--session", session_id]);
            }
            command.cwd(worktree_path);
            command
        };

        let child = pair
            .slave
            .spawn_command(command)
            .context("failed to start Opencode")?;
        let child = wrap_portable_child(child);

        let status_handle = self.clone();
        let output_callbacks = callbacks.clone();
        let running = Arc::new(AtomicBool::new(true));
        let output_writer = writer.clone();
        let idle_running = Arc::clone(&running);
        let idle_handle = self.clone();
        let idle_callbacks = callbacks.clone();
        std::thread::spawn(move || {
            while idle_running.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(250));
                if let Some(status) = idle_handle.status_if_idle(Instant::now()) {
                    (idle_callbacks.on_status)(status);
                }
            }
        });

        std::thread::spawn(move || {
            let mut reader = reader;
            let mut buffer = [0u8; 8192];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(size) => {
                        let now = Instant::now();
                        let chunk = String::from_utf8_lossy(&buffer[..size]).to_string();
                        if let Some(status) = status_handle.status_from_output(&buffer[..size], now)
                        {
                            (output_callbacks.on_status)(status);
                        }
                        if status_handle.should_close_sidebar() {
                            Self::toggle_sidebar(&output_writer);
                        }
                        (output_callbacks.on_output)(chunk);
                    }
                    Err(error) => {
                        warn!("opencode PTY read failed: {}", error);
                        break;
                    }
                }
            }
        });

        let exit_callbacks = callbacks.clone();
        let exit_child = child.clone();
        let exit_state = Arc::clone(&self.state);
        let exit_running = Arc::clone(&running);
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
                        Err(error) => {
                            warn!("opencode process wait failed: {}", error);
                            break 1;
                        }
                    }
                }
                std::thread::sleep(Duration::from_millis(200));
            };
            exit_running.store(false, Ordering::Relaxed);
            let mut state = exit_state.lock();
            state.child = None;
            state.writer = None;
            state.master = None;
            (exit_callbacks.on_exit)(exit_code);
        });

        let mut state = self.state.lock();
        state.child = Some(child);
        state.writer = Some(writer);
        state.master = Some(master);
        Ok(())
    }

    fn reset(&mut self, rows: usize, cols: usize) {
        let mut state = self.state.lock();
        state.screen = Screen::new(rows, cols);
        state.last_output = None;
        state.last_status = None;
        state.sidebar_close_attempted = false;
    }

    fn resize(&mut self, rows: usize, cols: usize) {
        let mut state = self.state.lock();
        state.screen.resize(rows, cols);
        if let Some(master) = &state.master {
            let _ = master.lock().resize(crate::utils::pty::TerminalSize {
                cols: cols as u16,
                rows: rows as u16,
            });
        }
    }

    fn write(&mut self, data: &[u8]) -> anyhow::Result<()> {
        let writer = {
            let state = self.state.lock();
            state.writer.clone()
        }
        .context("opencode terminal is not running")?;
        let mut writer = writer.lock();
        writer.write_all(data)?;
        writer.flush()?;
        Ok(())
    }
}

fn write_project_config(path: &Path) -> anyhow::Result<()> {
    let should_write = match fs::read_to_string(path) {
        Ok(contents) => contents != OPENCODE_TUI_CONFIG_CONTENT,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
        Err(error) => {
            return Err(error).with_context(|| format!("failed reading {}", path.display()));
        }
    };

    if should_write {
        fs::write(path, OPENCODE_TUI_CONFIG_CONTENT)
            .with_context(|| format!("failed writing {}", path.display()))?;
    }

    Ok(())
}

fn ensure_git_exclude_entry(worktree_path: &Path, entry: &str) -> anyhow::Result<()> {
    let repo = Repository::discover(worktree_path).with_context(|| {
        format!(
            "failed to open git repository for {}",
            worktree_path.display()
        )
    })?;
    let exclude_path = repo.commondir().join("info").join("exclude");
    if let Some(parent) = exclude_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed creating {}", parent.display()))?;
    }
    let existing = match fs::read_to_string(&exclude_path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("failed reading {}", exclude_path.display()));
        }
    };

    if existing.lines().any(|line| line.trim() == entry) {
        return Ok(());
    }

    let mut updated = existing;
    if !updated.is_empty() && !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(entry);
    updated.push('\n');
    fs::write(&exclude_path, updated)
        .with_context(|| format!("failed writing {}", exclude_path.display()))?;
    Ok(())
}

fn screen_has_approval_prompt(screen_text: &str) -> bool {
    let mut search_from = 0;
    while let Some(relative_index) = screen_text[search_from..].find(OPENCODE_APPROVAL_MARKER) {
        let marker_index = search_from + relative_index;
        let prefix = &screen_text[..marker_index];
        let trailing_whitespace_len: usize = prefix
            .chars()
            .rev()
            .take_while(|ch| ch.is_whitespace())
            .map(char::len_utf8)
            .sum();
        let triangle_end = prefix.len().saturating_sub(trailing_whitespace_len);
        if prefix[..triangle_end].ends_with('△') {
            return true;
        }
        search_from = marker_index + OPENCODE_APPROVAL_MARKER.len();
    }
    false
}
