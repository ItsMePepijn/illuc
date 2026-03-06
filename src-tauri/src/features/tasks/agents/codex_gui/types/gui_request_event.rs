use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuiRequestQuestionOption {
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuiRequestQuestion {
    pub id: String,
    pub header: String,
    pub question: String,
    pub is_other: bool,
    pub is_secret: bool,
    pub options: Vec<GuiRequestQuestionOption>,
}

#[derive(Debug, Clone)]
pub enum GuiRequestEvent {
    Cleared,
    CommandApproval {
        request_id: String,
        item_id: String,
        approval_id: Option<String>,
        command: Option<String>,
        cwd: Option<String>,
        reason: Option<String>,
        network_host: Option<String>,
        network_protocol: Option<String>,
        additional_read_roots: Vec<String>,
        additional_write_roots: Vec<String>,
        additional_network: bool,
        available_decisions: Vec<String>,
        proposed_exec_policy: Vec<String>,
        proposed_network_policy: Vec<String>,
    },
    FileChangeApproval {
        request_id: String,
        item_id: String,
        reason: Option<String>,
        grant_root: Option<String>,
        available_decisions: Vec<String>,
    },
    UserInput {
        request_id: String,
        item_id: String,
        questions: Vec<GuiRequestQuestion>,
    },
}
