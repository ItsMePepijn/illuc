use crate::error::{Result, TaskError};
use crate::utils::fs::ensure_file;
use crate::utils::windows::suppress_console_window;
use std::path::Path;
use std::process::Command;

pub fn spawn(path: &Path) -> Result<()> {
    #[cfg(windows)]
    let candidates = ["code.exe", "code.cmd", "code"];
    #[cfg(not(windows))]
    let candidates = ["code"];

    for candidate in candidates {
        let mut command = Command::new(candidate);
        #[cfg(windows)]
        suppress_console_window(&mut command);
        let result = command.arg(path).spawn();
        match result {
            Ok(_) => return Ok(()),
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    continue;
                } else {
                    return Err(err.into());
                }
            }
        }
    }
    Err(TaskError::Message(
        "Unable to launch VS Code. Make sure the `code` command is available.".to_string(),
    ))
}

pub fn spawn_file(path: &Path, line: Option<u32>, column: Option<u32>) -> Result<()> {
    ensure_file(path)?;
    let line = line.unwrap_or(1);
    let column = column.unwrap_or(1);
    let target = format!("{}:{}:{}", path.display(), line, column);

    #[cfg(windows)]
    let candidates = ["code.exe", "code.cmd", "code"];
    #[cfg(not(windows))]
    let candidates = ["code"];

    for candidate in candidates {
        let mut command = Command::new(candidate);
        #[cfg(windows)]
        suppress_console_window(&mut command);
        let result = command.arg("-r").arg("--goto").arg(&target).spawn();
        match result {
            Ok(_) => return Ok(()),
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    continue;
                } else {
                    return Err(err.into());
                }
            }
        }
    }
    Err(TaskError::Message(
        "Unable to launch VS Code. Make sure the `code` command is available.".to_string(),
    ))
}
