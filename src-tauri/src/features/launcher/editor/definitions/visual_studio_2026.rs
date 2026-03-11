use super::super::{EditorDefinition, EditorIcon};

pub(super) const DEFINITION: EditorDefinition = EditorDefinition {
    id: "visual-studio-2026",
    name: "Visual Studio 2026",
    icon: EditorIcon::Generic,
    command_candidates: &[],
    linux_names: &[],
    linux_execs: &[],
    linux_desktop_ids: &[],
    windows_paths: &[
        "%PROGRAMFILES%\\Microsoft Visual Studio\\2026\\Community\\Common7\\IDE\\devenv.exe",
        "%PROGRAMFILES%\\Microsoft Visual Studio\\2026\\Professional\\Common7\\IDE\\devenv.exe",
        "%PROGRAMFILES%\\Microsoft Visual Studio\\2026\\Enterprise\\Common7\\IDE\\devenv.exe",
    ],
};
