#[derive(Debug, Clone)]
pub struct GuiTokenUsageEvent {
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_output_tokens: u64,
    pub last_total_tokens: u64,
    pub last_input_tokens: u64,
    pub last_cached_input_tokens: u64,
    pub last_output_tokens: u64,
    pub last_reasoning_output_tokens: u64,
    pub model_context_window: Option<u64>,
}
