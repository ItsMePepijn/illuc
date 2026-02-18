use crate::error::{Result, TaskError};
use std::path::{Path, PathBuf};

const ILLUC_GITIGNORE_CONTENTS: &str = "*\n.gitignore\n";

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

pub fn ensure_illuc_dir(base_path: &Path) -> Result<PathBuf> {
    let illuc_dir = base_path.join(".illuc");
    std::fs::create_dir_all(&illuc_dir)?;

    let gitignore_path = illuc_dir.join(".gitignore");
    if !gitignore_path.exists() {
        std::fs::write(&gitignore_path, ILLUC_GITIGNORE_CONTENTS)?;
    }

    Ok(illuc_dir)
}

#[cfg(test)]
mod tests {
    use super::ensure_illuc_dir;
    use crate::error::Result;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn ensure_illuc_dir_creates_gitignore_with_expected_rules() -> Result<()> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let base_dir = std::env::temp_dir().join(format!("illuc-fs-test-{nanos}"));
        std::fs::create_dir_all(&base_dir)?;

        let illuc_dir = ensure_illuc_dir(&base_dir)?;
        let gitignore_path = illuc_dir.join(".gitignore");

        assert!(illuc_dir.exists());
        assert!(gitignore_path.exists());
        let contents = std::fs::read_to_string(gitignore_path)?;
        assert_eq!(contents, "*\n.gitignore\n");

        std::fs::remove_dir_all(&base_dir)?;
        Ok(())
    }
}
