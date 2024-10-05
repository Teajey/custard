pub mod keeper;

use anyhow::Result;
use camino::{Utf8Path as Path, Utf8PathBuf};
use chrono::{DateTime, Utc};
use serde::Serialize;

pub use keeper::Keeper;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FrontmatterFile {
    pub name: String,
    pub frontmatter: Option<serde_yaml::Mapping>,
    pub body: String,
    pub modified: DateTime<Utc>,
    pub created: DateTime<Utc>,
}

impl PartialOrd for FrontmatterFile {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.created.cmp(&other.created))
    }
}

impl Ord for FrontmatterFile {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.created.cmp(&other.created)
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Short {
    pub name: String,
    pub frontmatter: Option<serde_yaml::Mapping>,
    one_liner: Option<String>,
    pub modified: DateTime<Utc>,
    pub created: DateTime<Utc>,
}

impl PartialOrd for Short {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.created.cmp(&other.created))
    }
}

impl Ord for Short {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.created.cmp(&other.created)
    }
}

impl From<FrontmatterFile> for Short {
    fn from(
        FrontmatterFile {
            name,
            frontmatter,
            body,
            modified,
            created,
        }: FrontmatterFile,
    ) -> Self {
        let lines = body.lines().collect::<Vec<_>>();
        let one_liner = if lines.len() == 1 {
            Some(lines[0].to_owned())
        } else {
            None
        };
        Self {
            name,
            frontmatter,
            one_liner,
            modified,
            created,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReadFromPathError {
    #[error("Failed to parse frontmatter for '{0}': {1}")]
    Yaml(String, serde_yaml::Error),
    #[error("Failed to load: {0}")]
    Io(#[from] std::io::Error),
    #[error("Tried to read from path with no file name: {0}")]
    NoFileNamePath(Utf8PathBuf),
}

impl FrontmatterFile {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn frontmatter(&self) -> Option<&serde_yaml::Mapping> {
        self.frontmatter.as_ref()
    }

    pub fn body(&self) -> &str {
        &self.body
    }

    pub fn created(&self) -> &DateTime<Utc> {
        &self.created
    }

    pub fn modified(&self) -> &DateTime<Utc> {
        &self.modified
    }

    pub fn read_from_path(path: &Path) -> Result<Self, ReadFromPathError> {
        let name = path
            .file_name()
            .ok_or_else(|| ReadFromPathError::NoFileNamePath(path.to_path_buf()))?
            .to_owned();
        let metadata = std::fs::metadata(path)?;
        let modified = metadata.modified()?.into();
        let created = metadata.created()?.into();
        let string = std::fs::read_to_string(path)?;

        if !string.starts_with("---\n") {
            let md = FrontmatterFile {
                name,
                frontmatter: None,
                body: string,
                modified,
                created,
            };
            return Ok(md);
        }

        let [_, frontmatter, body] = string.splitn(3, "---\n").collect::<Vec<_>>()[..] else {
            let md = FrontmatterFile {
                name,
                frontmatter: None,
                body: string,
                modified,
                created,
            };
            return Ok(md);
        };

        let frontmatter = serde_yaml::from_str(frontmatter)
            .map_err(|err| ReadFromPathError::Yaml(name.clone(), err))?;

        Ok(FrontmatterFile {
            name,
            frontmatter: Some(frontmatter),
            body: body.to_owned(),
            modified,
            created,
        })
    }
}
