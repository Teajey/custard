use camino::{Utf8Path as Path, Utf8PathBuf as PathBuf};

pub fn path_has_extensions(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .is_some_and(|ext| extensions.contains(&ext))
}

pub fn filepaths_with_extensions(
    dir: &Path,
    extensions: &[&str],
) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut results = Vec::new();
    let mut dirs = vec![dir.to_path_buf()];

    while let Some(current_dir) = dirs.pop() {
        for entry in current_dir.read_dir_utf8()? {
            let path = entry?.into_path();
            if path.is_dir() {
                let is_hidden = path.file_name().is_some_and(|name| name.starts_with('.'));
                if !is_hidden {
                    dirs.push(path);
                }
            } else if path.is_file() && path_has_extensions(&path, extensions) {
                results.push(path);
            }
        }
    }

    Ok(results)
}
