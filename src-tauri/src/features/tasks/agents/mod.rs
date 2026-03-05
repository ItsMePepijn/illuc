use crate::features::tasks::TaskStatus;
use crate::utils::pty::{ChildHandle, MasterHandle, WriteHandle};
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

pub mod codex;
pub mod codex_gui;
pub mod copilot;
pub mod open_code;

pub struct AgentRuntime {
    pub child: Arc<Mutex<ChildHandle>>,
    pub writer: WriteHandle,
    pub master: MasterHandle,
}

#[derive(Clone)]
pub struct AgentCallbacks {
    pub on_output: Arc<dyn Fn(String) + Send + Sync>,
    pub on_status: Arc<dyn Fn(TaskStatus) + Send + Sync>,
    pub on_exit: Arc<dyn Fn(i32) + Send + Sync>,
    pub on_gui_event: Arc<dyn Fn(GuiMessageEvent) + Send + Sync>,
    pub on_gui_activity: Arc<dyn Fn(GuiActivityEvent) + Send + Sync>,
    pub on_gui_plan: Arc<dyn Fn(GuiPlanEvent) + Send + Sync>,
    pub on_gui_token_usage: Arc<dyn Fn(GuiTokenUsageEvent) + Send + Sync>,
    pub on_gui_request: Arc<dyn Fn(GuiRequestEvent) + Send + Sync>,
    pub on_gui_hydrated: Arc<dyn Fn() + Send + Sync>,
}

#[derive(Clone, Copy)]
pub enum GuiMessageRole {
    User,
    Assistant,
    System,
    Reasoning,
}

impl GuiMessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            GuiMessageRole::User => "user",
            GuiMessageRole::Assistant => "assistant",
            GuiMessageRole::System => "system",
            GuiMessageRole::Reasoning => "reasoning",
        }
    }
}

#[derive(Clone)]
pub struct GuiMessageEvent {
    pub message_id: String,
    pub role: GuiMessageRole,
    pub content: String,
    pub is_delta: bool,
    pub is_final: bool,
}

#[derive(Clone)]
pub struct GuiActivityEvent {
    pub label: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuiPlanStep {
    pub step: String,
    pub status: String,
}

#[derive(Clone)]
pub struct GuiPlanEvent {
    pub explanation: Option<String>,
    pub plan: Vec<GuiPlanStep>,
}

#[derive(Clone)]
pub struct GuiTokenUsageEvent {
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_output_tokens: u64,
    pub last_total_tokens: u64,
    pub last_input_tokens: u64,
    pub last_cached_input_tokens: u64,
    pub last_output_tokens: u64,
    pub last_reasoning_output_tokens: u64,
    pub model_context_window: Option<u64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuiRequestQuestionOption {
    pub label: String,
    pub description: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuiRequestQuestion {
    pub id: String,
    pub header: String,
    pub question: String,
    pub is_other: bool,
    pub is_secret: bool,
    pub options: Vec<GuiRequestQuestionOption>,
}

#[derive(Clone)]
pub enum GuiRequestEvent {
    Cleared,
    CommandApproval {
        request_id: String,
        item_id: String,
        approval_id: Option<String>,
        command: Option<String>,
        cwd: Option<String>,
        reason: Option<String>,
        network_host: Option<String>,
        network_protocol: Option<String>,
        additional_read_roots: Vec<String>,
        additional_write_roots: Vec<String>,
        additional_network: bool,
        available_decisions: Vec<String>,
        proposed_exec_policy: Vec<String>,
        proposed_network_policy: Vec<String>,
    },
    FileChangeApproval {
        request_id: String,
        item_id: String,
        reason: Option<String>,
        grant_root: Option<String>,
        available_decisions: Vec<String>,
    },
    UserInput {
        request_id: String,
        item_id: String,
        questions: Vec<GuiRequestQuestion>,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentModelCapability {
    pub model: String,
    pub reasoning_efforts: Vec<String>,
}

pub trait Agent: Send + Sync {
    fn start(
        &mut self,
        worktree_path: &Path,
        callbacks: AgentCallbacks,
        rows: u16,
        cols: u16,
    ) -> anyhow::Result<AgentRuntime>;

    fn reset(&mut self, rows: usize, cols: usize);

    fn resize(&mut self, rows: usize, cols: usize);

    fn send_message(&mut self, _content: String) -> anyhow::Result<()> {
        Err(anyhow!("agent does not support GUI messaging"))
    }

    fn set_model(&mut self, _model: Option<String>) -> anyhow::Result<()> {
        Ok(())
    }

    fn available_models(&self) -> Vec<String> {
        Vec::new()
    }

    fn available_model_capabilities(&self) -> Vec<AgentModelCapability> {
        Vec::new()
    }

    fn selected_model(&self) -> Option<String> {
        None
    }

    fn selected_reasoning_effort(&self) -> Option<String> {
        None
    }

    fn interrupt(&mut self) -> anyhow::Result<()> {
        Err(anyhow!("agent does not support interrupt"))
    }

    fn set_reasoning_effort(&mut self, _effort: Option<String>) -> anyhow::Result<()> {
        Ok(())
    }

    fn refresh_rate_limits(&mut self) -> anyhow::Result<Option<Value>> {
        Ok(None)
    }

    fn respond_ui_request(&mut self, _request_id: String, _response: Value) -> anyhow::Result<()> {
        Err(anyhow!("agent does not support ui requests"))
    }

    fn compact_thread(&mut self) -> anyhow::Result<()> {
        Err(anyhow!("agent does not support thread compaction"))
    }

    fn rollback_thread(&mut self, _num_turns: usize) -> anyhow::Result<Vec<GuiMessageEvent>> {
        Err(anyhow!("agent does not support thread rollback"))
    }

    fn start_new_thread(&mut self) -> anyhow::Result<()> {
        Err(anyhow!("agent does not support starting a new thread"))
    }
}
