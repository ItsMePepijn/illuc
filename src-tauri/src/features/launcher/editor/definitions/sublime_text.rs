use super::super::{EditorDefinition, EditorIcon};

pub(super) const DEFINITION: EditorDefinition = EditorDefinition {
    id: "sublime-text",
    name: "Sublime Text",
    icon: EditorIcon::SublimeText,
    command_candidates: &["subl", "subl.exe"],
    linux_names: &["sublime text"],
    linux_execs: &["subl", "sublime_text"],
    linux_desktop_ids: &["sublime_text", "sublime-text", "sublime"],
    windows_paths: &[
        "%PROGRAMFILES%\\Sublime Text\\sublime_text.exe",
        "%PROGRAMFILES(X86)%\\Sublime Text\\sublime_text.exe",
    ],
};
