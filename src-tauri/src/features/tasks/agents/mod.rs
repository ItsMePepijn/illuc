use crate::features::tasks::TaskStatus;
use crate::utils::pty::{ChildHandle, MasterHandle, WriteHandle};
use parking_lot::Mutex;
use std::path::Path;
use std::sync::Arc;

pub mod codex;
pub mod copilot;

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
}
