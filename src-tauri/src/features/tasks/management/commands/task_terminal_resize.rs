use crate::commands::CommandResult;
use crate::error::TaskError;
use crate::features::tasks::{TaskManager, TerminalKind};
use crate::utils::pty::TerminalSize;
use anyhow::Context;
use log::debug;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub task_id: Uuid,
    pub kind: TerminalKind,
    pub cols: u16,
    pub rows: u16,
}

pub type Response = ();

#[tauri::command]
pub async fn task_terminal_resize(
    manager: tauri::State<'_, TaskManager>,
    req: Request,
) -> CommandResult<Response> {
    if !matches!(req.kind, TerminalKind::Worktree) {
        return Err("task_terminal_resize only supports the worktree shell".to_string());
    }

    let task_id = req.task_id;
    debug!(
        "resizing worktree terminal task_id={} rows={} cols={}",
        task_id, req.rows, req.cols
    );
    let master = {
        let tasks = manager.inner.tasks.read();
        let record = tasks
            .get(&task_id)
            .ok_or_else(|| TaskError::NotFound.to_string())?;
        match &record.shell {
            Some(runtime) => runtime.master.clone(),
            None => return Err(TaskError::NotRunning.to_string()),
        }
    };
    master
        .lock()
        .resize(TerminalSize {
            cols: req.cols,
            rows: req.rows,
        })
        .with_context(|| "failed to resize worktree terminal")
        .map_err(|err| err.to_string())?;
    Ok(())
}
