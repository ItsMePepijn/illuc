use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GuiMessagePresentationKind {
    User,
    Standard,
    Reasoning,
    Tool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GuiMessageTextFormat {
    Markdown,
    Plain,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GuiToolRowKind {
    Command,
    Search,
    Read,
    Change,
    Text,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuiToolRow {
    pub kind: GuiToolRowKind,
    pub label: String,
    pub value: Option<String>,
    pub path: Option<String>,
    pub added: Option<u32>,
    pub removed: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuiMessagePresentation {
    pub kind: GuiMessagePresentationKind,
    pub text: Option<String>,
    pub text_format: Option<GuiMessageTextFormat>,
    pub tool_rows: Vec<GuiToolRow>,
    pub tool_status_label: Option<String>,
    pub is_tool_running: bool,
}
