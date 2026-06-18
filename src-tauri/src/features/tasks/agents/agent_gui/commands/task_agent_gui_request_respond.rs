use crate::commands::CommandResult;
use crate::features::tasks::agents::agent_gui::commands::task_agent_gui_common::with_running_gui_agent_mut;
use crate::features::tasks::TaskManager;
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub task_id: Uuid,
    pub request_id: String,
    pub response: Value,
}

pub type Response = ();

#[tauri::command]
pub async fn task_agent_gui_request_respond(
    manager: tauri::State<'_, TaskManager>,
    req: Request,
) -> CommandResult<Response> {
    let response = normalize_decision(req.response);
    log::info!(
        "task_agent_gui_request_respond task_id={} request_id={} payload={}",
        req.task_id,
        req.request_id,
        response
    );
    with_running_gui_agent_mut(&manager, req.task_id, |gui_agent| {
        gui_agent
            .respond_ui_request(req.request_id, response)
            .map_err(|error| error.to_string())
    })
}

fn normalize_decision(response: Value) -> Value {
    let mut object = match response {
        Value::Object(object) => object,
        value => return value,
    };

    let decision = object
        .get("decision")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let Some(decision) = decision else {
        return Value::Object(object);
    };

    let normalized = match decision.to_ascii_lowercase().as_str() {
        "approve" | "approved" | "allow" | "allowed" | "accept" | "accepted" | "yes" => "accept",
        "approved_for_session"
        | "approve_for_session"
        | "accept_for_session"
        | "acceptforsession" => "acceptForSession",
        "deny" | "denied" | "decline" | "declined" | "reject" | "rejected" | "no" => "decline",
        "abort" | "cancel" | "cancelled" | "canceled" => "cancel",
        _ => decision,
    };
    object.insert(
        "decision".to_string(),
        Value::String(normalized.to_string()),
    );
    Value::Object(object)
}

#[cfg(test)]
mod tests {
    use super::normalize_decision;
    use serde_json::json;

    #[test]
    fn normalizes_legacy_approval_aliases_to_codex_v2_decisions() {
        assert_eq!(
            normalize_decision(json!({ "decision": "approved" })),
            json!({ "decision": "accept" })
        );
        assert_eq!(
            normalize_decision(json!({ "decision": "approved_for_session" })),
            json!({ "decision": "acceptForSession" })
        );
        assert_eq!(
            normalize_decision(json!({ "decision": "denied" })),
            json!({ "decision": "decline" })
        );
    }

    #[test]
    fn preserves_structured_approval_decisions() {
        let response = json!({
            "decision": {
                "acceptWithExecpolicyAmendment": {
                    "execpolicy_amendment": {
                        "rule": "prefix_rule",
                        "prefix": ["npm", "test"]
                    }
                }
            }
        });

        assert_eq!(normalize_decision(response.clone()), response);
    }
}
