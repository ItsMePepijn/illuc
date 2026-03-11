use super::super::{EditorDefinition, EditorIcon};

pub(super) const DEFINITION: EditorDefinition = EditorDefinition {
    id: "datagrip",
    name: "DataGrip",
    icon: EditorIcon::Jetbrains,
    command_candidates: &["datagrip", "datagrip.sh", "datagrip64.exe"],
    linux_names: &["datagrip"],
    linux_execs: &["datagrip", "datagrip.sh", "datagrip64.exe"],
    linux_desktop_ids: &["jetbrains-datagrip", "datagrip"],
    windows_paths: &[],
};
