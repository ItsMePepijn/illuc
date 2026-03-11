use super::super::{EditorDefinition, EditorIcon};

pub(super) const DEFINITION: EditorDefinition = EditorDefinition {
    id: "kate",
    name: "Kate",
    icon: EditorIcon::Generic,
    command_candidates: &["kate", "kate.exe"],
    linux_names: &["kate"],
    linux_execs: &["kate"],
    linux_desktop_ids: &["org.kde.kate", "kate"],
    windows_paths: &["%PROGRAMFILES%\\Kate\\bin\\kate.exe"],
};
