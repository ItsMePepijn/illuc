use super::super::{EditorDefinition, EditorIcon};

pub(super) const DEFINITION: EditorDefinition = EditorDefinition {
    id: "android-studio",
    name: "Android Studio",
    icon: EditorIcon::Jetbrains,
    command_candidates: &["studio", "studio.sh", "studio64.exe"],
    linux_names: &["android studio"],
    linux_execs: &["studio", "studio.sh", "studio64.exe"],
    linux_desktop_ids: &["android-studio", "jetbrains-studio", "studio"],
    windows_paths: &[
        "%PROGRAMFILES%\\Android\\Android Studio\\bin\\studio64.exe",
        "%PROGRAMFILES%\\Android\\Android Studio\\bin\\studio.exe",
    ],
};
