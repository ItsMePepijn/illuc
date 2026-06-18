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
    let item_id = string_field(&params, &["itemId", "item_id"])
        .unwrap_or_default()
        .to_string();

    let event = match method {
        "item/commandExecution/requestApproval" | "item/permissions/requestApproval" => {
            GuiRequestEvent::CommandApproval {
                request_id: request_id_text.clone(),
                item_id,
                approval_id: string_field(&params, &["approvalId", "approval_id"])
                    .map(str::to_string),
                command: string_field(&params, &["command"]).map(str::to_string),
                cwd: string_field(&params, &["cwd"]).map(str::to_string),
                reason: string_field(&params, &["reason"]).map(str::to_string),
                network_host: params
                    .pointer("/networkApprovalContext/host")
                    .or_else(|| params.pointer("/network_approval_context/host"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
                network_protocol: params
                    .pointer("/networkApprovalContext/protocol")
                    .or_else(|| params.pointer("/network_approval_context/protocol"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
                additional_read_roots: params
                    .pointer("/additionalPermissions/fileSystem/read")
                    .or_else(|| params.pointer("/additional_permissions/file_system/read"))
                    .and_then(Value::as_array)
                    .map(string_vec)
                    .unwrap_or_default(),
                additional_write_roots: params
                    .pointer("/additionalPermissions/fileSystem/write")
                    .or_else(|| params.pointer("/additional_permissions/file_system/write"))
                    .and_then(Value::as_array)
                    .map(string_vec)
                    .unwrap_or_default(),
                additional_network: params
                    .pointer("/additionalPermissions/network")
                    .or_else(|| params.pointer("/additional_permissions/network"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                available_decisions: string_vec_field(
                    &params,
                    &["availableDecisions", "available_decisions"],
                )
                .unwrap_or_else(|| vec!["approved".to_string(), "denied".to_string()]),
                proposed_exec_policy: string_vec_field(
                    &params,
                    &[
                        "proposedExecpolicyAmendment",
                        "proposedExecPolicyAmendment",
                        "proposed_execpolicy_amendment",
                        "proposed_exec_policy_amendment",
                    ],
                )
                .unwrap_or_default(),
                proposed_network_policy: array_field(
                    &params,
                    &[
                        "proposedNetworkPolicyAmendments",
                        "proposed_network_policy_amendments",
                    ],
                )
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
            }
        },
        "item/fileChange/requestApproval" => GuiRequestEvent::FileChangeApproval {
            request_id: request_id_text.clone(),
            item_id,
            reason: string_field(&params, &["reason"]).map(str::to_string),
            grant_root: string_field(&params, &["grantRoot", "grant_root"])
                .map(str::to_string),
            available_decisions: string_vec_field(
                &params,
                &["availableDecisions", "available_decisions"],
            )
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

fn string_field<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| value.get(*key)?.as_str())
}

fn array_field<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Vec<Value>> {
    keys.iter().find_map(|key| value.get(*key)?.as_array())
}

fn string_vec_field(value: &Value, keys: &[&str]) -> Option<Vec<String>> {
    array_field(value, keys).map(string_vec)
}

fn string_vec(items: &Vec<Value>) -> Vec<String> {
    items
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_request_event;
    use crate::features::tasks::agents::agent_gui::types::GuiRequestEvent;
    use serde_json::json;

    #[test]
    fn command_approval_preserves_snake_case_available_decisions() {
        let request = json!({
            "params": {
                "itemId": "item-1",
                "available_decisions": ["approved_for_session", "denied"]
            }
        });

        let parsed =
            parse_request_event(&request, &json!(7), "item/commandExecution/requestApproval")
                .expect("request should parse");

        match parsed.event {
            GuiRequestEvent::CommandApproval {
                available_decisions,
                ..
            } => assert_eq!(
                available_decisions,
                vec!["approved_for_session".to_string(), "denied".to_string()]
            ),
            _ => panic!("expected command approval"),
        }
    }

    #[test]
    fn file_change_approval_preserves_snake_case_available_decisions() {
        let request = json!({
            "params": {
                "itemId": "item-1",
                "available_decisions": ["approved_for_session", "denied"]
            }
        });

        let parsed = parse_request_event(&request, &json!(7), "item/fileChange/requestApproval")
            .expect("request should parse");

        match parsed.event {
            GuiRequestEvent::FileChangeApproval {
                available_decisions,
                ..
            } => assert_eq!(
                available_decisions,
                vec!["approved_for_session".to_string(), "denied".to_string()]
            ),
            _ => panic!("expected file change approval"),
        }
    }

    #[test]
    fn permission_approval_request_uses_command_approval_payload() {
        let request = json!({
            "params": {
                "item_id": "item-1",
                "available_decisions": ["approved_execpolicy_amendment", "denied"]
            }
        });

        let parsed = parse_request_event(&request, &json!(7), "item/permissions/requestApproval")
            .expect("request should parse");

        match parsed.event {
            GuiRequestEvent::CommandApproval {
                item_id,
                available_decisions,
                ..
            } => {
                assert_eq!(item_id, "item-1");
                assert_eq!(
                    available_decisions,
                    vec![
                        "approved_execpolicy_amendment".to_string(),
                        "denied".to_string()
                    ]
                );
            }
            _ => panic!("expected command approval"),
        }
    }
}
