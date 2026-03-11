use super::super::{EditorDefinition, EditorIcon};

pub(super) const DEFINITION: EditorDefinition = EditorDefinition {
    id: "notepadpp",
    name: "Notepad++",
    icon: EditorIcon::NotepadPlusPlus,
    command_candidates: &["notepad++", "notepad++.exe"],
    linux_names: &["notepad++"],
    linux_execs: &["notepad++"],
    linux_desktop_ids: &["notepad++", "notepad-plus-plus"],
    windows_paths: &[
        "%PROGRAMFILES%\\Notepad++\\notepad++.exe",
        "%PROGRAMFILES(X86)%\\Notepad++\\notepad++.exe",
    ],
};
