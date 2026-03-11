use super::super::{EditorDefinition, EditorIcon};

pub(super) const DEFINITION: EditorDefinition = EditorDefinition {
    id: "rustrover",
    name: "RustRover",
    icon: EditorIcon::Jetbrains,
    command_candidates: &["rustrover", "rustrover.sh", "rustrover64.exe"],
    linux_names: &["rustrover"],
    linux_execs: &["rustrover", "rustrover.sh", "rustrover64.exe"],
    linux_desktop_ids: &["jetbrains-rustrover", "rustrover"],
    windows_paths: &[],
};
