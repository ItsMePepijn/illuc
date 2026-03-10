use crate::commands::CommandResult;
use crate::features::tasks::agents::agent_tui::commands::task_agent_tui_common::require_running_tui_agent_mut;
use crate::features::tasks::TaskManager;
use anyhow::Context;
use log::warn;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub task_id: Uuid,
    pub data: String,
}

pub type Response = ();

#[tauri::command]
pub async fn task_agent_tui_write(
    manager: tauri::State<'_, TaskManager>,
    req: Request,
) -> CommandResult<Response> {
    let mut tasks = manager.inner.tasks.write();
    let tui_agent = require_running_tui_agent_mut(&mut tasks, req.task_id)?;
    if let Err(err) = tui_agent
        .write(req.data.as_bytes())
        .with_context(|| "failed to write to tui")
    {
        warn!("failed to write tui input for {}: {}", req.task_id, err);
        return Err(err.to_string());
    }
    Ok(())
}
