use super::GuiMessagePresentation;

#[derive(Debug, Clone, Copy)]
pub enum GuiMessageRole {
    User,
    Assistant,
    System,
    Reasoning,
}

impl GuiMessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            GuiMessageRole::User => "user",
            GuiMessageRole::Assistant => "assistant",
            GuiMessageRole::System => "system",
            GuiMessageRole::Reasoning => "reasoning",
        }
    }
}

#[derive(Debug, Clone)]
pub struct GuiMessageEvent {
    pub message_id: String,
    pub role: GuiMessageRole,
    pub content: String,
    pub presentation: GuiMessagePresentation,
    pub is_delta: bool,
    pub is_final: bool,
}
