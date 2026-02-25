use crate::commands::CommandResult;
use crate::error::TaskError;
use crate::features::tasks::{AgentKind, TaskRecord};
use std::collections::HashMap;
use uuid::Uuid;

pub(crate) fn require_running_codex_gui_record_mut<'a>(
    tasks: &'a mut HashMap<Uuid, TaskRecord>,
    task_id: Uuid,
) -> CommandResult<&'a mut TaskRecord> {
    let record = tasks
        .get_mut(&task_id)
        .ok_or_else(|| TaskError::NotFound.to_string())?;
    if record.runtime.is_none() {
        return Err(TaskError::NotRunning.to_string());
    }
    if record.agent_kind != AgentKind::CodexGui {
        return Err("task is not running Codex GUI".to_string());
    }
    Ok(record)
}
