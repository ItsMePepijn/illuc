mod event_handlers;
pub mod commands;
mod message_parsing;
mod rpc;
mod runtime;
pub mod types;

use crate::features::tasks::agents::{
    Agent, AgentCallbacks, AgentModelCapability, AgentRuntime,
};
use crate::features::tasks::agents::codex_gui::types::GuiMessageEvent;
use crate::utils::pty::{ProcessExitStatus, ProcessHandle, TerminalMaster, TerminalSize};
use anyhow::Context;
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::io::{BufWriter, Write};
use std::process::{Child, ChildStdin};
use std::sync::mpsc::SyncSender;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct CodexGuiAgent {
    pub(super) state: Arc<Mutex<CodexGuiAgentState>>,
}

pub(super) struct CodexGuiAgentState {
    stdin: Option<BufWriter<ChildStdin>>,
    cwd: Option<String>,
    thread_id: Option<String>,
    current_turn_id: Option<String>,
    pending_messages: VecDeque<String>,
    thread_list_request_id: Option<u64>,
    thread_resume_request_id: Option<u64>,
    model_list_request_id: Option<u64>,
    rate_limits_request_id: Option<u64>,
    next_id: u64,
    model: Option<String>,
    reasoning_effort: Option<String>,
    available_models: Vec<String>,
    available_model_capabilities: Vec<AgentModelCapability>,
    latest_rate_limits: Option<Value>,
    pending_server_requests: HashMap<String, SyncSender<Value>>,
    rollback_request_id: Option<u64>,
    rollback_history_events: Vec<GuiMessageEvent>,
    activity_label: Option<String>,
    activity_started_at: Option<DateTime<Utc>>,
    callbacks: Option<AgentCallbacks>,
}

impl Default for CodexGuiAgent {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(CodexGuiAgentState {
                stdin: None,
                cwd: None,
                thread_id: None,
                current_turn_id: None,
                pending_messages: VecDeque::new(),
                thread_list_request_id: None,
                thread_resume_request_id: None,
                model_list_request_id: None,
                rate_limits_request_id: None,
                next_id: 1,
                model: None,
                reasoning_effort: None,
                available_models: Vec::new(),
                available_model_capabilities: Vec::new(),
                latest_rate_limits: None,
                pending_server_requests: HashMap::new(),
                rollback_request_id: None,
                rollback_history_events: Vec::new(),
                activity_label: None,
                activity_started_at: None,
                callbacks: None,
            })),
        }
    }
}

impl Agent for CodexGuiAgent {
    fn start(
        &mut self,
        worktree_path: &std::path::Path,
        callbacks: AgentCallbacks,
        _rows: u16,
        _cols: u16,
    ) -> anyhow::Result<AgentRuntime> {
        runtime::start(self, worktree_path, callbacks)
    }

    fn reset(&mut self, _rows: usize, _cols: usize) {}

    fn resize(&mut self, _rows: usize, _cols: usize) {}

    fn send_message(&mut self, content: String) -> anyhow::Result<()> {
        let text = content.trim().to_string();
        if text.is_empty() {
            return Ok(());
        }
        let mut state = self.state.lock();
        rpc::send_turn_steer_request(&mut state, text)
    }

    fn set_model(&mut self, model: Option<String>) -> anyhow::Result<()> {
        let mut state = self.state.lock();
        state.model = model
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        Ok(())
    }

    fn available_models(&self) -> Vec<String> {
        let state = self.state.lock();
        state.available_models.clone()
    }

    fn available_model_capabilities(&self) -> Vec<AgentModelCapability> {
        let state = self.state.lock();
        state.available_model_capabilities.clone()
    }

    fn selected_model(&self) -> Option<String> {
        let state = self.state.lock();
        state.model.clone()
    }

    fn selected_reasoning_effort(&self) -> Option<String> {
        let state = self.state.lock();
        state.reasoning_effort.clone()
    }

    fn interrupt(&mut self) -> anyhow::Result<()> {
        let mut state = self.state.lock();
        rpc::send_turn_interrupt_request(&mut state)
    }

    fn set_reasoning_effort(&mut self, effort: Option<String>) -> anyhow::Result<()> {
        let mut state = self.state.lock();
        state.reasoning_effort = effort
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        Ok(())
    }

    fn refresh_rate_limits(&mut self) -> anyhow::Result<Option<Value>> {
        let request_id = {
            let mut state = self.state.lock();
            rpc::send_rate_limits_request(&mut state)?
        };

        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        loop {
            {
                let state = self.state.lock();
                if state.rate_limits_request_id != Some(request_id) {
                    return Ok(state.latest_rate_limits.clone());
                }
            }
            if std::time::Instant::now() >= deadline {
                let state = self.state.lock();
                return Ok(state.latest_rate_limits.clone());
            }
            std::thread::sleep(Duration::from_millis(30));
        }
    }

    fn respond_ui_request(&mut self, request_id: String, response: Value) -> anyhow::Result<()> {
        let sender = {
            let mut state = self.state.lock();
            state.pending_server_requests.remove(&request_id)
        }
        .context("codex gui request is no longer pending")?;
        sender
            .send(response)
            .map_err(|_| anyhow::anyhow!("failed to deliver codex gui request response"))
    }

    fn compact_thread(&mut self) -> anyhow::Result<()> {
        let mut state = self.state.lock();
        rpc::send_thread_compact_request(&mut state)
    }

    fn rollback_thread(&mut self, num_turns: usize) -> anyhow::Result<Vec<GuiMessageEvent>> {
        let request_id = {
            let mut state = self.state.lock();
            state.rollback_history_events.clear();
            rpc::send_thread_rollback_request(&mut state, num_turns)?
        };

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            {
                let mut state = self.state.lock();
                if state.rollback_request_id != Some(request_id) {
                    return Ok(std::mem::take(&mut state.rollback_history_events));
                }
            }
            if std::time::Instant::now() >= deadline {
                let mut state = self.state.lock();
                state.rollback_request_id = None;
                return Ok(std::mem::take(&mut state.rollback_history_events));
            }
            std::thread::sleep(Duration::from_millis(30));
        }
    }

    fn start_new_thread(&mut self) -> anyhow::Result<()> {
        let mut state = self.state.lock();
        state.thread_id = None;
        state.current_turn_id = None;
        state.thread_list_request_id = None;
        state.thread_resume_request_id = None;
        state.rollback_request_id = None;
        state.rollback_history_events.clear();
        state.activity_label = None;
        state.activity_started_at = None;
        state.pending_messages.clear();
        rpc::send_thread_start_request(&mut state)
    }
}

pub(super) struct NullMaster;

impl TerminalMaster for NullMaster {
    fn resize(&self, _size: TerminalSize) -> anyhow::Result<()> {
        Ok(())
    }
}

pub(super) struct NullWriter;

impl Write for NullWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(super) struct StdChild {
    pub(super) child: Mutex<Child>,
}

impl ProcessHandle for StdChild {
    fn kill(&mut self) -> anyhow::Result<()> {
        self.child
            .lock()
            .kill()
            .context("failed to kill codex app-server")
    }

    fn try_wait(&mut self) -> anyhow::Result<Option<ProcessExitStatus>> {
        self.child
            .lock()
            .try_wait()
            .context("failed to query codex app-server")
            .map(|status| {
                status.map(|status| {
                    let code = status.code().unwrap_or(1);
                    ProcessExitStatus::from_code(code)
                })
            })
    }
}
