use crate::features::tasks::TaskStatus;
use anyhow::anyhow;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

pub mod acp;
pub mod agent_gui;
pub mod agent_tui;
pub mod codex;
pub mod codex_gui;
pub mod copilot;
pub mod open_code;

use self::agent_gui::types::{
    GuiActivityEvent, GuiMessageEvent, GuiPlanEvent, GuiRequestEvent, GuiTokenUsageEvent,
};

#[derive(Clone)]
pub struct AgentCallbacks {
    pub on_output: Arc<dyn Fn(String) + Send + Sync>,
    pub on_status: Arc<dyn Fn(TaskStatus) + Send + Sync>,
    pub on_exit: Arc<dyn Fn(i32) + Send + Sync>,
    pub on_gui_event: Arc<dyn Fn(GuiMessageEvent) + Send + Sync>,
    pub on_gui_history: Arc<dyn Fn(Vec<GuiMessageEvent>) + Send + Sync>,
    pub on_gui_activity: Arc<dyn Fn(GuiActivityEvent) + Send + Sync>,
    pub on_gui_plan: Arc<dyn Fn(GuiPlanEvent) + Send + Sync>,
    pub on_gui_token_usage: Arc<dyn Fn(GuiTokenUsageEvent) + Send + Sync>,
    pub on_gui_request: Arc<dyn Fn(GuiRequestEvent) + Send + Sync>,
    pub on_gui_hydrated: Arc<dyn Fn() + Send + Sync>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentModelCapability {
    pub model: String,
    pub reasoning_efforts: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSlashCommand {
    pub name: String,
    pub description: String,
    pub input_hint: Option<String>,
}

pub trait Agent: Send + Sync {
    fn start(&mut self, _worktree_path: &Path, _callbacks: AgentCallbacks) -> anyhow::Result<()> {
        Err(anyhow!("agent does not support process startup"))
    }

    fn as_gui_agent(&self) -> Option<&dyn GuiAgent> {
        None
    }

    fn as_gui_agent_mut(&mut self) -> Option<&mut dyn GuiAgent> {
        None
    }

    fn as_tui_agent_mut(&mut self) -> Option<&mut dyn TuiAgent> {
        None
    }

    #[allow(dead_code)]
    fn is_running(&self) -> bool;

    fn stop(&mut self) -> anyhow::Result<()> {
        Err(anyhow!("agent does not support stop"))
    }
}

pub trait TuiAgent: Agent {
    fn start_tui(
        &mut self,
        worktree_path: &Path,
        callbacks: AgentCallbacks,
        rows: u16,
        cols: u16,
    ) -> anyhow::Result<()>;

    fn reset(&mut self, rows: usize, cols: usize);

    fn resize(&mut self, rows: usize, cols: usize);

    fn write(&mut self, _data: &[u8]) -> anyhow::Result<()> {
        Err(anyhow!("tui agent does not support writing"))
    }
}

pub trait GuiAgent: Agent {
    fn as_gui_session_agent(&self) -> Option<&dyn GuiSessionAgent> {
        None
    }

    fn as_gui_session_agent_mut(&mut self) -> Option<&mut dyn GuiSessionAgent> {
        None
    }

    fn as_gui_thread_agent(&self) -> Option<&dyn GuiThreadAgent> {
        None
    }

    fn as_gui_thread_agent_mut(&mut self) -> Option<&mut dyn GuiThreadAgent> {
        None
    }

    fn as_gui_usage_agent(&self) -> Option<&dyn GuiUsageAgent> {
        None
    }

    fn as_gui_usage_agent_mut(&mut self) -> Option<&mut dyn GuiUsageAgent> {
        None
    }

    fn supports_service_tier_toggle(&self) -> bool {
        false
    }

    fn send_message(&mut self, _content: String) -> anyhow::Result<()> {
        Err(anyhow!("gui agent does not support messaging"))
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

    fn available_slash_commands(&self) -> Vec<AgentSlashCommand> {
        Vec::new()
    }

    fn selected_model(&self) -> Option<String> {
        None
    }

    fn selected_reasoning_effort(&self) -> Option<String> {
        None
    }

    fn selected_service_tier(&self) -> Option<String> {
        None
    }

    fn interrupt(&mut self) -> anyhow::Result<()> {
        Err(anyhow!("gui agent does not support interrupt"))
    }

    fn set_reasoning_effort(&mut self, _effort: Option<String>) -> anyhow::Result<()> {
        Ok(())
    }

    fn set_service_tier(&mut self, _service_tier: Option<String>) -> anyhow::Result<()> {
        Ok(())
    }

    fn respond_ui_request(&mut self, _request_id: String, _response: Value) -> anyhow::Result<()> {
        Err(anyhow!("gui agent does not support ui requests"))
    }
}

pub trait GuiSessionAgent: GuiAgent {
    fn start_new_thread(&mut self) -> anyhow::Result<()> {
        Err(anyhow!("gui agent does not support starting a new thread"))
    }
}

pub trait GuiThreadAgent: GuiAgent {
    fn compact_thread(&mut self) -> anyhow::Result<()> {
        Err(anyhow!("gui agent does not support thread compaction"))
    }

    fn rollback_thread(&mut self, _num_turns: usize) -> anyhow::Result<Vec<GuiMessageEvent>> {
        Err(anyhow!("gui agent does not support thread rollback"))
    }
}

pub trait GuiUsageAgent: GuiAgent {
    fn refresh_rate_limits(&mut self) -> anyhow::Result<Option<Value>> {
        Ok(None)
    }
}

pub trait AcpAgent: Agent {}
