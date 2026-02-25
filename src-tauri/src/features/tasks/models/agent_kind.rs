use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentKind {
    Codex,
    #[serde(rename = "codex_gui")]
    CodexGui,
    Copilot,
}
