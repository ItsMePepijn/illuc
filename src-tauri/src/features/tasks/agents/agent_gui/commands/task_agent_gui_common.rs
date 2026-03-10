use crate::commands::CommandResult;
use crate::error::TaskError;
use crate::features::tasks::agents::{
    Agent, GuiAgent, GuiSessionAgent, GuiThreadAgent, GuiUsageAgent,
};
use crate::features::tasks::{TaskManager, TaskRecord};
use std::collections::HashMap;
use uuid::Uuid;

pub(crate) fn with_running_gui_agent_mut<R>(
    manager: &TaskManager,
    task_id: Uuid,
    action: impl FnOnce(&mut dyn GuiAgent) -> CommandResult<R>,
) -> CommandResult<R> {
    with_running_agent_mut(manager, task_id, |agent| {
        let gui_agent = agent
            .as_gui_agent_mut()
            .ok_or_else(|| "task does not use a GUI agent".to_string())?;
        action(gui_agent)
    })
}

pub(crate) fn with_running_gui_session_agent_mut<R>(
    manager: &TaskManager,
    task_id: Uuid,
    action: impl FnOnce(&mut dyn GuiSessionAgent) -> CommandResult<R>,
) -> CommandResult<R> {
    with_running_gui_agent_mut(manager, task_id, |gui_agent| {
        let session_agent = gui_agent
            .as_gui_session_agent_mut()
            .ok_or_else(|| "GUI agent does not support starting a new chat".to_string())?;
        action(session_agent)
    })
}

pub(crate) fn with_running_gui_thread_agent_mut<R>(
    manager: &TaskManager,
    task_id: Uuid,
    action: impl FnOnce(&mut dyn GuiThreadAgent) -> CommandResult<R>,
) -> CommandResult<R> {
    with_running_gui_agent_mut(manager, task_id, |gui_agent| {
        let thread_agent = gui_agent
            .as_gui_thread_agent_mut()
            .ok_or_else(|| "GUI agent does not support thread history controls".to_string())?;
        action(thread_agent)
    })
}

pub(crate) fn with_running_gui_usage_agent_mut<R>(
    manager: &TaskManager,
    task_id: Uuid,
    action: impl FnOnce(&mut dyn GuiUsageAgent) -> CommandResult<R>,
) -> CommandResult<R> {
    with_running_gui_agent_mut(manager, task_id, |gui_agent| {
        let usage_agent = gui_agent
            .as_gui_usage_agent_mut()
            .ok_or_else(|| "GUI agent does not expose usage data".to_string())?;
        action(usage_agent)
    })
}

pub(crate) fn require_gui_agent<'a>(
    tasks: &'a HashMap<Uuid, TaskRecord>,
    task_id: Uuid,
) -> CommandResult<&'a dyn GuiAgent> {
    let record = require_task_record(tasks, task_id)?;
    record
        .agent
        .as_gui_agent()
        .ok_or_else(|| "task does not use a GUI agent".to_string())
}

fn require_task_record<'a>(
    tasks: &'a HashMap<Uuid, TaskRecord>,
    task_id: Uuid,
) -> CommandResult<&'a TaskRecord> {
    tasks
        .get(&task_id)
        .ok_or_else(|| TaskError::NotFound.to_string())
}

fn require_running_task_record_mut<'a>(
    tasks: &'a mut HashMap<Uuid, TaskRecord>,
    task_id: Uuid,
) -> CommandResult<&'a mut TaskRecord> {
    let record = tasks
        .get_mut(&task_id)
        .ok_or_else(|| TaskError::NotFound.to_string())?;
    if !record.agent.is_running() {
        return Err(TaskError::NotRunning.to_string());
    }
    Ok(record)
}

fn with_running_agent_mut<R>(
    manager: &TaskManager,
    task_id: Uuid,
    action: impl FnOnce(&mut dyn Agent) -> CommandResult<R>,
) -> CommandResult<R> {
    let mut agent = {
        let mut tasks = manager.inner.tasks.write();
        let record = require_running_task_record_mut(&mut tasks, task_id)?;
        std::mem::replace(&mut record.agent, Box::new(BusyAgentPlaceholder))
    };

    let result = action(agent.as_mut());

    let mut tasks = manager.inner.tasks.write();
    let record = tasks
        .get_mut(&task_id)
        .ok_or_else(|| TaskError::NotFound.to_string())?;
    record.agent = agent;

    result
}

struct BusyAgentPlaceholder;

impl Agent for BusyAgentPlaceholder {
    fn is_running(&self) -> bool {
        true
    }
}
