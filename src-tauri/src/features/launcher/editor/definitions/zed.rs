use super::super::{EditorDefinition, EditorIcon};

pub(super) const DEFINITION: EditorDefinition = EditorDefinition {
    id: "zed",
    name: "Zed",
    icon: EditorIcon::Zed,
    command_candidates: &["zed", "zed.exe"],
    linux_names: &["zed"],
    linux_execs: &["zed"],
    linux_desktop_ids: &["zed", "dev.zed.zed"],
    windows_paths: &[
        "%LOCALAPPDATA%\\Programs\\Zed\\Zed.exe",
        "%PROGRAMFILES%\\Zed\\Zed.exe",
    ],
};
