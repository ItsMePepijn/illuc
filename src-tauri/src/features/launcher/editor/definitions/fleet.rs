use super::super::{EditorDefinition, EditorIcon};

pub(super) const DEFINITION: EditorDefinition = EditorDefinition {
    id: "fleet",
    name: "Fleet",
    icon: EditorIcon::Jetbrains,
    command_candidates: &["fleet", "Fleet", "fleet.exe"],
    linux_names: &["fleet"],
    linux_execs: &["fleet", "Fleet"],
    linux_desktop_ids: &["jetbrains-fleet", "fleet"],
    windows_paths: &[
        "%LOCALAPPDATA%\\Programs\\Fleet\\Fleet.exe",
        "%PROGRAMFILES%\\Fleet\\Fleet.exe",
    ],
};
