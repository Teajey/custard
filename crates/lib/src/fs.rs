use camino::{Utf8Path as Path, Utf8PathBuf as PathBuf};

pub fn path_has_extensions(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .is_some_and(|ext| extensions.contains(&ext))
}

pub fn filepaths_with_extensions(
    dir: &Path,
    extensions: &[&str],
) -> Result<Vec<PathBuf>, std::io::Error> {
    dir.read_dir_utf8()?
        .filter_map(|entry| {
            entry
                .map(|entry| {
                    let path = entry.path().to_path_buf();
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
