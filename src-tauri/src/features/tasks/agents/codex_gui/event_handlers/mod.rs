mod notification;
mod response;
mod server_request;
mod stderr_logger;

use super::CodexGuiAgentState;
use anyhow::Result;
use parking_lot::Mutex;
use serde_json::Value;
use std::process::ChildStderr;
use std::sync::Arc;

pub(super) fn handle_notification(state: &Arc<Mutex<CodexGuiAgentState>>, value: &Value) {
    notification::handle_notification(state, value);
}

pub(super) fn handle_response(state: &Arc<Mutex<CodexGuiAgentState>>, value: &Value) {
    response::handle_response(state, value);
}

pub(super) fn handle_server_request(
    state: &Arc<Mutex<CodexGuiAgentState>>,
    request: &Value,
    request_id: &Value,
    method: &str,
) -> Result<()> {
    server_request::handle_server_request(state, request, request_id, method)
}

pub(super) fn spawn_stderr_logger(stderr: Option<ChildStderr>) {
    stderr_logger::spawn_stderr_logger(stderr);
}
