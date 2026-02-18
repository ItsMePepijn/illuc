use crate::features::tasks::{TaskSummary, TerminalKind};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

pub fn emit_status(app: &AppHandle, summary: &TaskSummary) {
    if let Err(error) = app.emit("task_status_changed", summary) {
        log::warn!("failed to emit task_status_changed event: {error}");
    }
}

pub fn emit_terminal_output(app: &AppHandle, task_id: Uuid, data: String, kind: TerminalKind) {
    let payload = TerminalOutputPayload {
        task_id,
        data,
        kind,
    };
    if let Err(error) = app.emit("task_terminal_output", payload) {
        log::warn!("failed to emit task_terminal_output event: {error}");
    }
}

pub fn emit_terminal_exit(app: &AppHandle, task_id: Uuid, exit_code: i32, kind: TerminalKind) {
    let payload = TerminalExitPayload {
        task_id,
        exit_code,
        kind,
    };
    if let Err(error) = app.emit("task_terminal_exit", payload) {
        log::warn!("failed to emit task_terminal_exit event: {error}");
    }
}

pub fn emit_diff_changed(app: &AppHandle, task_id: Uuid) {
    let payload = DiffChangedPayload { task_id };
    if let Err(error) = app.emit("task_diff_changed", payload) {
        log::warn!("failed to emit task_diff_changed event: {error}");
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TerminalOutputPayload {
    task_id: Uuid,
    data: String,
    kind: TerminalKind,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TerminalExitPayload {
    task_id: Uuid,
    exit_code: i32,
    kind: TerminalKind,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DiffChangedPayload {
    task_id: Uuid,
}
