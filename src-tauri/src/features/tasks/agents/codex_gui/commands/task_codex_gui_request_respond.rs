use crate::commands::CommandResult;
use crate::features::tasks::agents::codex_gui::commands::task_codex_gui_common::require_running_codex_gui_record_mut;
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
pub async fn task_codex_gui_request_respond(
    manager: tauri::State<'_, TaskManager>,
    req: Request,
) -> CommandResult<Response> {
    let response = normalize_decision(req.response);
    let mut tasks = manager.inner.tasks.write();
    let record = require_running_codex_gui_record_mut(&mut tasks, req.task_id)?;
    record
        .agent
        .respond_ui_request(req.request_id, response)
        .map_err(|error| error.to_string())
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
        "approve" | "approved" | "allow" | "allowed" | "accept" | "accepted" | "yes" => {
            "approved"
        }
        "deny" | "denied" | "decline" | "declined" | "reject" | "rejected" | "no" => "denied",
        _ => decision,
    };
    object.insert("decision".to_string(), Value::String(normalized.to_string()));
    Value::Object(object)
}
