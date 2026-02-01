use crate::error::{Result, TaskError};
use std::path::Path;

pub fn ensure_directory(path: &Path) -> Result<()> {
    if path.exists() {
        if path.is_dir() {
            Ok(())
        } else {
            Err(TaskError::Message(format!(
                "{} is not a directory",
                path.display()
            )))
        }
    } else {
        Err(TaskError::Message(format!(
            "{} does not exist",
            path.display()
        )))
    }
}

pub fn ensure_file(path: &Path) -> Result<()> {
    if path.exists() {
        if path.is_file() {
            Ok(())
        } else {
            Err(TaskError::Message(format!(
                "{} is not a file",
                path.display()
            )))
        }
    } else {
        Err(TaskError::Message(format!(
            "{} does not exist",
            path.display()
        )))
    }
}
