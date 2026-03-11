use super::super::{EditorDefinition, EditorIcon};

pub(super) const DEFINITION: EditorDefinition = EditorDefinition {
    id: "pycharm",
    name: "PyCharm",
    icon: EditorIcon::Jetbrains,
    command_candidates: &["pycharm", "pycharm.sh", "pycharm64.exe"],
    linux_names: &["pycharm"],
    linux_execs: &["pycharm", "pycharm.sh", "pycharm64.exe"],
    linux_desktop_ids: &["jetbrains-pycharm", "pycharm"],
    windows_paths: &[],
};
