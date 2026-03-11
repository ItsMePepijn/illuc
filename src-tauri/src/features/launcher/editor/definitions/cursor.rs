use super::super::{EditorDefinition, EditorIcon};

pub(super) const DEFINITION: EditorDefinition = EditorDefinition {
    id: "cursor",
    name: "Cursor",
    icon: EditorIcon::Cursor,
    command_candidates: &["cursor", "cursor.exe", "cursor.cmd"],
    linux_names: &["cursor"],
    linux_execs: &["cursor"],
    linux_desktop_ids: &["cursor"],
    windows_paths: &[
        "%LOCALAPPDATA%\\Programs\\Cursor\\Cursor.exe",
        "%PROGRAMFILES%\\Cursor\\Cursor.exe",
    ],
};
