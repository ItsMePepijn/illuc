use super::super::{EditorDefinition, EditorIcon};

pub(super) const DEFINITION: EditorDefinition = EditorDefinition {
    id: "windsurf",
    name: "Windsurf",
    icon: EditorIcon::Windsurf,
    command_candidates: &["windsurf", "windsurf.exe", "windsurf.cmd"],
    linux_names: &["windsurf"],
    linux_execs: &["windsurf"],
    linux_desktop_ids: &["windsurf"],
    windows_paths: &[
        "%LOCALAPPDATA%\\Programs\\Windsurf\\Windsurf.exe",
        "%PROGRAMFILES%\\Windsurf\\Windsurf.exe",
    ],
};
