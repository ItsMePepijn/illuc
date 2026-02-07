use crate::features::tasks::{TaskSummary, TerminalKind};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

pub fn emit_status(app: &AppHandle, summary: &TaskSummary) {
    let _ = app.emit("task_status_changed", summary);
}

pub fn emit_terminal_output(app: &AppHandle, task_id: Uuid, data: String, kind: TerminalKind) {
    let payload = TerminalOutputPayload {
        task_id,
        data,
        kind,
    };
    let _ = app.emit("task_terminal_output", payload);
}

pub fn emit_terminal_exit(app: &AppHandle, task_id: Uuid, exit_code: i32, kind: TerminalKind) {
    let payload = TerminalExitPayload {
        task_id,
        exit_code,
        kind,
    };
    let _ = app.emit("task_terminal_exit", payload);
}

pub fn emit_diff_changed(app: &AppHandle, task_id: Uuid) {
    let payload = DiffChangedPayload { task_id };
    let _ = app.emit("task_diff_changed", payload);
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
