use std::io::{BufRead, BufReader};
use std::process::ChildStderr;

pub(super) fn spawn_stderr_logger(stderr: Option<ChildStderr>) {
    let Some(stderr) = stderr else {
        return;
    };
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(std::result::Result::ok) {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                log::warn!("codex app-server stderr: {}", trimmed);
            }
        }
    });
}
