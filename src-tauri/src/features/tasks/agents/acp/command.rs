use anyhow::Result;
use serde_json::Value;
use std::sync::mpsc::SyncSender;

pub(crate) enum AcpCommand {
    Prompt {
        content: String,
        reply: SyncSender<Result<()>>,
    },
    SetConfigOption {
        config_id: String,
        value: String,
        reply: SyncSender<Result<()>>,
    },
    Cancel {
        reply: SyncSender<Result<()>>,
    },
    RespondUiRequest {
        request_id: String,
        response: Value,
        reply: SyncSender<Result<()>>,
    },
    NewSession {
        reply: SyncSender<Result<()>>,
    },
    Shutdown,
}
