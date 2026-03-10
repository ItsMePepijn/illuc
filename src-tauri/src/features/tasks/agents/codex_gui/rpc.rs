use super::CodexGuiAgentState;
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::io::Write;

fn service_tier_value(service_tier: Option<&str>) -> Option<Value> {
    match service_tier
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some("fast") => Some(json!("fast")),
        Some("flex") => Some(Value::Null),
        _ => None,
    }
}

pub(super) fn next_id(state: &mut CodexGuiAgentState) -> u64 {
    let id = state.next_id;
    state.next_id += 1;
    id
}

pub(super) fn send_rpc(state: &mut CodexGuiAgentState, payload: Value) -> Result<()> {
    let writer = state.stdin.as_mut().context("codex gui stdin not ready")?;
    let line = serde_json::to_string(&payload)?;
    writer.write_all(line.as_bytes())?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

pub(super) fn send_turn_request(state: &mut CodexGuiAgentState, content: String) -> Result<()> {
    let Some(thread_id) = state.thread_id.clone() else {
        state.pending_messages.push_back(content);
        return Ok(());
    };
    let request_id = next_id(state);
    let mut params = serde_json::Map::new();
    params.insert("threadId".to_string(), json!(thread_id));
    if let Some(model) = state.model.clone() {
        params.insert("model".to_string(), json!(model));
    }
    if let Some(service_tier) = service_tier_value(state.service_tier.as_deref()) {
        params.insert("serviceTier".to_string(), service_tier);
    }
    params.insert(
        "input".to_string(),
        json!([{
            "type": "text",
            "text": content
        }]),
    );
    if let Some(effort) = state.reasoning_effort.clone() {
        params.insert("effort".to_string(), json!(effort));
    }
    let payload = json!({
        "id": request_id,
        "method": "turn/start",
        "params": params
    });
    send_rpc(state, payload)
}

pub(super) fn send_turn_steer_request(
    state: &mut CodexGuiAgentState,
    content: String,
) -> Result<()> {
    let Some(thread_id) = state.thread_id.clone() else {
        state.pending_messages.push_back(content);
        return Ok(());
    };
    let Some(expected_turn_id) = state.current_turn_id.clone() else {
        return send_turn_request(state, content);
    };
    let request_id = next_id(state);
    let payload = json!({
        "id": request_id,
        "method": "turn/steer",
        "params": {
            "threadId": thread_id,
            "expectedTurnId": expected_turn_id,
            "input": [{
                "type": "text",
                "text": content
            }]
        }
    });
    send_rpc(state, payload)
}

pub(super) fn send_turn_interrupt_request(state: &mut CodexGuiAgentState) -> Result<()> {
    let Some(thread_id) = state.thread_id.clone() else {
        return Ok(());
    };
    let Some(turn_id) = state.current_turn_id.clone() else {
        return Ok(());
    };
    let request_id = next_id(state);
    let payload = json!({
        "id": request_id,
        "method": "turn/interrupt",
        "params": {
            "threadId": thread_id,
            "turnId": turn_id
        }
    });
    send_rpc(state, payload)
}

pub(super) fn send_thread_list_request(state: &mut CodexGuiAgentState) -> Result<()> {
    let request_id = next_id(state);
    state.thread_list_request_id = Some(request_id);
    let payload = json!({
        "id": request_id,
        "method": "thread/list",
        "params": {
            "cwd": state.cwd.clone(),
            "archived": false,
            "sortKey": "updated_at",
            "limit": 1
        }
    });
    send_rpc(state, payload)
}

pub(super) fn send_model_list_request(state: &mut CodexGuiAgentState) -> Result<()> {
    let request_id = next_id(state);
    state.model_list_request_id = Some(request_id);
    let payload = json!({
        "id": request_id,
        "method": "model/list",
        "params": {
            "includeHidden": false,
            "limit": 100
        }
    });
    send_rpc(state, payload)
}

pub(super) fn send_thread_start_request(state: &mut CodexGuiAgentState) -> Result<()> {
    let request_id = next_id(state);
    let mut payload = json!({
        "id": request_id,
        "method": "thread/start",
        "params": {
            "cwd": state.cwd.clone(),
            "approvalPolicy": "on-request",
            "sandbox": "workspace-write"
        }
    });
    if let Some(service_tier) = service_tier_value(state.service_tier.as_deref()) {
        payload["params"]["serviceTier"] = service_tier;
    }
    send_rpc(state, payload)
}

pub(super) fn send_rate_limits_request(state: &mut CodexGuiAgentState) -> Result<u64> {
    let request_id = next_id(state);
    state.rate_limits_request_id = Some(request_id);
    let payload = json!({
        "id": request_id,
        "method": "account/rateLimits/read"
    });
    send_rpc(state, payload)?;
    Ok(request_id)
}

pub(super) fn send_thread_resume_request(
    state: &mut CodexGuiAgentState,
    thread_id: &str,
) -> Result<()> {
    let request_id = next_id(state);
    state.thread_resume_request_id = Some(request_id);
    let mut payload = json!({
        "id": request_id,
        "method": "thread/resume",
        "params": {
            "threadId": thread_id,
            "cwd": state.cwd.clone(),
            "approvalPolicy": "on-request",
            "sandbox": "workspace-write"
        }
    });
    if let Some(service_tier) = service_tier_value(state.service_tier.as_deref()) {
        payload["params"]["serviceTier"] = service_tier;
    }
    send_rpc(state, payload)
}

pub(super) fn send_thread_compact_request(state: &mut CodexGuiAgentState) -> Result<()> {
    let Some(thread_id) = state.thread_id.clone() else {
        anyhow::bail!("codex gui thread not ready");
    };
    let request_id = next_id(state);
    let payload = json!({
        "id": request_id,
        "method": "thread/compact/start",
        "params": {
            "threadId": thread_id
        }
    });
    send_rpc(state, payload)
}

pub(super) fn send_thread_rollback_request(
    state: &mut CodexGuiAgentState,
    num_turns: usize,
) -> Result<u64> {
    let Some(thread_id) = state.thread_id.clone() else {
        anyhow::bail!("codex gui thread not ready");
    };
    if num_turns == 0 {
        anyhow::bail!("num_turns must be at least 1");
    }
    let request_id = next_id(state);
    state.rollback_request_id = Some(request_id);
    let payload = json!({
        "id": request_id,
        "method": "thread/rollback",
        "params": {
            "threadId": thread_id,
            "numTurns": num_turns
        }
    });
    send_rpc(state, payload)?;
    Ok(request_id)
}

pub(super) fn flush_pending_messages(state: &mut CodexGuiAgentState) -> Result<()> {
    while let Some(message) = state.pending_messages.pop_front() {
        send_turn_request(state, message)?;
    }
    Ok(())
}
