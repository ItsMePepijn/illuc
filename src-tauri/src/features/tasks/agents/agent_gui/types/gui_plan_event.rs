use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuiPlanStep {
    pub step: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct GuiPlanEvent {
    pub explanation: Option<String>,
    pub plan: Vec<GuiPlanStep>,
}
