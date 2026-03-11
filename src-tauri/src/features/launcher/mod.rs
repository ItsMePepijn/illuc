use crate::error::Result;
use crate::utils::fs::ensure_directory;
use std::path::Path;

pub mod commands;
mod editor;
mod explorer;
mod linux;
mod terminal;
mod windows;

pub use editor::EditorApp;

pub fn list_installed_editors() -> Vec<EditorApp> {
    editor::list_installed()
}

pub fn open_path_in_editor(path: &Path, editor_id: &str) -> Result<()> {
    ensure_directory(path)?;
    editor::open_path(path, editor_id)
}

pub fn open_file_in_editor(
    path: &Path,
    editor_id: &str,
    line: Option<u32>,
    column: Option<u32>,
) -> Result<()> {
    editor::open_file(path, editor_id, line, column)
}

pub fn open_path_in_default_editor(path: &Path) -> Result<()> {
    ensure_directory(path)?;
    editor::open_path_in_default(path)
}

pub fn open_file_in_default_editor(
    path: &Path,
    line: Option<u32>,
    column: Option<u32>,
) -> Result<()> {
    editor::open_file_in_default(path, line, column)
}

pub fn open_path_terminal(path: &Path) -> Result<()> {
    ensure_directory(path)?;
    terminal::spawn(path)
}

pub fn open_path_in_explorer(path: &Path) -> Result<()> {
    ensure_directory(path)?;
    explorer::spawn(path)
}
