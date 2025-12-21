use crate::tasks::DiffFile;

pub fn merge_diff_files(mut staged: Vec<DiffFile>, mut unstaged: Vec<DiffFile>) -> Vec<DiffFile> {
    staged.append(&mut unstaged);
    let mut combined = Vec::new();
    for file in staged {
        if !combined
            .iter()
            .any(|existing: &DiffFile| existing.path == file.path)
        {
            combined.push(file);
        }
    }
    combined
}
