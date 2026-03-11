use super::super::{EditorDefinition, EditorIcon};

pub(super) const DEFINITION: EditorDefinition = EditorDefinition {
    id: "rubymine",
    name: "RubyMine",
    icon: EditorIcon::Jetbrains,
    command_candidates: &["rubymine", "rubymine.sh", "rubymine64.exe"],
    linux_names: &["rubymine"],
    linux_execs: &["rubymine", "rubymine.sh", "rubymine64.exe"],
    linux_desktop_ids: &["jetbrains-rubymine", "rubymine"],
    windows_paths: &[],
};
