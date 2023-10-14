use std::path::{Path, PathBuf};

pub fn path_has_extensions(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|ext| extensions.contains(&ext))
}

pub fn filepaths_with_extensions(
    dir: &Path,
    extensions: &[&str],
) -> Result<Vec<PathBuf>, std::io::Error> {
    std::fs::read_dir(dir)?
        .filter_map(|entry| {
            entry
                .map(|entry| {
                    let path = entry.path();
                    if !path.is_file() {
                        return None;
                    }
                    if path_has_extensions(&path, extensions) {
                        Some(path)
                    } else {
                        None
                    }
                })
                .transpose()
        })
        .collect()
}
