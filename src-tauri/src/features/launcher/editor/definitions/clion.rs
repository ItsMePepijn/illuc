use super::super::{EditorDefinition, EditorIcon};

pub(super) const DEFINITION: EditorDefinition = EditorDefinition {
    id: "clion",
    name: "CLion",
    icon: EditorIcon::Jetbrains,
    command_candidates: &["clion", "clion.sh", "clion64.exe"],
    linux_names: &["clion"],
    linux_execs: &["clion", "clion.sh", "clion64.exe"],
    linux_desktop_ids: &["jetbrains-clion", "clion"],
    windows_paths: &[],
};
