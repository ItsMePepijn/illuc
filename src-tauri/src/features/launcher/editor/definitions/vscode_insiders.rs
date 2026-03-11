use super::super::{EditorDefinition, EditorIcon};

pub(super) const DEFINITION: EditorDefinition = EditorDefinition {
    id: "vscode-insiders",
    name: "VS Code Insiders",
    icon: EditorIcon::VscodeInsiders,
    command_candidates: &["code-insiders", "code-insiders.exe", "code-insiders.cmd"],
    linux_names: &[
        "visual studio code - insiders",
        "visual studio code insiders",
    ],
    linux_execs: &["code-insiders"],
    linux_desktop_ids: &[
        "code-insiders",
        "visual-studio-code-insiders",
        "com.visualstudio.code.insiders",
    ],
    windows_paths: &[
        "%LOCALAPPDATA%\\Programs\\Microsoft VS Code Insiders\\Code - Insiders.exe",
        "%PROGRAMFILES%\\Microsoft VS Code Insiders\\Code - Insiders.exe",
    ],
};
