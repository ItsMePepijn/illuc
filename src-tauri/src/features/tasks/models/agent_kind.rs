use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    Codex,
    Copilot,
    OpenCode,
}
