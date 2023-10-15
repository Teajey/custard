use std::fmt::Debug;
use std::path::{Path, PathBuf};

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct UTF8FilePath {
    path_buf: PathBuf,
    name: String,
    extension: Option<String>,
}

impl Debug for UTF8FilePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.path_buf.fmt(f)
    }
}

impl AsRef<Path> for UTF8FilePath {
    fn as_ref(&self) -> &Path {
        self.path_buf.as_ref()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Path has no file name (it ends with `..`)")]
    NoFileName,
    #[error("Path is non-UTF-8")]
    NonUTF8,
}

impl TryFrom<PathBuf> for UTF8FilePath {
    type Error = Error;

    fn try_from(path_buf: PathBuf) -> Result<Self, Self::Error> {
        let name = path_buf
            .file_name()
            .ok_or(Error::NoFileName)?
            .to_str()
            .ok_or(Error::NonUTF8)?
            .to_owned();
        let extension = path_buf
            .extension()
            .map(|ext| ext.to_str().expect("utf-8 was already checked").to_owned());
        Ok(Self {
            path_buf,
            name,
            extension,
        })
    }
}

impl UTF8FilePath {
    pub fn name(&self) -> &str {
        &self.name
    }

    // pub fn extension(&self) -> Option<&str> {
    //     self.extension.as_deref()
    // }
}
