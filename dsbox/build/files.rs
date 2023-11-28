use std::fs::read_dir;
use std::io::Result;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};

pub fn collect_files(root: impl AsRef<Path>, excludes: &[&str]) -> Result<Vec<PathBuf>> {
    let mut globset = GlobSetBuilder::new();
    for exclude in excludes {
        globset.add(Glob::new(exclude).unwrap());
    }
    let globset = globset.build().unwrap();
    let mut paths = Vec::new();
    let root = root.as_ref();
    collect_files_rec(root, root, &globset, &mut paths)?;
    Ok(paths)
}

fn collect_files_rec(root: &Path, current: &Path, excludes: &GlobSet, results: &mut Vec<PathBuf>) -> Result<()> {
    'entries: for entry in read_dir(current)? {
        let path = entry?.path();
        if excludes.is_match(&path.strip_prefix(root).unwrap()) { continue 'entries; }
        if path.is_file() {
            results.push(path);
        } else if path.is_dir() {
            collect_files_rec(root, &path, excludes, results)?;
        }
    }
    Ok(())
}