use anyhow::{Context, Result};
use parking_lot::Mutex;
use portable_pty::{ExitStatus, MasterPty};
use std::io::{Read, Write};
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub struct TerminalSize {
    pub rows: u16,
    pub cols: u16,
}

#[derive(Clone, Copy, Debug)]
pub struct ProcessExitStatus {
    code: i32,
}

impl ProcessExitStatus {
    pub fn from_code(code: i32) -> Self {
        Self { code }
    }

    pub fn exit_code(&self) -> i32 {
        self.code
    }

    pub fn success(&self) -> bool {
        self.code == 0
    }
}

impl From<ExitStatus> for ProcessExitStatus {
    fn from(status: ExitStatus) -> Self {
        Self {
            code: status.exit_code() as i32,
        }
    }
}

pub trait TerminalMaster: Send {
    fn resize(&self, size: TerminalSize) -> Result<()>;
}

pub trait ProcessHandle: Send + Sync {
    fn kill(&mut self) -> Result<()>;
    fn try_wait(&mut self) -> Result<Option<ProcessExitStatus>>;
}

pub type MasterHandle = Arc<Mutex<Box<dyn TerminalMaster + Send>>>;
pub type WriteHandle = Arc<Mutex<Box<dyn Write + Send>>>;
pub type ReadHandle = Box<dyn Read + Send>;
pub type ChildHandle = Box<dyn ProcessHandle + Send + Sync>;

struct PortableMaster {
    inner: Box<dyn MasterPty + Send>,
}

impl TerminalMaster for PortableMaster {
    fn resize(&self, size: TerminalSize) -> Result<()> {
        self.inner
            .resize(portable_pty::PtySize {
                cols: size.cols,
                rows: size.rows,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("failed to resize terminal")
    }
}

struct PortableChild {
    inner: Box<dyn portable_pty::Child + Send + Sync>,
}

impl ProcessHandle for PortableChild {
    fn kill(&mut self) -> Result<()> {
        self.inner.kill().context("failed to kill child process")
    }

    fn try_wait(&mut self) -> Result<Option<ProcessExitStatus>> {
        self.inner
            .try_wait()
            .context("failed to query child process")
            .map(|status| status.map(ProcessExitStatus::from))
    }
}

pub fn wrap_portable_master(master: Box<dyn MasterPty + Send>) -> MasterHandle {
    Arc::new(Mutex::new(Box::new(PortableMaster { inner: master })))
}

pub fn wrap_portable_child(
    child: Box<dyn portable_pty::Child + Send + Sync>,
) -> Arc<Mutex<ChildHandle>> {
    Arc::new(Mutex::new(Box::new(PortableChild { inner: child })))
}
