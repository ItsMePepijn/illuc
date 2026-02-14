use crate::error::Result;
use crate::utils::fs::ensure_directory;
use std::path::Path;

pub mod commands;
mod explorer;
mod terminal;
mod vscode;

pub fn open_path_in_vscode(path: &Path) -> Result<()> {
    ensure_directory(path)?;
    vscode::spawn(path)
}

pub fn open_file_in_vscode(path: &Path, line: Option<u32>, column: Option<u32>) -> Result<()> {
    vscode::spawn_file(path, line, column)
}

pub fn open_path_terminal(path: &Path) -> Result<()> {
    ensure_directory(path)?;
    terminal::spawn(path)
}

pub fn open_path_in_explorer(path: &Path) -> Result<()> {
    ensure_directory(path)?;
    explorer::spawn(path)
}
