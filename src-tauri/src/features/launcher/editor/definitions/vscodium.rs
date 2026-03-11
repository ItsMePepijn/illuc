use super::super::{EditorDefinition, EditorIcon};

pub(super) const DEFINITION: EditorDefinition = EditorDefinition {
    id: "vscodium",
    name: "VSCodium",
    icon: EditorIcon::Vscodium,
    command_candidates: &["codium", "codium.exe", "codium.cmd"],
    linux_names: &["vscodium"],
    linux_execs: &["codium"],
    linux_desktop_ids: &["codium", "vscodium", "com.vscodium.codium"],
    windows_paths: &[
        "%LOCALAPPDATA%\\Programs\\VSCodium\\VSCodium.exe",
        "%PROGRAMFILES%\\VSCodium\\VSCodium.exe",
    ],
};
