use super::super::{EditorDefinition, EditorIcon};

pub(super) const DEFINITION: EditorDefinition = EditorDefinition {
    id: "rider",
    name: "Rider",
    icon: EditorIcon::Jetbrains,
    command_candidates: &["rider", "rider.sh", "rider64.exe"],
    linux_names: &["rider"],
    linux_execs: &["rider", "rider.sh", "rider64.exe"],
    linux_desktop_ids: &["jetbrains-rider", "rider"],
    windows_paths: &[],
};
