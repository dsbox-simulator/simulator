use std::fs::read_dir;
use std::io::Result;
use std::path::{Path, PathBuf};

pub fn collect_files(root: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    let root = root.as_ref();
    collect_files_rec(root, root, &mut paths)?;
    Ok(paths)
}

fn collect_files_rec(root: &Path, current: &Path, results: &mut Vec<PathBuf>) -> Result<()> {
    for entry in read_dir(current)? {
        let path = entry?.path();
        if path.is_file() {
            results.push(path);
        } else if path.is_dir() {
            collect_files_rec(root, &path, results)?;
        }
    }
    Ok(())
}