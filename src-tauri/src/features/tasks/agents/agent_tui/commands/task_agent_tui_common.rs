use crate::commands::CommandResult;
use crate::error::TaskError;
use crate::features::tasks::agents::TuiAgent;
use crate::features::tasks::TaskRecord;
use std::collections::HashMap;
use uuid::Uuid;

pub(crate) fn require_running_tui_agent_mut<'a>(
    tasks: &'a mut HashMap<Uuid, TaskRecord>,
    task_id: Uuid,
) -> CommandResult<&'a mut dyn TuiAgent> {
    let record = tasks
        .get_mut(&task_id)
        .ok_or_else(|| TaskError::NotFound.to_string())?;
    if !record.agent.is_running() {
        return Err(TaskError::NotRunning.to_string());
    }
    record
        .agent
        .as_tui_agent_mut()
        .ok_or_else(|| "task does not use a tui agent".to_string())
}
