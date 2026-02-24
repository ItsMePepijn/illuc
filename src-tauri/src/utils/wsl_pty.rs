use crate::utils::pty::{
    ChildHandle, MasterHandle, ProcessExitStatus, ProcessHandle, ReadHandle, TerminalMaster,
    TerminalSize, WriteHandle,
};
use crate::utils::windows::{bash_escape, suppress_console_window, to_wsl_path};
use anyhow::{anyhow, Context, Result};
use parking_lot::Mutex;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;

const HELPER_PATH: &str = "/tmp/illuc/wsl-pty-helper.py";
const HELPER_SCRIPT: &str = include_str!("wsl_pty_helper.py");
const PORT_PREFIX: &str = "PORT:";

pub struct WslPty {
    pub master: MasterHandle,
    pub writer: WriteHandle,
    pub reader: ReadHandle,
    pub child: Arc<Mutex<ChildHandle>>,
}

struct WslMaster {
    port: u16,
}

impl TerminalMaster for WslMaster {
    fn resize(&self, size: TerminalSize) -> Result<()> {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port))
            .with_context(|| "failed to connect to WSL PTY control port")?;
        write!(stream, "{} {}\n", size.rows, size.cols)
            .with_context(|| "failed to send WSL PTY resize")?;
        if let Err(error) = stream.flush() {
            log::warn!("failed to flush WSL PTY resize command: {error}");
        }
        Ok(())
    }
}

struct WslChild {
    inner: Child,
}

impl ProcessHandle for WslChild {
    fn kill(&mut self) -> Result<()> {
        self.inner.kill().context("failed to kill WSL PTY child")
    }

    fn try_wait(&mut self) -> Result<Option<ProcessExitStatus>> {
        self.inner
            .try_wait()
            .context("failed to query WSL PTY child")
            .map(|status| {
                status.map(|value| ProcessExitStatus::from_code(value.code().unwrap_or(-1)))
            })
    }
}

pub fn spawn_wsl_pty(
    worktree_path: &std::path::Path,
    command: &str,
    args: &[&str],
    rows: u16,
    cols: u16,
    term: Option<&str>,
) -> Result<WslPty> {
    let wsl_path = to_wsl_path(worktree_path)
        .ok_or_else(|| anyhow!("failed to resolve WSL path for {}", worktree_path.display()))?;
    ensure_helper(&wsl_path)?;

    let mut helper_args = vec![
        HELPER_PATH.to_string(),
        "--control-port".to_string(),
        "0".to_string(),
        "--rows".to_string(),
        rows.to_string(),
        "--cols".to_string(),
        cols.to_string(),
        "--cwd".to_string(),
        wsl_path.clone(),
    ];
    if let Some(value) = term {
        helper_args.push("--term".to_string());
        helper_args.push(value.to_string());
    }
    helper_args.push(command.to_string());
    helper_args.extend(args.iter().map(|value| value.to_string()));

    let command_line = build_python_command(&helper_args);
    let mut child = Command::new("wsl.exe");
    suppress_console_window(&mut child);
    child.args(["--cd", &wsl_path, "--", "bash", "-lc", &command_line]);
    child
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = child.spawn().context("failed to spawn WSL PTY helper")?;
    let stdin = child
        .stdin
        .take()
        .context("failed to obtain WSL PTY stdin")?;
    let stdout = child
        .stdout
        .take()
        .context("failed to obtain WSL PTY stdout")?;
    let stderr = child
        .stderr
        .take()
        .context("failed to obtain WSL PTY stderr")?;

    let (stderr_reader, port) = read_helper_port(stderr)?;
    std::thread::spawn(move || {
        let mut reader = stderr_reader;
        let mut buffer = String::new();
        loop {
            buffer.clear();
            match reader.read_line(&mut buffer) {
                Ok(_) => {}
                Err(error) => {
                    log::warn!("failed to read WSL PTY stderr: {error}");
                    break;
                }
            }
            if buffer.is_empty() {
                break;
            }
        }
    });

    let master = Arc::new(Mutex::new(
        Box::new(WslMaster { port }) as Box<dyn TerminalMaster + Send>
    ));
    let writer: WriteHandle = Arc::new(Mutex::new(Box::new(stdin)));
    let reader: ReadHandle = Box::new(stdout);
    let child: Arc<Mutex<ChildHandle>> = Arc::new(Mutex::new(Box::new(WslChild { inner: child })));

    Ok(WslPty {
        master,
        writer,
        reader,
        child,
    })
}

fn read_helper_port(
    stderr: impl Read + Send + 'static,
) -> Result<(BufReader<Box<dyn Read + Send>>, u16)> {
    let mut reader = BufReader::new(Box::new(stderr) as Box<dyn Read + Send>);
    let mut last_message = String::new();
    for _ in 0..5 {
        let mut line = String::new();
        let bytes = reader
            .read_line(&mut line)
            .context("failed to read WSL PTY port")?;
        if bytes == 0 {
            break;
        }
        if line.starts_with(PORT_PREFIX) {
            let port = line[PORT_PREFIX.len()..]
                .trim()
                .parse::<u16>()
                .context("invalid WSL PTY port")?;
            return Ok((reader, port));
        }
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            last_message = trimmed.to_string();
        }
    }
    if last_message.is_empty() {
        Err(anyhow!("WSL PTY helper did not provide port"))
    } else {
        Err(anyhow!(
            "WSL PTY helper failed before providing port: {}",
            last_message
        ))
    }
}

fn ensure_helper(wsl_path: &str) -> Result<()> {
    let command_line = format!(
        "set -e\nmkdir -p /tmp/illuc\ncat > {path} <<'ILLUC_PTY_EOF'\n{script}\nILLUC_PTY_EOF\nchmod 700 {path}",
        path = HELPER_PATH,
        script = HELPER_SCRIPT
    );
    let mut command = Command::new("wsl.exe");
    suppress_console_window(&mut command);
    command.args(["--cd", wsl_path, "--", "bash", "-lc", &command_line]);
    let status = command
        .status()
        .context("failed to install WSL PTY helper")?;
    if !status.success() {
        return Err(anyhow!("failed to install WSL PTY helper"));
    }
    Ok(())
}

fn build_bash_command(command: &str, args: &[String]) -> String {
    let mut command_line = command.to_string();
    for arg in args {
        command_line.push(' ');
        command_line.push_str(&bash_escape(arg));
    }
    command_line
}

fn build_python_command(args: &[String]) -> String {
    let python3 = build_bash_command("python3", args);
    let python = build_bash_command("python", args);
    format!(
        "if command -v python3 >/dev/null 2>&1; then {python3}; \
        elif command -v python >/dev/null 2>&1; then {python}; \
        else echo 'python3/python not found in WSL' 1>&2; exit 127; fi"
    )
}
