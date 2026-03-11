use super::super::{EditorDefinition, EditorIcon};

pub(super) const DEFINITION: EditorDefinition = EditorDefinition {
    id: "goland",
    name: "GoLand",
    icon: EditorIcon::Jetbrains,
    command_candidates: &["goland", "goland.sh", "goland64.exe"],
    linux_names: &["goland"],
    linux_execs: &["goland", "goland.sh", "goland64.exe"],
    linux_desktop_ids: &["jetbrains-goland", "goland"],
    windows_paths: &[],
};
