#[cfg(target_os = "windows")]
use anyhow::{anyhow, Context, Result};
#[cfg(target_os = "windows")]
use portable_pty::CommandBuilder;
#[cfg(target_os = "windows")]
use std::path::Path;
use std::process::Command;

#[cfg(target_os = "windows")]
pub fn suppress_console_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
pub fn suppress_console_window(_command: &mut Command) {}

#[cfg(target_os = "windows")]
pub fn to_wsl_path(path: &Path) -> Option<String> {
    windows_path_text_to_wsl_path(&path.to_string_lossy())
}

pub(crate) fn windows_path_text_to_wsl_path(value: &str) -> Option<String> {
    let mut path_str = value.trim().replace('\\', "/");
    if path_str.is_empty() {
        return None;
    }
    if let Some(rest) = path_str.strip_prefix("//?/UNC/") {
        path_str = format!("//{}", rest);
    } else if let Some(rest) = path_str.strip_prefix("//?/") {
        path_str = rest.to_string();
    }

    if let Some(path) = wsl_unc_path_to_posix(&path_str) {
        return Some(path);
    }
    if path_str.starts_with("/mnt/") {
        return Some(path_str);
    }
    if path_str.starts_with('/') && !path_str.starts_with("//") {
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

fn wsl_unc_path_to_posix(value: &str) -> Option<String> {
    let path = value.strip_prefix("//")?;
    let mut parts = path.split('/').filter(|part| !part.is_empty());
    let server = parts.next()?;
    if !server.eq_ignore_ascii_case("wsl$") && !server.eq_ignore_ascii_case("wsl.localhost") {
        return None;
    }
    let _distro = parts.next()?;
    let rest = parts.collect::<Vec<_>>().join("/");
    if rest.is_empty() {
        Some("/".to_string())
    } else {
        Some(format!("/{}", rest))
    }
}

#[cfg(target_os = "windows")]
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

#[cfg(target_os = "windows")]
fn build_wsl_command_parts(worktree_path: &Path, command: &str, args: &[&str]) -> (String, String) {
    let wsl_path = to_wsl_path(worktree_path).unwrap_or_else(|| "/".to_string());
    let mut command_line = format!("cd {} && {}", bash_escape(&wsl_path), command);
    for arg in args {
        command_line.push(' ');
        command_line.push_str(&bash_escape(arg));
    }
    (wsl_path, command_line)
}

#[cfg(target_os = "windows")]
pub fn build_wsl_command(worktree_path: &Path, command: &str, args: &[&str]) -> CommandBuilder {
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

#[cfg(target_os = "windows")]
pub fn build_wsl_process_command(worktree_path: &Path, command: &str, args: &[&str]) -> Command {
    let mut command_builder = Command::new("wsl.exe");
    suppress_console_window(&mut command_builder);
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

#[cfg(target_os = "windows")]
pub fn resolve_wsl_home_dir() -> Result<std::path::PathBuf> {
    let mut command = Command::new("wsl.exe");
    suppress_console_window(&mut command);
    command.args(["--", "bash", "-lc", "wslpath -w \"$HOME\""]);

    let output = command
        .output()
        .context("failed to query WSL home directory")?;
    if !output.status.success() {
        return Err(anyhow!("failed to query WSL home directory"));
    }

    let home = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if home.is_empty() {
        return Err(anyhow!("WSL home directory is empty"));
    }

    Ok(std::path::PathBuf::from(home))
}

#[cfg(test)]
mod tests {
    use super::windows_path_text_to_wsl_path;

    #[test]
    fn converts_drive_letter_paths_to_wsl_paths() {
        assert_eq!(
            windows_path_text_to_wsl_path(r"C:\Users\Alice\repo").as_deref(),
            Some("/mnt/c/Users/Alice/repo")
        );
        assert_eq!(
            windows_path_text_to_wsl_path(r"\\?\D:\work\repo").as_deref(),
            Some("/mnt/d/work/repo")
        );
    }

    #[test]
    fn converts_wsl_unc_paths_to_posix_paths() {
        assert_eq!(
            windows_path_text_to_wsl_path(r"\\wsl.localhost\Ubuntu\home\alice\repo").as_deref(),
            Some("/home/alice/repo")
        );
        assert_eq!(
            windows_path_text_to_wsl_path(r"\\wsl$\Ubuntu\mnt\c\Users\Alice\repo").as_deref(),
            Some("/mnt/c/Users/Alice/repo")
        );
        assert_eq!(
            windows_path_text_to_wsl_path(r"\\?\UNC\wsl.localhost\Ubuntu\home\alice\repo")
                .as_deref(),
            Some("/home/alice/repo")
        );
    }

    #[test]
    fn leaves_posix_wsl_paths_unchanged() {
        assert_eq!(
            windows_path_text_to_wsl_path("/mnt/c/Users/Alice/repo").as_deref(),
            Some("/mnt/c/Users/Alice/repo")
        );
        assert_eq!(
            windows_path_text_to_wsl_path("/home/alice/repo").as_deref(),
            Some("/home/alice/repo")
        );
    }

    #[test]
    fn rejects_non_wsl_unc_paths() {
        assert_eq!(windows_path_text_to_wsl_path(r"\\server\share\repo"), None);
    }
}
