use crate::agents::{Agent, AgentCallbacks, AgentRuntime, ChildHandle};
use crate::tasks::TaskStatus;
use crate::utils::screen::{Screen, ScreenPerformer};
use anyhow::Context;
use parking_lot::Mutex;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use vte::Parser;

const DEFAULT_ROWS: u16 = 40;
const DEFAULT_COLS: u16 = 120;
const APPROVAL_PROMPT: &str = "would you like to run the following command";

#[derive(Clone)]
pub struct CodexAgent {
    state: Arc<Mutex<CodexAgentState>>,
}

struct CodexAgentState {
    screen: Screen,
    parser: Parser,
    last_output: Option<Instant>,
    last_status: Option<TaskStatus>,
    prompt_active: bool,
}

impl Default for CodexAgent {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(CodexAgentState {
                screen: Screen::new(DEFAULT_ROWS as usize, DEFAULT_COLS as usize),
                parser: Parser::new(),
                last_output: None,
                last_status: None,
                prompt_active: false,
            })),
        }
    }
}

impl CodexAgent {
    fn status_from_output(&self, raw: &[u8], timestamp: Instant) -> Option<TaskStatus> {
        let mut state = self.state.lock();
        state.last_output = Some(timestamp);
        let CodexAgentState { screen, parser, .. } = &mut *state;
        let mut performer = ScreenPerformer::new(screen);
        for byte in raw {
            parser.advance(&mut performer, *byte);
        }
        let prompt_now = screen
            .full_text()
            .to_ascii_lowercase()
            .contains(APPROVAL_PROMPT);
        state.prompt_active = prompt_now;
        let status = if prompt_now {
            TaskStatus::AwaitingApproval
        } else {
            TaskStatus::Working
        };
        if state.last_status != Some(status) {
            state.last_status = Some(status);
            Some(status)
        } else {
            None
        }
    }

    fn status_if_idle(&self, now: Instant) -> Option<TaskStatus> {
        let mut state = self.state.lock();
        let last = state.last_output?;
        if now.duration_since(last) >= Duration::from_millis(1000)
            && state.last_status == Some(TaskStatus::Working)
        {
            state.last_status = Some(TaskStatus::Idle);
            return Some(TaskStatus::Idle);
        }
        None
    }
}

impl Agent for CodexAgent {
    fn start(
        &mut self,
        worktree_path: &Path,
        args: Option<Vec<String>>,
        env: Option<HashMap<String, String>>,
        callbacks: AgentCallbacks,
    ) -> anyhow::Result<AgentRuntime> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: DEFAULT_ROWS,
            cols: DEFAULT_COLS,
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
        let master = Arc::new(Mutex::new(master));
        let writer = Arc::new(Mutex::new(writer));

        let args = args.unwrap_or_else(|| vec!["resume".to_string()]);
        let mut command = CommandBuilder::new("codex");
        command.args(args.iter().map(|s| s.as_str()));
        command.cwd(worktree_path);
        if let Some(env) = env {
            for (key, value) in env {
                command.env(key, value);
            }
        }

        let child = pair
            .slave
            .spawn_command(command)
            .context("failed to start Codex")?;
        let child: Arc<Mutex<ChildHandle>> = Arc::new(Mutex::new(child));

        let status_handle = self.clone();
        let output_callbacks = callbacks.clone();
        let running = Arc::new(AtomicBool::new(true));
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
                        if let Some(status) =
                            status_handle.status_from_output(&buffer[..size], now)
                        {
                            (output_callbacks.on_status)(status);
                        }
                        (output_callbacks.on_output)(chunk);
                    }
                    Err(_) => break,
                }
            }
        });

        let exit_callbacks = callbacks.clone();
        let exit_child = child.clone();
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
                        Err(_) => break 1,
                    }
                }
                std::thread::sleep(Duration::from_millis(200));
            };
            exit_running.store(false, Ordering::Relaxed);
            (exit_callbacks.on_exit)(exit_code);
        });

        Ok(AgentRuntime {
            child,
            writer,
            master,
        })
    }

    fn reset(&mut self, rows: usize, cols: usize) {
        let mut state = self.state.lock();
        state.screen = Screen::new(rows, cols);
        state.parser = Parser::new();
        state.last_output = None;
        state.last_status = None;
        state.prompt_active = false;
    }

    fn resize(&mut self, rows: usize, cols: usize) {
        self.state.lock().screen.resize(rows, cols);
    }

}
