use super::super::{EditorDefinition, EditorIcon};

pub(super) const DEFINITION: EditorDefinition = EditorDefinition {
    id: "geany",
    name: "Geany",
    icon: EditorIcon::Generic,
    command_candidates: &["geany", "geany.exe"],
    linux_names: &["geany"],
    linux_execs: &["geany"],
    linux_desktop_ids: &["geany"],
    windows_paths: &["%PROGRAMFILES%\\Geany\\bin\\geany.exe"],
};
