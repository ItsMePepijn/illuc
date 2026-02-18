use crate::commands::CommandResult;
use crate::error::TaskError;
use crate::features::tasks::{TaskManager, TerminalKind};
use crate::utils::pty::TerminalSize;
use anyhow::Context;
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
    match req.kind {
        TerminalKind::Agent => {
            let task_id = req.task_id;
            let master = {
                let tasks = manager.inner.tasks.read();
                let record = tasks
                    .get(&task_id)
                    .ok_or_else(|| TaskError::NotFound.to_string())?;
                match &record.runtime {
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
                .with_context(|| "failed to resize terminal")
                .map_err(|err| err.to_string())?;
            {
                let mut tasks = manager.inner.tasks.write();
                if let Some(record) = tasks.get_mut(&task_id) {
                    record.agent.resize(req.rows as usize, req.cols as usize);
                }
            }
            Ok(())
        }
        TerminalKind::Worktree => {
            let task_id = req.task_id;
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
    }
}
