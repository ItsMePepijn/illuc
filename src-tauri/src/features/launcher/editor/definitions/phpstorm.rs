use super::super::{EditorDefinition, EditorIcon};

pub(super) const DEFINITION: EditorDefinition = EditorDefinition {
    id: "phpstorm",
    name: "PhpStorm",
    icon: EditorIcon::Jetbrains,
    command_candidates: &["phpstorm", "phpstorm.sh", "phpstorm64.exe"],
    linux_names: &["phpstorm"],
    linux_execs: &["phpstorm", "phpstorm.sh", "phpstorm64.exe"],
    linux_desktop_ids: &["jetbrains-phpstorm", "phpstorm"],
    windows_paths: &[],
};
