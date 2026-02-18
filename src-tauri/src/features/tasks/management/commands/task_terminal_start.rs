use crate::commands::CommandResult;
use crate::error::TaskError;
use crate::features::tasks::events::{emit_terminal_exit, emit_terminal_output};
use crate::features::tasks::{
    build_worktree_shell_command, TaskManager, TaskRuntime, TerminalKind, DEFAULT_PTY_COLS,
    DEFAULT_PTY_ROWS,
};
use crate::utils::pty::{wrap_portable_child, wrap_portable_master};
use anyhow::Context;
use log::warn;
use parking_lot::Mutex;
use portable_pty::{native_pty_system, PtySize};
use serde::Deserialize;
use std::io::Read;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub task_id: Uuid,
    pub kind: TerminalKind,
    pub cols: Option<u16>,
    pub rows: Option<u16>,
}

pub type Response = ();

#[tauri::command]
pub async fn task_terminal_start(
    manager: tauri::State<'_, TaskManager>,
    app_handle: tauri::AppHandle,
    req: Request,
) -> CommandResult<Response> {
    match req.kind {
        TerminalKind::Agent => Ok(()),
        TerminalKind::Worktree => {
            let task_id = req.task_id;
            {
                let tasks = manager.inner.tasks.read();
                let record = tasks
                    .get(&task_id)
                    .ok_or_else(|| TaskError::NotFound.to_string())?;
                if record.shell.is_some() {
                    return Ok(());
                }
            }

            let worktree_path = manager
                .worktree_path(task_id)
                .map_err(|err| err.to_string())?;
            let rows = req.rows.unwrap_or(DEFAULT_PTY_ROWS).max(1);
            let cols = req.cols.unwrap_or(DEFAULT_PTY_COLS).max(1);

            let pty_system = native_pty_system();
            let pair = pty_system
                .openpty(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .map_err(|err| err.to_string())?;

            let master = pair.master;
            let writer = master
                .take_writer()
                .context("failed to obtain worktree terminal writer")
                .map_err(|err| err.to_string())?;
            let reader = master
                .try_clone_reader()
                .context("failed to clone worktree terminal reader")
                .map_err(|err| err.to_string())?;
            let master = wrap_portable_master(master);
            let writer = Arc::new(Mutex::new(writer));

            let command = build_worktree_shell_command(worktree_path.as_path());
            let child = pair
                .slave
                .spawn_command(command)
                .context("failed to start worktree terminal")
                .map_err(|err| err.to_string())?;
            let child = wrap_portable_child(child);

            let output_app = app_handle.clone();
            std::thread::spawn(move || {
                let mut reader = reader;
                let mut buffer = [0u8; 8192];
                loop {
                    match reader.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(size) => {
                            let chunk = String::from_utf8_lossy(&buffer[..size]).to_string();
                            emit_terminal_output(
                                &output_app,
                                task_id,
                                chunk,
                                TerminalKind::Worktree,
                            );
                        }
                        Err(err) => {
                            warn!(
                                "worktree terminal read failed for task {}: {}",
                                task_id, err
                            );
                            break;
                        }
                    }
                }
            });

            let exit_manager = manager.inner().clone();
            let exit_app = app_handle.clone();
            let exit_child = child.clone();
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
                            Err(err) => {
                                warn!(
                                    "worktree terminal wait failed for task {}: {}",
                                    task_id, err
                                );
                                break 1;
                            }
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(200));
                };
                let mut tasks = exit_manager.inner.tasks.write();
                if let Some(record) = tasks.get_mut(&task_id) {
                    record.shell = None;
                }
                emit_terminal_exit(&exit_app, task_id, exit_code, TerminalKind::Worktree);
            });

            let runtime = TaskRuntime {
                child,
                writer,
                master,
            };

            let mut tasks = manager.inner.tasks.write();
            let record = tasks
                .get_mut(&task_id)
                .ok_or_else(|| TaskError::NotFound.to_string())?;
            if record.shell.is_none() {
                record.shell = Some(runtime);
            }
            Ok(())
        }
    }
}
