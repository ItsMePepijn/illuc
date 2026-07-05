#[cfg(not(target_os = "windows"))]
use portable_pty::CommandBuilder;
#[cfg(not(target_os = "windows"))]
use std::env;
#[cfg(not(target_os = "windows"))]
use std::ffi::OsString;
#[cfg(not(target_os = "windows"))]
use std::path::{Path, PathBuf};
#[cfg(not(target_os = "windows"))]
use std::process::Command as ProcessCommand;
#[cfg(target_os = "windows")]
use std::process::Command;

#[cfg(target_os = "windows")]
pub(crate) fn command_exists(command: &str) -> bool {
    Command::new("wsl.exe")
        .args([
            "--",
            "bash",
            "-lc",
            &format!("command -v {command} >/dev/null 2>&1"),
        ])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn command_exists(command: &str) -> bool {
    find_command(command).is_some()
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn resolve_command(command: &str) -> OsString {
    find_command(command)
        .map(PathBuf::into_os_string)
        .unwrap_or_else(|| OsString::from(command))
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn apply_command_env(command: &mut CommandBuilder) {
    if let Some(path) = expanded_command_path() {
        command.env("PATH", path);
    }
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn apply_process_env(command: &mut ProcessCommand) {
    if let Some(path) = expanded_command_path() {
        command.env("PATH", path);
    }
}

#[cfg(not(target_os = "windows"))]
fn find_command(command: &str) -> Option<PathBuf> {
    if command.contains('/') {
        let path = PathBuf::from(command);
        return is_executable_file(&path).then_some(path);
    }

    find_command_with_env(command, env::var_os("PATH"), env::var_os("HOME"))
}

#[cfg(not(target_os = "windows"))]
fn expanded_command_path() -> Option<OsString> {
    expanded_path_with_env(env::var_os("PATH"), env::var_os("HOME"))
}

#[cfg(not(target_os = "windows"))]
fn find_command_with_env(
    command: &str,
    path_var: Option<OsString>,
    home_dir: Option<OsString>,
) -> Option<PathBuf> {
    command_search_dirs(path_var, home_dir)
        .into_iter()
        .map(|directory| directory.join(command))
        .find(|path| is_executable_file(path))
}

#[cfg(not(target_os = "windows"))]
fn expanded_path_with_env(
    path_var: Option<OsString>,
    home_dir: Option<OsString>,
) -> Option<OsString> {
    env::join_paths(command_search_dirs(path_var, home_dir)).ok()
}

#[cfg(not(target_os = "windows"))]
fn command_search_dirs(path_var: Option<OsString>, home_dir: Option<OsString>) -> Vec<PathBuf> {
    let mut directories = Vec::new();

    if let Some(path_var) = path_var {
        directories.extend(env::split_paths(&path_var));
    }

    if let Some(home_dir) = home_dir {
        let home_dir = PathBuf::from(home_dir);
        directories.extend([
            home_dir.join(".local/bin"),
            home_dir.join("bin"),
            home_dir.join(".cargo/bin"),
            home_dir.join(".npm-global/bin"),
            home_dir.join(".local/share/npm/bin"),
            home_dir.join(".local/share/pnpm"),
            home_dir.join(".bun/bin"),
            home_dir.join(".deno/bin"),
            home_dir.join(".asdf/shims"),
            home_dir.join(".mise/shims"),
            home_dir.join(".claude/local"),
        ]);
    }

    directories.extend([
        PathBuf::from("/opt/npm-global/bin"),
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
    ]);

    let mut unique = Vec::new();
    for directory in directories {
        if !unique.iter().any(|existing| existing == &directory) {
            unique.push(directory);
        }
    }
    unique
}

#[cfg(all(not(target_os = "windows"), unix))]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    path.metadata()
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(all(not(target_os = "windows"), not(unix)))]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

#[cfg(all(test, not(target_os = "windows")))]
mod tests {
    use super::*;
    use std::fs;

    #[cfg(unix)]
    fn make_executable(path: &Path) {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[cfg(not(unix))]
    fn make_executable(_path: &Path) {}

    #[test]
    fn finds_command_in_home_local_bin_when_path_is_missing_it() {
        let temp = tempfile::tempdir().unwrap();
        let bin_dir = temp.path().join(".local/bin");
        fs::create_dir_all(&bin_dir).unwrap();
        let command_path = bin_dir.join("claude");
        fs::write(&command_path, "#!/bin/sh\n").unwrap();
        make_executable(&command_path);

        assert_eq!(
            find_command_with_env("claude", None, Some(temp.path().as_os_str().to_owned())),
            Some(command_path)
        );
    }

    #[test]
    fn path_directories_take_precedence_over_fallback_directories() {
        let temp = tempfile::tempdir().unwrap();
        let path_dir = temp.path().join("path-bin");
        let fallback_dir = temp.path().join(".local/bin");
        fs::create_dir_all(&path_dir).unwrap();
        fs::create_dir_all(&fallback_dir).unwrap();
        let path_command = path_dir.join("claude");
        let fallback_command = fallback_dir.join("claude");
        fs::write(&path_command, "#!/bin/sh\n").unwrap();
        fs::write(&fallback_command, "#!/bin/sh\n").unwrap();
        make_executable(&path_command);
        make_executable(&fallback_command);

        assert_eq!(
            find_command_with_env(
                "claude",
                Some(path_dir.as_os_str().to_owned()),
                Some(temp.path().as_os_str().to_owned())
            ),
            Some(path_command)
        );
    }

    #[test]
    fn expanded_path_includes_home_fallbacks() {
        let temp = tempfile::tempdir().unwrap();
        let path_dir = temp.path().join("path-bin");
        let expanded = expanded_path_with_env(
            Some(path_dir.as_os_str().to_owned()),
            Some(temp.path().as_os_str().to_owned()),
        )
        .unwrap();
        let directories = env::split_paths(&expanded).collect::<Vec<_>>();

        assert!(directories.contains(&path_dir));
        assert!(directories.contains(&temp.path().join(".local/bin")));
        assert!(directories.contains(&temp.path().join(".cargo/bin")));
    }
}
