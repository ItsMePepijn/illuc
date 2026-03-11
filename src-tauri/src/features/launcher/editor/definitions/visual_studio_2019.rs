use super::super::{EditorDefinition, EditorIcon};

pub(super) const DEFINITION: EditorDefinition = EditorDefinition {
    id: "visual-studio-2019",
    name: "Visual Studio 2019",
    icon: EditorIcon::Generic,
    command_candidates: &[],
    linux_names: &[],
    linux_execs: &[],
    linux_desktop_ids: &[],
    windows_paths: &[
        "%PROGRAMFILES(X86)%\\Microsoft Visual Studio\\2019\\Community\\Common7\\IDE\\devenv.exe",
        "%PROGRAMFILES(X86)%\\Microsoft Visual Studio\\2019\\Professional\\Common7\\IDE\\devenv.exe",
        "%PROGRAMFILES(X86)%\\Microsoft Visual Studio\\2019\\Enterprise\\Common7\\IDE\\devenv.exe",
    ],
};
