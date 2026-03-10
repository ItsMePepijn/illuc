use super::command::AcpCommand;
use super::state::SharedAcpAgentState;
use crate::utils::pty::{ProcessExitStatus, ProcessHandle, TerminalMaster, TerminalSize};
use anyhow::{anyhow, Result};
use std::io::Write;
use tokio::sync::mpsc::UnboundedSender;

pub(crate) struct AcpProcessHandle {
    state: SharedAcpAgentState,
    control_tx: UnboundedSender<AcpCommand>,
}

impl AcpProcessHandle {
    pub(crate) fn new(state: SharedAcpAgentState, control_tx: UnboundedSender<AcpCommand>) -> Self {
        Self { state, control_tx }
    }
}

impl ProcessHandle for AcpProcessHandle {
    fn kill(&mut self) -> Result<()> {
        self.control_tx
            .send(AcpCommand::Shutdown)
            .map_err(|_| anyhow!("ACP runtime command channel closed"))
    }

    fn try_wait(&mut self) -> Result<Option<ProcessExitStatus>> {
        let state = self.state.lock();
        Ok(state.exit_code.map(ProcessExitStatus::from_code))
    }
}

pub(crate) struct NullMaster;

impl TerminalMaster for NullMaster {
    fn resize(&self, _size: TerminalSize) -> Result<()> {
        Ok(())
    }
}

pub(crate) struct NullWriter;

impl Write for NullWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
