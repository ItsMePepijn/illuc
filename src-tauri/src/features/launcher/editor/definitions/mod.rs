mod android_studio;
mod clion;
mod cursor;
mod datagrip;
mod fleet;
mod geany;
mod goland;
mod intellij_idea;
mod kate;
mod notepadpp;
mod phpstorm;
mod pycharm;
mod rider;
mod rubymine;
mod rustrover;
mod sublime_text;
mod visual_studio_2017;
mod visual_studio_2019;
mod visual_studio_2022;
mod visual_studio_2026;
mod vscode;
mod vscode_insiders;
mod vscodium;
mod webstorm;
mod windsurf;
mod zed;

use super::EditorDefinition;

pub(super) const EDITOR_DEFINITIONS: &[EditorDefinition] = &[
    vscode::DEFINITION,
    vscodium::DEFINITION,
    visual_studio_2026::DEFINITION,
    visual_studio_2022::DEFINITION,
    visual_studio_2019::DEFINITION,
    visual_studio_2017::DEFINITION,
    rider::DEFINITION,
    vscode_insiders::DEFINITION,
    cursor::DEFINITION,
    windsurf::DEFINITION,
    zed::DEFINITION,
    sublime_text::DEFINITION,
    notepadpp::DEFINITION,
    kate::DEFINITION,
    geany::DEFINITION,
    fleet::DEFINITION,
    android_studio::DEFINITION,
    intellij_idea::DEFINITION,
    webstorm::DEFINITION,
    phpstorm::DEFINITION,
    pycharm::DEFINITION,
    datagrip::DEFINITION,
    goland::DEFINITION,
    clion::DEFINITION,
    rustrover::DEFINITION,
    rubymine::DEFINITION,
];
