use super::super::{rpc, CodexGuiAgentState};
use crate::features::tasks::agents::agent_gui::types::{
    GuiRequestEvent, GuiRequestQuestion, GuiRequestQuestionOption,
};
use anyhow::Result;
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::sync::mpsc::sync_channel;
use std::sync::Arc;

pub(super) fn handle_server_request(
    state: &Arc<Mutex<CodexGuiAgentState>>,
    request: &Value,
    request_id: &Value,
    method: &str,
) -> Result<()> {
    let Some(request) = parse_request_event(request, request_id, method) else {
        let payload = json!({
            "id": request_id,
            "result": {}
        });
        let mut state = state.lock();
        return rpc::send_rpc(&mut state, payload);
    };

    let (callbacks, response) = {
        let (sender, receiver) = sync_channel(1);
        let callbacks = {
            let mut guard = state.lock();
            guard
                .pending_server_requests
                .insert(request.request_id.clone(), sender);
            guard.callbacks.clone()
        };
        if let Some(callbacks) = callbacks.clone() {
            (callbacks.on_gui_request)(request.event);
        }
        let response = receiver.recv().unwrap_or_else(|_| json!({}));
        (callbacks, response)
    };

    {
        let mut guard = state.lock();
        rpc::send_rpc(
            &mut guard,
            json!({
                "id": request_id,
                "result": response
            }),
        )?;
    }

    if let Some(callbacks) = callbacks {
        (callbacks.on_gui_request)(GuiRequestEvent::Cleared);
    }

    Ok(())
}

struct PendingRequestEvent {
    request_id: String,
    event: GuiRequestEvent,
}

fn parse_request_event(
    request: &Value,
    request_id: &Value,
    method: &str,
) -> Option<PendingRequestEvent> {
    let request_id_text = request_id.to_string();
    let params = request.get("params").cloned().unwrap_or(Value::Null);
    let item_id = params
        .get("itemId")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    let event = match method {
        "item/commandExecution/requestApproval" => GuiRequestEvent::CommandApproval {
            request_id: request_id_text.clone(),
            item_id,
            approval_id: params
                .get("approvalId")
                .and_then(Value::as_str)
                .map(str::to_string),
            command: params
                .get("command")
                .and_then(Value::as_str)
                .map(str::to_string),
            cwd: params
                .get("cwd")
                .and_then(Value::as_str)
                .map(str::to_string),
            reason: params
                .get("reason")
                .and_then(Value::as_str)
                .map(str::to_string),
            network_host: params
                .pointer("/networkApprovalContext/host")
                .and_then(Value::as_str)
                .map(str::to_string),
            network_protocol: params
                .pointer("/networkApprovalContext/protocol")
                .and_then(Value::as_str)
                .map(str::to_string),
            additional_read_roots: params
                .pointer("/additionalPermissions/fileSystem/read")
                .and_then(Value::as_array)
                .map(string_vec)
                .unwrap_or_default(),
            additional_write_roots: params
                .pointer("/additionalPermissions/fileSystem/write")
                .and_then(Value::as_array)
                .map(string_vec)
                .unwrap_or_default(),
            additional_network: params
                .pointer("/additionalPermissions/network")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            available_decisions: params
                .get("availableDecisions")
                .and_then(Value::as_array)
                .map(string_vec)
                .unwrap_or_else(|| vec!["approved".to_string(), "denied".to_string()]),
            proposed_exec_policy: params
                .get("proposedExecpolicyAmendment")
                .and_then(Value::as_array)
                .map(string_vec)
                .unwrap_or_default(),
            proposed_network_policy: params
                .get("proposedNetworkPolicyAmendments")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| {
                            Some(format!(
                                "{} {}",
                                item.get("action")?.as_str()?,
                                item.get("host")?.as_str()?
                            ))
                        })
                        .collect()
                })
                .unwrap_or_default(),
        },
        "item/fileChange/requestApproval" => GuiRequestEvent::FileChangeApproval {
            request_id: request_id_text.clone(),
            item_id,
            reason: params
                .get("reason")
                .and_then(Value::as_str)
                .map(str::to_string),
            grant_root: params
                .get("grantRoot")
                .and_then(Value::as_str)
                .map(str::to_string),
            available_decisions: params
                .get("availableDecisions")
                .and_then(Value::as_array)
                .map(string_vec)
                .unwrap_or_else(|| vec!["approved".to_string(), "denied".to_string()]),
        },
        "item/tool/requestUserInput" => GuiRequestEvent::UserInput {
            request_id: request_id_text.clone(),
            item_id,
            questions: params
                .get("questions")
                .and_then(Value::as_array)
                .map(|questions| {
                    questions
                        .iter()
                        .map(|question| GuiRequestQuestion {
                            id: question
                                .get("id")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string(),
                            header: question
                                .get("header")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string(),
                            question: question
                                .get("question")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_string(),
                            is_other: question
                                .get("isOther")
                                .and_then(Value::as_bool)
                                .unwrap_or(false),
                            is_secret: question
                                .get("isSecret")
                                .and_then(Value::as_bool)
                                .unwrap_or(false),
                            options: question
                                .get("options")
                                .and_then(Value::as_array)
                                .map(|options| {
                                    options
                                        .iter()
                                        .map(|option| GuiRequestQuestionOption {
                                            label: option
                                                .get("label")
                                                .and_then(Value::as_str)
                                                .unwrap_or_default()
                                                .to_string(),
                                            description: option
                                                .get("description")
                                                .and_then(Value::as_str)
                                                .unwrap_or_default()
                                                .to_string(),
                                        })
                                        .collect()
                                })
                                .unwrap_or_default(),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        },
        _ => return None,
    };

    Some(PendingRequestEvent {
        request_id: request_id_text,
        event,
    })
}

fn string_vec(items: &Vec<Value>) -> Vec<String> {
    items
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}
