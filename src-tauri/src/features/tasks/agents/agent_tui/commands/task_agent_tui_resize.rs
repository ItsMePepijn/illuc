use crate::commands::CommandResult;
use crate::features::tasks::agents::agent_tui::commands::task_agent_tui_common::require_running_tui_agent_mut;
use crate::features::tasks::TaskManager;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub task_id: Uuid,
    pub cols: u16,
    pub rows: u16,
}

pub type Response = ();

#[tauri::command]
pub async fn task_agent_tui_resize(
    manager: tauri::State<'_, TaskManager>,
    req: Request,
) -> CommandResult<Response> {
    let mut tasks = manager.inner.tasks.write();
    let tui_agent = require_running_tui_agent_mut(&mut tasks, req.task_id)?;
    tui_agent.resize(req.rows as usize, req.cols as usize);
    Ok(())
}
