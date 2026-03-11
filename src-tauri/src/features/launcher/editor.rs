mod definitions;

use crate::error::{Result, TaskError};
use crate::utils::fs::ensure_file;
use definitions::EDITOR_DEFINITIONS;
use serde::Serialize;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EditorApp {
    pub id: String,
    pub name: String,
    pub icon: EditorIcon,
    pub icon_data_url: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EditorIcon {
    Vscode,
    VscodeInsiders,
    Vscodium,
    Cursor,
    Windsurf,
    Zed,
    SublimeText,
    NotepadPlusPlus,
    Jetbrains,
    Generic,
}

#[derive(Clone, Debug)]
struct InstalledEditor {
    app: EditorApp,
    launch: LaunchCommand,
}

#[derive(Clone, Debug)]
pub(crate) struct LaunchCommand {
    pub(crate) command: PathBuf,
    pub(crate) args: Vec<String>,
}

#[derive(Clone, Copy)]
pub(crate) struct EditorDefinition {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) icon: EditorIcon,
    pub(crate) command_candidates: &'static [&'static str],
    pub(crate) linux_names: &'static [&'static str],
    pub(crate) linux_execs: &'static [&'static str],
    pub(crate) linux_desktop_ids: &'static [&'static str],
    pub(crate) windows_paths: &'static [&'static str],
}

pub fn list_installed() -> Vec<EditorApp> {
    detect_installed_editors()
        .into_iter()
        .map(|editor| editor.app)
        .collect()
}

pub fn open_path_in_default(path: &Path) -> Result<()> {
    let editor = detect_installed_editors()
        .into_iter()
        .next()
        .ok_or_else(|| {
            TaskError::Message(
                "Unable to launch editor. No supported editors were detected.".to_string(),
            )
        })?;

    spawn_launch_command(&editor.launch, path)
}

pub fn open_path(path: &Path, editor_id: &str) -> Result<()> {
    let installed = detect_installed_editors();
    let editor = installed
        .into_iter()
        .find(|editor| editor.app.id == editor_id)
        .ok_or_else(|| {
            TaskError::Message(format!(
                "Unable to launch editor. `{editor_id}` is not currently installed."
            ))
        })?;

    spawn_launch_command(&editor.launch, path)
}

pub fn open_file(
    path: &Path,
    editor_id: &str,
    line: Option<u32>,
    column: Option<u32>,
) -> Result<()> {
    ensure_file(path)?;

    let installed = detect_installed_editors();
    let editor = installed
        .into_iter()
        .find(|editor| editor.app.id == editor_id)
        .ok_or_else(|| {
            TaskError::Message(format!(
                "Unable to launch editor. `{editor_id}` is not currently installed."
            ))
        })?;

    let args = build_file_open_args(editor_id, path, line, column)?;
    spawn_launch_command_with_args(&editor.launch, &args)
}

pub fn open_file_in_default(path: &Path, line: Option<u32>, column: Option<u32>) -> Result<()> {
    ensure_file(path)?;

    let editor = detect_installed_editors()
        .into_iter()
        .next()
        .ok_or_else(|| {
            TaskError::Message(
                "Unable to launch editor. No supported editors were detected.".to_string(),
            )
        })?;

    let args = build_file_open_args(&editor.app.id, path, line, column)?;
    spawn_launch_command_with_args(&editor.launch, &args)
}

pub(crate) fn resolve_command_path(
    command_candidates: &[&str],
    windows_paths: &[&str],
) -> Option<PathBuf> {
    super::windows::resolve_install_path(windows_paths)
        .or_else(|| find_command_in_path(command_candidates))
}

