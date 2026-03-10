use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct GuiActivityEvent {
    pub label: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
}
