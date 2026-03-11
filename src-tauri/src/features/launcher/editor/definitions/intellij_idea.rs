use super::super::{EditorDefinition, EditorIcon};

pub(super) const DEFINITION: EditorDefinition = EditorDefinition {
    id: "intellij-idea",
    name: "IntelliJ IDEA",
    icon: EditorIcon::Jetbrains,
    command_candidates: &["idea", "idea.sh", "idea64.exe"],
    linux_names: &["intellij idea", "intellij"],
    linux_execs: &["idea", "idea.sh", "idea64.exe"],
    linux_desktop_ids: &["jetbrains-idea", "idea", "intellij-idea"],
    windows_paths: &[],
};
