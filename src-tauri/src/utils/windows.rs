use portable_pty::CommandBuilder;
use std::path::Path;
use std::process::Command;

pub fn to_wsl_path(path: &Path) -> Option<String> {
    let mut path_str = path.to_string_lossy().replace('\\', "/");
    if path_str.starts_with("//?/") {
        path_str = path_str.trim_start_matches("//?/").to_string();
    }
    if path_str.starts_with("/mnt/") {
        return Some(path_str);
    }
    if path_str.len() >= 3 {
        let drive = path_str.chars().next()?;
        let colon = path_str.chars().nth(1)?;
        let slash = path_str.chars().nth(2)?;
        if drive.is_ascii_alphabetic() && colon == ':' && slash == '/' {
            let rest = &path_str[3..];
            return Some(format!(
                "/mnt/{}/{}",
                drive.to_ascii_lowercase(),
                rest.trim_start_matches('/')
            ));
        }
    }
    None
}

pub fn bash_escape(value: &str) -> String {
    let mut escaped = String::from("'");
    for ch in value.chars() {
        if ch == '\'' {
            escaped.push_str("'\"'\"'");
        } else {
            escaped.push(ch);
        }
    }
    escaped.push('\'');
    escaped
}

fn build_wsl_command_parts(
    worktree_path: &Path,
    command: &str,
    args: &[&str],
) -> (String, String) {
    let wsl_path = to_wsl_path(worktree_path).unwrap_or_else(|| "/".to_string());
    let mut command_line = format!("cd {} && {}", bash_escape(&wsl_path), command);
    for arg in args {
        command_line.push(' ');
        command_line.push_str(&bash_escape(arg));
    }
    (wsl_path, command_line)
}

pub fn build_wsl_command(
    worktree_path: &Path,
    command: &str,
    args: &[&str],
) -> CommandBuilder {
    let mut command_builder = CommandBuilder::new("wsl.exe");
    let (wsl_path, command_line) = build_wsl_command_parts(worktree_path, command, args);
    command_builder.args([
        "--cd",
        &wsl_path,
        "--",
        "bash",
        "-lc",
        command_line.as_str(),
    ]);
    command_builder
}

pub fn build_wsl_process_command(
    worktree_path: &Path,
    command: &str,
    args: &[&str],
) -> Command {
    let mut command_builder = Command::new("wsl.exe");
    let (wsl_path, command_line) = build_wsl_command_parts(worktree_path, command, args);
    command_builder.args([
        "--cd",
        &wsl_path,
        "--",
        "bash",
        "-lc",
        command_line.as_str(),
    ]);
    command_builder
}
