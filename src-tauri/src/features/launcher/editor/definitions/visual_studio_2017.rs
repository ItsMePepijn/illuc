use super::super::{EditorDefinition, EditorIcon};

pub(super) const DEFINITION: EditorDefinition = EditorDefinition {
    id: "visual-studio-2017",
    name: "Visual Studio 2017",
    icon: EditorIcon::Generic,
    command_candidates: &[],
    linux_names: &[],
    linux_execs: &[],
    linux_desktop_ids: &[],
    windows_paths: &[
        "%PROGRAMFILES(X86)%\\Microsoft Visual Studio\\2017\\Community\\Common7\\IDE\\devenv.exe",
        "%PROGRAMFILES(X86)%\\Microsoft Visual Studio\\2017\\Professional\\Common7\\IDE\\devenv.exe",
        "%PROGRAMFILES(X86)%\\Microsoft Visual Studio\\2017\\Enterprise\\Common7\\IDE\\devenv.exe",
    ],
};
