pub mod map;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::utf8_filepath::UTF8FilePath;
pub use map::Map;

#[derive(Debug, Clone, Serialize)]
pub struct FrontmatterFile {
    name: String,
    frontmatter: Option<serde_yaml::Mapping>,
    body: String,
    modified: DateTime<Utc>,
    created: DateTime<Utc>,
}

#[derive(Serialize, PartialEq, Eq)]
pub struct Short {
    name: String,
    frontmatter: Option<serde_yaml::Mapping>,
    modified: DateTime<Utc>,
    created: DateTime<Utc>,
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
            body: _,
            modified,
            created,
        }: FrontmatterFile,
    ) -> Self {
        Self {
            name,
            frontmatter,
            modified,
            created,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReadFromPathError {
    #[error("Failed to parse frontmatter: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Failed to load: {0}")]
    Io(#[from] std::io::Error),
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

    pub fn read_from_path(path: &UTF8FilePath) -> Result<Self, ReadFromPathError> {
        let name = path.name().to_owned();
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

        let frontmatter = serde_yaml::from_str(frontmatter)?;

        Ok(FrontmatterFile {
            name,
            frontmatter: Some(frontmatter),
            body: body.to_owned(),
            modified,
            created,
        })
    }
}
