use crate::tasks::TaskSummary;
use log::debug;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

pub fn emit_status(app: &AppHandle, summary: &TaskSummary) {
    debug!("emit task_status_changed task_id={} status={:?}", summary.task_id, summary.status);
    let _ = app.emit("task_status_changed", summary);
}

pub fn emit_terminal_output(app: &AppHandle, task_id: Uuid, data: String) {
    debug!("emit task_terminal_output task_id={} bytes={}", task_id, data.len());
    let payload = TerminalOutputPayload { task_id, data };
    let _ = app.emit("task_terminal_output", payload);
}

pub fn emit_terminal_exit(app: &AppHandle, task_id: Uuid, exit_code: i32) {
    debug!("emit task_terminal_exit task_id={} exit_code={}", task_id, exit_code);
    let payload = TerminalExitPayload { task_id, exit_code };
    let _ = app.emit("task_terminal_exit", payload);
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TerminalOutputPayload {
    task_id: Uuid,
    data: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TerminalExitPayload {
    task_id: Uuid,
    exit_code: i32,
}
