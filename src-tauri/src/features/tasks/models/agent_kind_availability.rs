use crate::features::tasks::models::agent_kind::AgentKind;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentKindAvailability {
    pub kind: AgentKind,
    pub label: &'static str,
    pub installed: bool,
}

impl AgentKindAvailability {
    pub fn from_kind(kind: AgentKind) -> Self {
        Self {
            kind,
            label: kind.label(),
            installed: kind.is_installed(),
        }
    }
}
