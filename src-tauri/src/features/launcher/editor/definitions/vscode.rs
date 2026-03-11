use super::super::{EditorDefinition, EditorIcon};

pub(super) const DEFINITION: EditorDefinition = EditorDefinition {
    id: "vscode",
    name: "VS Code",
    icon: EditorIcon::Vscode,
    command_candidates: &["code", "code.exe", "code.cmd"],
    linux_names: &["visual studio code"],
    linux_execs: &["code"],
    linux_desktop_ids: &["code", "visual-studio-code", "com.visualstudio.code"],
    windows_paths: &[
        "%LOCALAPPDATA%\\Programs\\Microsoft VS Code\\Code.exe",
        "%PROGRAMFILES%\\Microsoft VS Code\\Code.exe",
        "%PROGRAMFILES(X86)%\\Microsoft VS Code\\Code.exe",
    ],
};
