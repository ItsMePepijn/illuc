use super::command::AcpCommand;
use agent_client_protocol::{PermissionOption, RequestPermissionResponse, SessionId};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::mpsc::SyncSender;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Default)]
pub(crate) struct AcpAgentState {
    pub(crate) control_tx: Option<UnboundedSender<AcpCommand>>,
    pub(crate) exit_code: Option<i32>,
    pub(crate) session_id: Option<SessionId>,
    pub(crate) pending_permission_requests: HashMap<String, PendingPermissionRequest>,
    pub(crate) next_message_id: u64,
    pub(crate) active_message_ids: HashMap<String, String>,
}

pub(crate) type SharedAcpAgentState = Arc<Mutex<AcpAgentState>>;

pub(crate) struct PendingPermissionRequest {
    pub(crate) options: Vec<PermissionOption>,
    pub(crate) reply: SyncSender<RequestPermissionResponse>,
}