fn detect_installed_editors() -> Vec<InstalledEditor> {
    let mut installed = Vec::new();

    for definition in EDITOR_DEFINITIONS {
        if let Some(metadata) = super::linux::resolve_editor_launch(definition) {
            let icon_data_url = metadata.icon_data_url.or_else(|| {
                log::warn!(
                    "launcher editor `{}` (id={}) has no resolved Linux icon; falling back to default icon",
                    definition.name,
                    definition.id
                );
                None
            });
            installed.push(InstalledEditor {
                app: EditorApp {
                    id: definition.id.to_string(),
                    name: definition.name.to_string(),
                    icon: definition.icon,
                    icon_data_url,
                },
                launch: metadata.launch,
            });
            continue;
        }

        let installed_path = super::windows::resolve_install_path(definition.windows_paths);
        let launch_command =
            resolve_command_path(definition.command_candidates, definition.windows_paths);

        if let Some(command) = launch_command {
            let icon_source: Option<PathBuf> =
                installed_path.or_else(|| super::windows::icon_source_from_command(&command));
            let icon_data_url = resolve_icon_data_url(
                definition.name,
                definition.id,
                icon_source.as_deref(),
                &command,
            );
            installed.push(InstalledEditor {
                app: EditorApp {
                    id: definition.id.to_string(),
                    name: definition.name.to_string(),
                    icon: definition.icon,
                    icon_data_url,
                },
                launch: LaunchCommand {
                    command,
                    args: Vec::new(),
                },
            });
        }
    }
    installed
}

fn resolve_icon_data_url(
    editor_name: &str,
    editor_id: &str,
    icon_source: Option<&Path>,
    command: &Path,
) -> Option<String> {
    match icon_source {
        Some(source) => super::windows::load_icon_data_url(source).or_else(|| {
            log::warn!(
                "launcher editor `{}` (id={}) failed to load icon from {}; falling back to default icon",
                editor_name,
                editor_id,
                source.display()
            );
            None
        }),
        None => {
            log::warn!(
                "launcher editor `{}` (id={}) has no icon source for command {}; falling back to default icon",
                editor_name,
                editor_id,
                command.display()
            );
            None
        }
    }
}

fn spawn_launch_command(launch: &LaunchCommand, path: &Path) -> Result<()> {
    spawn_launch_command_with_args(launch, &[path.as_os_str().to_owned()])
}

fn spawn_launch_command_with_args(launch: &LaunchCommand, extra_args: &[OsString]) -> Result<()> {
    let mut command = Command::new(&launch.command);
    super::windows::prepare_command(&mut command);
    command.args(&launch.args).args(extra_args).spawn()?;
    Ok(())
}

fn build_file_open_args(
    editor_id: &str,
    path: &Path,
    line: Option<u32>,
    column: Option<u32>,
) -> Result<Vec<OsString>> {
    let line = line.unwrap_or(1);
    let column = column.unwrap_or(1);
    let target = format!("{}:{}:{}", path.display(), line, column);

    match editor_id {
        "vscode" | "vscode-insiders" | "vscodium" | "cursor" | "windsurf" => {
            Ok(vec!["-r".into(), "--goto".into(), target.into()])
        }
        "sublime-text" | "fleet" => Ok(vec![target.into()]),
        "notepadpp" => Ok(vec![
            format!("-n{}", line).into(),
            format!("-c{}", column).into(),
            path.as_os_str().to_owned(),
        ]),
        "kate" => Ok(vec![
            path.as_os_str().to_owned(),
            "-l".into(),
            line.to_string().into(),
            "-c".into(),
            column.to_string().into(),
        ]),
        "geany" => Ok(vec![
            "--line".into(),
            line.to_string().into(),
            "--column".into(),
            column.to_string().into(),
            path.as_os_str().to_owned(),
        ]),
        "android-studio" | "intellij-idea" | "webstorm" | "phpstorm" | "pycharm" | "datagrip"
        | "goland" | "clion" | "rustrover" | "rubymine" | "rider" => Ok(vec![
            "--line".into(),
            line.to_string().into(),
            path.as_os_str().to_owned(),
        ]),
        "zed" => Ok(vec![path.as_os_str().to_owned()]),
        _ => Err(TaskError::Message(format!(
            "Opening files in `{editor_id}` is not supported."
        ))),
    }
}

fn find_command_in_path(command_candidates: &[&str]) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;

    for directory in env::split_paths(&path_var) {
        for candidate in command_candidates {
            let candidate_path = directory.join(candidate);
            if candidate_path.is_file() {
                return Some(candidate_path);
            }
        }
    }

    None
}
