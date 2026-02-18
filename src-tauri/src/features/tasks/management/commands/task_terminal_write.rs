use crate::commands::CommandResult;
use crate::error::TaskError;
use crate::features::tasks::{TaskManager, TerminalKind};
use anyhow::Context;
use log::warn;
use serde::Deserialize;
use std::io::Write;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub task_id: Uuid,
    pub kind: TerminalKind,
    pub data: String,
}

pub type Response = ();

#[tauri::command]
pub async fn task_terminal_write(
    manager: tauri::State<'_, TaskManager>,
    req: Request,
) -> CommandResult<Response> {
    match req.kind {
        TerminalKind::Agent => {
            let task_id = req.task_id;
            let writer = {
                let tasks = manager.inner.tasks.read();
                let record = tasks
                    .get(&task_id)
                    .ok_or_else(|| TaskError::NotFound.to_string())?;
                match &record.runtime {
                    Some(runtime) => runtime.writer.clone(),
                    None => return Err(TaskError::NotRunning.to_string()),
                }
            };
            let mut writer_guard = writer.lock();
            writer_guard
                .write_all(req.data.as_bytes())
                .with_context(|| "failed to write to terminal")
                .map_err(|err| err.to_string())?;
            if let Err(err) = writer_guard.flush() {
                warn!("failed to flush terminal input for {}: {}", task_id, err);
            }
            Ok(())
        }
        TerminalKind::Worktree => {
            let task_id = req.task_id;
            let writer = {
                let tasks = manager.inner.tasks.read();
                let record = tasks
                    .get(&task_id)
                    .ok_or_else(|| TaskError::NotFound.to_string())?;
                match &record.shell {
                    Some(runtime) => runtime.writer.clone(),
                    None => return Err(TaskError::NotRunning.to_string()),
                }
            };
            let mut writer_guard = writer.lock();
            writer_guard
                .write_all(req.data.as_bytes())
                .with_context(|| "failed to write to worktree terminal")
                .map_err(|err| err.to_string())?;
            if let Err(err) = writer_guard.flush() {
                warn!(
                    "failed to flush worktree terminal input for {}: {}",
                    task_id, err
                );
            }
            Ok(())
        }
    }
}
