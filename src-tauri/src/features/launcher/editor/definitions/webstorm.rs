use super::super::{EditorDefinition, EditorIcon};

pub(super) const DEFINITION: EditorDefinition = EditorDefinition {
    id: "webstorm",
    name: "WebStorm",
    icon: EditorIcon::Jetbrains,
    command_candidates: &["webstorm", "webstorm.sh", "webstorm64.exe"],
    linux_names: &["webstorm"],
    linux_execs: &["webstorm", "webstorm.sh", "webstorm64.exe"],
    linux_desktop_ids: &["jetbrains-webstorm", "webstorm"],
    windows_paths: &[],
};
