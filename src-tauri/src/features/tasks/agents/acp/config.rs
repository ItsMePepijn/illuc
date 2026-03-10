use agent_client_protocol::{
    ClientCapabilities, ContentBlock, FileSystemCapability, InitializeRequest, LoadSessionRequest,
    NewSessionRequest, PromptRequest, SessionId,
};
use anyhow::Result;
use std::path::Path;
use std::process::Command;

pub trait AcpAgentConfig: Send + Sync + 'static {
    fn id(&self) -> &'static str;

    fn title(&self) -> &'static str {
        self.id()
    }

    fn build_command(&self, worktree_path: &Path) -> Command;

    fn client_capabilities(&self) -> ClientCapabilities {
        ClientCapabilities::new()
            .fs(FileSystemCapability::new()
                .read_text_file(true)
                .write_text_file(true))
            .terminal(true)
    }

    fn initialize_request(&self) -> InitializeRequest {
        InitializeRequest::new(agent_client_protocol::ProtocolVersion::LATEST)
            .client_capabilities(self.client_capabilities())
            .client_info(
                agent_client_protocol::Implementation::new(self.id(), env!("CARGO_PKG_VERSION"))
                    .title(self.title()),
            )
    }

    fn new_session_request(&self, worktree_path: &Path) -> Result<NewSessionRequest> {
        Ok(NewSessionRequest::new(worktree_path))
    }

    fn load_session_request(&self, _worktree_path: &Path) -> Result<Option<LoadSessionRequest>> {
        Ok(None)
    }

    fn prompt_request(&self, session_id: SessionId, content: String) -> PromptRequest {
        PromptRequest::new(session_id, vec![ContentBlock::from(content)])
    }

    fn summarize_permission(
        &self,
        request: &agent_client_protocol::RequestPermissionRequest,
    ) -> String {
        request
            .tool_call
            .fields
            .title
            .clone()
            .unwrap_or_else(|| "Allow this ACP tool call?".to_string())
    }
}
